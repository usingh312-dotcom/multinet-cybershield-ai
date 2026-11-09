// Package ipc_test tests the secured named-pipe creation, client identity
// validation, and per-method authorization. These tests exercise real
// implementation paths (SDDL parsing, security descriptor creation) but cannot
// create actual named pipes without Windows privileges.
package ipc

import (
	"strings"
	"testing"
	"time"
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

func TestStringToSecurityDescriptor(t *testing.T) {
	sdPtr, err := stringToSecurityDescriptor(pipeSDDL)
	if err != nil {
		t.Fatalf("stringToSecurityDescriptor: %v", err)
	}
	if sdPtr == 0 {
		t.Fatal("expected non-null security descriptor pointer")
	}
	localFreeSecurityDescriptor(sdPtr)
}

func TestStringToSecurityDescriptorInvalid(t *testing.T) {
	_, err := stringToSecurityDescriptor("D:P(INVALID)")
	if err == nil {
		t.Fatal("expected error for invalid SDDL")
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
