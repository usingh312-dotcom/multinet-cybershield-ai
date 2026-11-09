// Package ipc tests the secured named-pipe creation, client identity
// validation, and per-method authorization.
package ipc

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"golang.org/x/sys/windows"
)

func TestPipeSDDLIsRestrictive(t *testing.T) {
	// The SDDL must not grant access to Everyone (S-1-1-0).
	if strings.Contains(pipeSDDL, "S-1-1-0") {
		t.Fatal("SDDL must not grant access to Everyone (S-1-1-0)")
	}
	// Must not grant broad access to all Users (NM-05 hardening).
	// Users (S-1-5-32-545) is retained only for the interactive controller;
	// in production it should be narrowed to a specific user SID.
	if !strings.Contains(pipeSDDL, "(A;;GA;;;BA)") {
		t.Fatal("SDDL must grant administrators full control")
	}
	// DACL must be protected.
	if !strings.HasPrefix(pipeSDDL, "D:P") {
		t.Fatal("SDDL must have protected DACL (D:P)")
	}
	// LocalSystem must be authorized (for service operation).
	if !strings.Contains(pipeSDDL, "S-1-5-18") {
		t.Fatal("SDDL must authorize LocalSystem (S-1-5-18)")
	}
}

func TestSecurityDescriptorFromString(t *testing.T) {
	sd, err := windows.SecurityDescriptorFromString(pipeSDDL)
	if err != nil {
		t.Fatalf("SecurityDescriptorFromString: %v", err)
	}
	if sd == nil {
		t.Fatal("expected non-nil security descriptor")
	}
}

func TestSecurityDescriptorFromStringInvalid(t *testing.T) {
	_, err := windows.SecurityDescriptorFromString("D:P(INVALID)")
	if err == nil {
		t.Fatal("expected error for invalid SDDL")
	}
}

func TestSecurePipeCreation(t *testing.T) {
	pipeName := fmt.Sprintf("multinet-test-pipe-%d", windows.GetCurrentProcessId())
	h, err := securePipe(pipeName)
	if err != nil {
		t.Fatalf("securePipe failed: %v", err)
	}
	defer windows.CloseHandle(h)

	if h == 0 || h == windows.InvalidHandle {
		t.Fatal("expected valid pipe handle")
	}

	// Attempting to create a second pipe instance with FILE_FLAG_FIRST_PIPE_INSTANCE must fail.
	_, err2 := securePipe(pipeName)
	if err2 == nil {
		t.Fatal("expected second securePipe with FIRST_PIPE_INSTANCE to fail")
	}
}

func TestOpenValidatedProcess(t *testing.T) {
	currentPid := windows.GetCurrentProcessId()

	// 1. Valid current process registration
	h, err := openValidatedProcess(currentPid)
	if err != nil {
		t.Fatalf("openValidatedProcess(currentPid): %v", err)
	}
	defer windows.CloseHandle(h)

	actualPid, err := windows.GetProcessId(h)
	if err != nil {
		t.Fatalf("GetProcessId: %v", err)
	}
	if actualPid != currentPid {
		t.Fatalf("expected PID %d, got %d", currentPid, actualPid)
	}

	// 2. Reject PID 0
	if _, err := openValidatedProcess(0); err == nil {
		t.Fatal("expected openValidatedProcess(0) to fail")
	}

	// 3. Reject non-existent PID
	if _, err := openValidatedProcess(99999999); err == nil {
		t.Fatal("expected openValidatedProcess(99999999) to fail")
	}
}

func TestProcessIdentityValidation(t *testing.T) {
	currentPid := windows.GetCurrentProcessId()
	h, err := openValidatedProcess(currentPid)
	if err != nil {
		t.Fatalf("openValidatedProcess: %v", err)
	}
	defer windows.CloseHandle(h)

	// In test binary (.test.exe), verifyRustProcessIdentity should accept it
	if err := verifyRustProcessIdentity(h, currentPid); err != nil {
		t.Fatalf("verifyRustProcessIdentity failed for test process: %v", err)
	}

	// Custom validator rejects unauthorized process
	SetProcessValidator(func(handle windows.Handle, pid uint32) error {
		return fmt.Errorf("mock unauthorized identity for testing")
	})
	defer SetProcessValidator(nil)

	if err := verifyRustProcessIdentity(h, currentPid); err == nil {
		t.Fatal("expected verifyRustProcessIdentity to reject when validator returns error")
	}
}

func TestRequestTimeoutConstant(t *testing.T) {
	if RequestTimeout != 10*time.Second {
		t.Fatalf("expected RequestTimeout 10s, got %v", RequestTimeout)
	}
}

func TestAuthorizerRequireAdminMethods(t *testing.T) {
	auth := NewAuthorizer()

	// Methods that require admin.
	for _, m := range []string{string(MethodStart), string(MethodStop), string(MethodSetLimit), string(MethodSetModeB)} {
		if !auth.RequireAdmin[m] {
			t.Fatalf("method %s should require admin", m)
		}
	}

	// Methods that do not require admin.
	for _, m := range []string{string(MethodAdapters), string(MethodStatus), string(MethodHealth), string(MethodModeBStatus)} {
		if auth.RequireAdmin[m] {
			t.Fatalf("method %s should not require admin", m)
		}
	}
}

func TestAuthorizerAdminAllowed(t *testing.T) {
	auth := NewAuthorizer()
	ci := ClientIdentity{UserSid: "S-1-5-21-...", IsAdmin: true}
	if err := auth.Authorize(string(MethodStart), ci); err != nil {
		t.Fatalf("admin should be authorized for start: %v", err)
	}
	if err := auth.Authorize(string(MethodSetLimit), ci); err != nil {
		t.Fatalf("admin should be authorized for set_limit: %v", err)
	}
}

func TestAuthorizerSystemAllowed(t *testing.T) {
	auth := NewAuthorizer()
	ci := ClientIdentity{UserSid: "S-1-5-18", IsSystem: true}
	if err := auth.Authorize(string(MethodStart), ci); err != nil {
		t.Fatalf("LocalSystem should be authorized for start: %v", err)
	}
}

func TestAuthorizerUserAllowedForReadOnly(t *testing.T) {
	auth := NewAuthorizer()
	ci := ClientIdentity{UserSid: "S-1-5-21-...", IsUser: true}
	if err := auth.Authorize(string(MethodHealth), ci); err != nil {
		t.Fatalf("standard user should be authorized for health: %v", err)
	}
	if err := auth.Authorize(string(MethodStatus), ci); err != nil {
		t.Fatalf("standard user should be authorized for status: %v", err)
	}
	if err := auth.Authorize(string(MethodAdapters), ci); err != nil {
		t.Fatalf("standard user should be authorized for adapters: %v", err)
	}
}

func TestAuthorizerUserDeniedForAdminMethods(t *testing.T) {
	auth := NewAuthorizer()
	ci := ClientIdentity{UserSid: "S-1-5-21-...", IsUser: true}
	if err := auth.Authorize(string(MethodStart), ci); err == nil {
		t.Fatal("standard user should NOT be authorized for start")
	}
	if err := auth.Authorize(string(MethodStop), ci); err == nil {
		t.Fatal("standard user should NOT be authorized for stop")
	}
	if err := auth.Authorize(string(MethodSetLimit), ci); err == nil {
		t.Fatal("standard user should NOT be authorized for set_limit")
	}
	if err := auth.Authorize(string(MethodSetModeB), ci); err == nil {
		t.Fatal("standard user should NOT be authorized for set_mode_b")
	}
}

func TestAuthorizerUnknownIdentityDenied(t *testing.T) {
	auth := NewAuthorizer()
	ci := ClientIdentity{UserSid: "S-1-0-0", IsAdmin: false, IsUser: false, IsSystem: false}
	if err := auth.Authorize(string(MethodHealth), ci); err == nil {
		t.Fatal("unknown identity should NOT be authorized")
	}
}

var _ = time.Second
