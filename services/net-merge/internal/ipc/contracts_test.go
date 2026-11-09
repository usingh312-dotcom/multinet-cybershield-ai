// Package ipc_test tests the versioned JSON IPC contract and server security
// limits. These tests exercise real implementation paths, not mocked success
// responses.
//
// Vendored test patterns adapted from net-merge upstream tests; new contract
// tests are Multinet-specific.
package ipc

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

func TestMarshalRequestRoundtrip(t *testing.T) {
	req := Request{
		ID:           "test-id-1",
		PipeVersion:  PIPEVersion,
		Method:       MethodHealth,
		Params:       nil,
	}
	b, err := MarshalRequest(req)
	if err != nil {
		t.Fatalf("MarshalRequest: %v", err)
	}
	if !strings.HasSuffix(string(b), "\n") {
		t.Fatal("expected newline-terminated frame")
	}
	var back Request
	if err := json.Unmarshal(b[:len(b)-1], &back); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if back.ID != req.ID || back.PipeVersion != req.PipeVersion || back.Method != req.Method {
		t.Fatal("roundtrip mismatch")
	}
}

func TestUnmarshalResponseInvalidJSON(t *testing.T) {
	_, err := UnmarshalResponse([]byte("not json"))
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
}

func TestNewOKResponse(t *testing.T) {
	res, err := NewOKResponse("id-1", map[string]string{"status": "ok"})
	if err != nil {
		t.Fatalf("NewOKResponse: %v", err)
	}
	if !res.OK || res.ID != "id-1" {
		t.Fatal("bad ok response")
	}
}

func TestNewErrorResponse(t *testing.T) {
	res := NewErrorResponse("id-1", "mode_conflict", "bonding engine active", false)
	if res.OK || res.ID != "id-1" {
		t.Fatal("bad error response")
	}
	if res.Error == nil || res.Error.Code != "mode_conflict" || res.Error.Retryable {
		t.Fatal("bad error detail")
	}
}

func TestRouterUnknownMethod(t *testing.T) {
	r := NewRouter(PIPEVersion)
	res, err := r.Dispatch(Request{ID: "x", PipeVersion: PIPEVersion, Method: "nope"}, ClientIdentity{})
	if err != nil {
		t.Fatalf("dispatch should not error for unknown method: %v", err)
	}
	if res.OK || res.Error == nil || res.Error.Code != "unknown_method" {
		t.Fatalf("expected unknown_method error, got %+v", res)
	}
}

func TestRouterVersionMismatch(t *testing.T) {
	r := NewRouter(PIPEVersion)
	res, err := r.Dispatch(Request{ID: "x", PipeVersion: 99, Method: MethodHealth}, ClientIdentity{})
	if err != nil {
		t.Fatalf("dispatch should not error for version mismatch: %v", err)
	}
	if res.OK || res.Error == nil || res.Error.Code != "unsupported_version" {
		t.Fatalf("expected unsupported_version error, got %+v", res)
	}
}

func TestMaxMessageSizeConstant(t *testing.T) {
	if MaxMessageSize != 1<<20 {
		t.Fatalf("expected MaxMessageSize 1MiB, got %d", MaxMessageSize)
	}
}

func TestSetModeBParamsRoundtrip(t *testing.T) {
	p := SetModeBParams{Active: true}
	b, err := json.Marshal(p)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	var back SetModeBParams
	if err := json.Unmarshal(b, &back); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if !back.Active {
		t.Fatal("set_mode_b active not roundtripped")
	}
}

// assertServiceError is a helper used by engine lifecycle tests.
func assertServiceError(t *testing.T, res Response, wantCode string) {
	t.Helper()
	if res.OK {
		t.Fatal("expected error response, got ok")
	}
	if res.Error == nil || res.Error.Code != wantCode {
		t.Fatalf("expected error code %q, got %+v", wantCode, res)
	}
}

var _ = errors.New // keep errors import used if needed elsewhere
