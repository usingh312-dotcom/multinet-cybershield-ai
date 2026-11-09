// Package ipc implements the named-pipe server that exposes the Net Merge
// service to MultiNet over a restricted Windows named pipe.
//
// The server creates \\.\pipe\multinet-netmerge-v1 with CreateNamedPipe and a
// restrictive security descriptor (pipesec_windows.go), speaks the versioned
// JSON contract (internal/ipc.contracts), and delegates privileged operations
// (Wintun, routes) to the Net Merge engine. It never exposes arbitrary command
// execution.
//
// Each client connection is authenticated by impersonating the client and
// validating its Windows security identity before any request is processed.
package ipc

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"unsafe"

	"golang.org/x/sys/windows"
)

// kernel32 proc for getting the named-pipe client process ID.
var (
	procGetNamedPipeClientProcessId = windows.NewLazySystemDLL("kernel32.dll").NewProc("GetNamedPipeClientProcessId")
)

// getPipeClientPID returns the process ID of the client connected to the given
// named pipe handle. This is the trusted way to identify the pipe client:
// the PID comes from the kernel, not from a caller-supplied value.
func getPipeClientPID(pipe windows.Handle) (uint32, error) {
	var pid uint32
	r1, _, _ := procGetNamedPipeClientProcessId.Call(
		uintptr(pipe),
		uintptr(unsafe.Pointer(&pid)),
	)
	if r1 == 0 {
		return 0, fmt.Errorf("GetNamedPipeClientProcessId failed")
	}
	return pid, nil
}

// Server is the named-pipe IPC server for the Net Merge service.
type Server struct {
	pipeName string
	router   *Router
	engine   Engine
	auth     *Authorizer
	mu       sync.Mutex
	closed   bool
	pipe     windows.Handle // current listening pipe instance

	// Concurrency guard: limits simultaneous pipe connections.
	connSem chan struct{}
}

// Engine is the subset of Net Merge functionality the IPC server needs.
// The real implementation wraps upstream net-merge (engine.Scheduler,
// engine.Balancer, tun.Tun).
type Engine interface {
	Discover() ([]AdapterInfo, []Skip, error)
	Start(cfg StartConfig) error
	Stop() error
	Status() (Status, error)
	SetLimit(adapter string, mbps float64) error
	ModeBActive() bool
	SetModeB(active bool, rustPid uint32) error
}

// AdapterInfo mirrors nics.Nic for the IPC boundary.
type AdapterInfo struct {
	Name   string
	Index  int
	IPv4   string
	IPv6   string
	Weight float64
}

// Skip is an adapter that was seen but not used.
type Skip struct {
	Name   string
	Reason string
}

// StartConfig carries the parameters for starting Mode A.
type StartConfig struct {
	Adapters   []string
	Weights    map[string]float64
	AutoWeight bool
	IPv6       string
	MTU        uint16
}

// Status is the per-adapter snapshot.
type Status struct {
	On       bool
	ModeA    bool
	ModeB    bool
	Adapters []AdapterStatus
	V6State  string
}

// NewServer creates a server bound to pipeName.
func NewServer(pipeName string, engine Engine) *Server {
	return &Server{
		pipeName: pipeName,
		router:   NewRouter(PIPEVersion),
		engine:   engine,
		auth:     NewAuthorizer(),
		connSem:  make(chan struct{}, MaxConcurrentConns),
	}
}

// RegisterHandlers wires the built-in method handlers.
func (s *Server) RegisterHandlers() {
	s.router.Register(MethodHealth, s.handleHealth)
	s.router.Register(MethodStatus, s.handleStatus)
	s.router.Register(MethodAdapters, s.handleAdapters)
	s.router.Register(MethodStart, s.handleStart)
	s.router.Register(MethodStop, s.handleStop)
	s.router.Register(MethodSetLimit, s.handleSetLimit)
	s.router.Register(MethodModeBStatus, s.handleModeBStatus)
	s.router.Register(MethodSetModeB, s.handleSetModeB)
}

// Run creates the secured named pipe and accepts client connections until ctx
// is done. Each accepted connection is validated (client identity) and then
// handled in a goroutine via HandleConn.
func (s *Server) Run(ctx context.Context) error {
	s.RegisterHandlers()

	// Create the first pipe instance with a restrictive security descriptor.
	pipe, err := securePipe(s.pipeName)
	if err != nil {
		return fmt.Errorf("create named pipe %s: %w", s.pipeName, err)
	}
	s.mu.Lock()
	s.pipe = pipe
	s.mu.Unlock()
	defer windows.CloseHandle(pipe)

	log.Printf("net-merge IPC listening on %s", s.pipeName)

	for {
		// Block until a client connects to this pipe instance.
		if err := windows.ConnectNamedPipe(pipe, nil); err != nil {
			if ctx.Err() != nil {
				return nil
			}
			// ERROR_PIPE_CONNECTED (535) means a client connected before
			// ConnectNamedPipe was called; treat as connected.
			if err != windows.ERROR_PIPE_CONNECTED {
				log.Printf("ipc: connect named pipe error: %v", err)
				// Create a fresh pipe instance and retry.
				pipe, err = securePipe(s.pipeName)
				if err != nil {
					return fmt.Errorf("recreate named pipe: %w", err)
				}
				s.mu.Lock()
				s.pipe = pipe
				s.mu.Unlock()
				continue
			}
		}

		// Validate the client's Windows security identity before processing.
		ci, err := validateClientIdentity(pipe)
		if err != nil {
			log.Printf("ipc: unauthorized client rejected: %v", err)
			windows.DisconnectNamedPipe(pipe)
			// Create a fresh pipe instance for the next client.
			pipe, err = securePipe(s.pipeName)
			if err != nil {
				return fmt.Errorf("recreate named pipe after rejection: %w", err)
			}
			s.mu.Lock()
			s.pipe = pipe
			s.mu.Unlock()
			continue
		}

		// Create the next pipe instance so we can keep accepting while
		// handling this client.
		nextPipe, err := securePipe(s.pipeName)
		if err != nil {
			log.Printf("ipc: create next pipe instance: %v", err)
			windows.DisconnectNamedPipe(pipe)
			continue
		}

		// Handle the current client; the goroutine owns the handle and
		// disconnects/closes it when done.
		go func() {
			// Concurrency limit: reject if too many simultaneous connections.
			select {
			case s.connSem <- struct{}{}:
				defer func() { <-s.connSem }()
			default:
				log.Printf("ipc: too many concurrent connections, rejecting")
				windows.DisconnectNamedPipe(pipe)
				windows.CloseHandle(pipe)
				return
			}
			s.HandleConn(pipe, ci)
		}()

		// Move to the next pipe instance.
		s.mu.Lock()
		s.pipe = nextPipe
		s.mu.Unlock()
		pipe = nextPipe
	}
}

// Close shuts down the listening pipe instance.
func (s *Server) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		return nil
	}
	s.closed = true
	if s.pipe != 0 {
		windows.DisconnectNamedPipe(s.pipe)
		windows.CloseHandle(s.pipe)
		s.pipe = 0
	}
	return nil
}

// writeResp marshals and writes a response to an *os.File (wrapping a pipe
// handle), terminated by a newline.
func writeResp(f *os.File, resp Response) error {
	b, err := json.Marshal(resp)
	if err != nil {
		return err
	}
	b = append(b, '\n')
	_, err = f.Write(b)
	return err
}

// PipeACL returns the security descriptor (SDDL) for the named pipe.
func PipeACL() string {
	return pipeSDDL
}

// signalWatcher returns a channel closed on SIGINT/SIGTERM for graceful
// shutdown.
func signalWatcher(ctx context.Context) {
	ch := make(chan os.Signal, 1)
	signal.Notify(ch, syscall.SIGINT, syscall.SIGTERM)
	<-ch
}

// Handlers ---------------------------------------------------------------

func (s *Server) handleHealth(req Request, ci ClientIdentity) (Response, error) {
	st, err := s.engine.Status()
	if err != nil {
		return Response{}, err
	}
	res, err := NewOKResponse(req.ID, HealthResult{
		Service:       "net-merge",
		Version:       "0.3.7-multinet",
		PipeVersion:   PIPEVersion,
		ModeAActive:   st.ModeA,
		ModeBActive:   st.ModeB,
		On:            st.On,
	})
	if err != nil {
		return Response{}, err
	}
	return res, nil
}

func (s *Server) handleStatus(req Request, ci ClientIdentity) (Response, error) {
	st, err := s.engine.Status()
	if err != nil {
		return Response{}, err
	}
	adapters := make([]AdapterStatus, 0, len(st.Adapters))
	for _, a := range st.Adapters {
		adapters = append(adapters, AdapterStatus{
			Name:         a.Name,
			Index:        uint32(a.Index),
			IPv4:         a.IPv4,
			IPv6:         a.IPv6,
			State:        a.State,
			Weight:       a.Weight,
			ActiveFlows:  a.ActiveFlows,
			TotalFlows:   a.TotalFlows,
			FailedFlows:  a.FailedFlows,
			BytesIn:      a.BytesIn,
			BytesOut:     a.BytesOut,
			RateInBps:    a.RateInBps,
			RateOutBps:   a.RateOutBps,
			LimitMbps:    a.LimitMbps,
		})
	}
	return NewOKResponse(req.ID, Status{
		On:       st.On,
		ModeA:    st.ModeA,
		ModeB:    st.ModeB,
		Adapters: adapters,
		V6State:  st.V6State,
	})
}

func (s *Server) handleAdapters(req Request, ci ClientIdentity) (Response, error) {
	adapters, skipped, err := s.engine.Discover()
	if err != nil {
		return Response{}, err
	}
	out := struct {
		Adapters []AdapterInfo `json:"adapters"`
		Skipped  []Skip        `json:"skipped"`
	}{Adapters: adapters, Skipped: skipped}
	return NewOKResponse(req.ID, out)
}

func (s *Server) handleStart(req Request, ci ClientIdentity) (Response, error) {
	var params StartParams
	if err := json.Unmarshal(req.Params, &params); err != nil {
		return NewErrorResponse(req.ID, "invalid_params", err.Error(), false), nil
	}
	if s.engine.ModeBActive() {
		return NewErrorResponse(req.ID, "mode_conflict", "bonding engine (Mode B) is active; stop it first", false), nil
	}
	cfg := StartConfig{
		Adapters:   params.Adapters,
		Weights:    params.Weights,
		AutoWeight: params.AutoWeight,
		IPv6:       params.IPv6,
		MTU:        params.MTU,
	}
	if err := s.engine.Start(cfg); err != nil {
		return NewErrorResponse(req.ID, "start_failed", err.Error(), true), nil
	}
	return NewOKResponse(req.ID, map[string]string{"status": "started"})
}

func (s *Server) handleStop(req Request, ci ClientIdentity) (Response, error) {
	if err := s.engine.Stop(); err != nil {
		return NewErrorResponse(req.ID, "stop_failed", err.Error(), true), nil
	}
	return NewOKResponse(req.ID, map[string]string{"status": "stopped"})
}

func (s *Server) handleSetLimit(req Request, ci ClientIdentity) (Response, error) {
	var params struct {
		Adapter     string  `json:"adapter"`
		MbitPerSec  float64 `json:"mbit_per_sec"`
	}
	if err := json.Unmarshal(req.Params, &params); err != nil {
		return NewErrorResponse(req.ID, "invalid_params", err.Error(), false), nil
	}
	if err := s.engine.SetLimit(params.Adapter, params.MbitPerSec); err != nil {
		return NewErrorResponse(req.ID, "set_limit_failed", err.Error(), true), nil
	}
	return NewOKResponse(req.ID, map[string]string{"status": "applied"})
}

func (s *Server) handleModeBStatus(req Request, ci ClientIdentity) (Response, error) {
	return NewOKResponse(req.ID, map[string]bool{"active": s.engine.ModeBActive()})
}

func (s *Server) handleSetModeB(req Request, ci ClientIdentity) (Response, error) {
	var params SetModeBParams
	if err := json.Unmarshal(req.Params, &params); err != nil {
		return NewErrorResponse(req.ID, "invalid_params", err.Error(), false), nil
	}
	// Bind the Rust process identity to the authenticated pipe client using
	// ci.ClientPid, which was obtained via GetNamedPipeClientProcessId() in
	// validateClientIdentity(). We do NOT trust a caller-supplied PID: the PID
	// comes from the kernel, preventing PID reuse and impersonation attacks
	// where a malicious caller supplies an arbitrary PID.
	if err := s.engine.SetModeB(params.Active, ci.ClientPid); err != nil {
		return NewErrorResponse(req.ID, "set_mode_b_failed", err.Error(), true), nil
	}
	return NewOKResponse(req.ID, map[string]bool{"active": params.Active, "mode_b": params.Active})
}
