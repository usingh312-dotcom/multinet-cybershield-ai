// Package ipc defines the versioned JSON contract spoken over the Windows
// named pipe between MultiNet (Rust client) and the Net Merge service (Go).
//
// Contract version 1. Message framing: one JSON object per line, newline
// delimited. The server validates every request and never exposes arbitrary
// command execution.
package ipc

import (
	"encoding/json"
	"fmt"
	"sync"
	"time"
)

// PIPEVersion is the current protocol version. The client must match it.
const PIPEVersion = 1

// DefaultPipeName is the Windows named-pipe endpoint.
const DefaultPipeName = `\\.\pipe\multinet-netmerge-v1`

// Method is an IPC method name.
type Method string

const (
	MethodHealth      Method = "health"
	MethodStatus      Method = "status"
	MethodAdapters    Method = "adapters"
	MethodStart       Method = "start"
	MethodStop        Method = "stop"
	MethodSetLimit    Method = "set_limit"
	MethodModeBStatus Method = "mode_b_status"
	MethodSetModeB    Method = "set_mode_b"
)

// Request is an inbound IPC message from MultiNet.
type Request struct {
	ID           string          `json:"id"`
	PipeVersion  uint16          `json:"pipe_version"`
	Method       Method          `json:"method"`
	Params       json.RawMessage `json:"params,omitempty"`
}

// Response is an outbound IPC message to MultiNet.
type Response struct {
	ID     string          `json:"id"`
	OK     bool            `json:"ok"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *ServiceError   `json:"error,omitempty"`
}

// ServiceError is a structured error returned in a failed response.
type ServiceError struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	Retryable bool   `json:"retryable"`
}

// StartParams are the parameters for the start method.
type StartParams struct {
	Adapters   []string             `json:"adapters"`
	Weights    map[string]float64   `json:"weights,omitempty"`
	AutoWeight bool                 `json:"auto_weight"`
	IPv6       string               `json:"ipv6"`
	MTU        uint16               `json:"mtu"`
}

// AdapterStatus is a per-adapter snapshot returned by status/adapters.
type AdapterStatus struct {
	Name            string  `json:"name"`
	Index           uint32  `json:"index"`
	IPv4            string  `json:"ipv4,omitempty"`
	IPv6            string  `json:"ipv6,omitempty"`
	State           string  `json:"state"` // up/down/unknown
	Weight          float64 `json:"weight"`
	ActiveFlows     uint64  `json:"active_flows"`
	TotalFlows      uint64  `json:"total_flows"`
	FailedFlows     uint64  `json:"failed_flows"`
	BytesIn         uint64  `json:"bytes_in"`
	BytesOut        uint64  `json:"bytes_out"`
	RateInBps       float64 `json:"rate_in_bps"`
	RateOutBps      float64 `json:"rate_out_bps"`
	LinkRateInBps   float64 `json:"link_rate_in_bps"`
	LinkRateOutBps  float64 `json:"link_rate_out_bps"`
	LimitMbps       float64 `json:"limit_mbps"`
}

// HealthResult is the result of the health method.
type HealthResult struct {
	Service       string `json:"service"`
	Version       string `json:"version"`
	PipeVersion   uint16 `json:"pipe_version"`
	ModeAActive   bool   `json:"mode_a_active"`
	ModeBActive   bool   `json:"mode_b_active"`
	On            bool   `json:"on"`
}

// SetModeBParams are the parameters for the set_mode_b method.
// When rust_pid is omitted or 0, the server binds to the authenticated named-pipe
// client's PID. When specified, the server validates that the PID belongs to an
// authorized Rust bonding engine process.
type SetModeBParams struct {
	Active  bool   `json:"active"`
	RustPID uint32 `json:"rust_pid,omitempty"`
}

// Security limits for the IPC pipe.
const (
	// MaxMessageSize is the largest single IPC message the server will accept.
	// Prevents memory exhaustion from oversized frames.
	MaxMessageSize = 1 << 20 // 1 MiB
	// MaxConcurrentConns limits the number of simultaneous pipe connections.
	MaxConcurrentConns = 64
	// RequestTimeout is the maximum time allowed to read a full request line.
	RequestTimeout = 10 * time.Second
)

// MarshalRequest encodes a Request as a newline-terminated JSON line.
func MarshalRequest(r Request) ([]byte, error) {
	b, err := json.Marshal(r)
	if err != nil {
		return nil, err
	}
	return append(b, '\n'), nil
}

// UnmarshalResponse decodes a newline-terminated JSON line into a Response.
func UnmarshalResponse(line []byte) (Response, error) {
	var r Response
	if err := json.Unmarshal(line, &r); err != nil {
		return Response{}, fmt.Errorf("invalid response: %w", err)
	}
	return r, nil
}

// NewOKResponse builds a successful response.
func NewOKResponse(id string, result any) (Response, error) {
	b, err := json.Marshal(result)
	if err != nil {
		return Response{}, err
	}
	return Response{ID: id, OK: true, Result: b}, nil
}

// NewErrorResponse builds a failed response.
func NewErrorResponse(id string, code, message string, retryable bool) Response {
	return Response{ID: id, OK: false, Error: &ServiceError{Code: code, Message: message, Retryable: retryable}}
}

// Handler is implemented by the service to handle a request.
// ci is the validated identity of the pipe client, obtained via
// GetNamedPipeClientProcessId. It should be trusted over any caller-supplied
// values in the request.
type Handler func(req Request, ci ClientIdentity) (Response, error)

// Router dispatches methods to handlers.
type Router struct {
	mu      sync.Mutex
	routes  map[Method]Handler
	version uint16
}

func NewRouter(version uint16) *Router {
	return &Router{version: version, routes: make(map[Method]Handler)}
}

func (r *Router) Register(m Method, h Handler) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.routes[m] = h
}

func (r *Router) Dispatch(req Request, ci ClientIdentity) (Response, error) {
	if req.PipeVersion != r.version {
		return Response{ID: req.ID, OK: false,
			Error: &ServiceError{Code: "unsupported_version",
				Message: fmt.Sprintf("pipe version %d not supported; expected %d", req.PipeVersion, r.version),
				Retryable: false}}, nil
	}
	r.mu.Lock()
	h, ok := r.routes[req.Method]
	r.mu.Unlock()
	if !ok {
		return Response{ID: req.ID, OK: false,
			Error: &ServiceError{Code: "unknown_method",
				Message: fmt.Sprintf("method %s not recognized", req.Method),
				Retryable: false}}, nil
	}
	return h(req, ci)
}
