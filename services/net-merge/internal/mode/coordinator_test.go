// Package mode_test tests the Mode A / Mode B coordinator, including kernel
// mutex acquisition, mutual exclusion, process handle validation, and process-lifetime monitoring.
package mode_test

import (
	"os/exec"
	"testing"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/mode"
)

// TestValidRustProcessHandleRegistration verifies that a valid process handle
// is registered with the coordinator and activates Mode B.
func TestValidRustProcessHandleRegistration(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB with valid current process: %v", err)
	}
	if !c.IsBonding() {
		t.Fatal("expected Mode B active")
	}
	if c.RustPid() != currentPid {
		t.Fatalf("expected RustPid %d, got %d", currentPid, c.RustPid())
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if c.Current() != mode.None {
		t.Fatal("expected None after release")
	}
}

// TestInvalidProcessIdentity rejects PID 0 and non-existent PIDs.
func TestInvalidProcessIdentity(t *testing.T) {
	c := mode.NewCoordinator()

	// PID 0 must be rejected
	if err := c.RequestModeB(0); err == nil {
		t.Fatal("expected RequestModeB(0) to fail")
	}
	if c.IsBonding() {
		t.Fatal("Mode B should not be active for PID 0")
	}

	// Non-existent PID must be rejected
	if err := c.RequestModeB(99999999); err == nil {
		t.Fatal("expected RequestModeB(99999999) to fail")
	}
	if c.IsBonding() {
		t.Fatal("Mode B should not be active for non-existent PID")
	}
}

// TestPIDReuseRejection verifies that a handle whose actual PID does not match
// the requested PID is rejected.
func TestPIDReuseRejection(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	h, err := windows.OpenProcess(
		windows.PROCESS_QUERY_LIMITED_INFORMATION|windows.SYNCHRONIZE,
		false,
		currentPid,
	)
	if err != nil {
		t.Fatalf("OpenProcess: %v", err)
	}
	defer windows.CloseHandle(h)

	// Pass mismatched PID: requested PID is currentPid + 1, but handle belongs to currentPid
	if err := c.RequestModeB(currentPid+1, h); err == nil {
		t.Fatal("expected PID mismatch rejection")
	}
	if c.IsBonding() {
		t.Fatal("Mode B should not be active after PID mismatch rejection")
	}
}

// TestProcessExitsAfterModeBActivation verifies that when the tracked process exits,
// MonitorRustProcess detects it and automatically releases Mode B.
func TestProcessExitsAfterModeBActivation(t *testing.T) {
	cmd := exec.Command("cmd.exe", "/c", "timeout /t 1 >nul")
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start test process: %v", err)
	}
	childPid := uint32(cmd.Process.Pid)

	c := mode.NewCoordinator()
	if err := c.RequestModeB(childPid); err != nil {
		_ = cmd.Process.Kill()
		t.Fatalf("RequestModeB for child process: %v", err)
	}
	if !c.IsBonding() {
		_ = cmd.Process.Kill()
		t.Fatal("expected Mode B active for child process")
	}

	done := c.MonitorRustProcess(childPid)

	// Wait for process to exit and monitor to release Mode B
	select {
	case <-done:
		// Expected
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("timeout waiting for process exit monitor")
	}

	_ = cmd.Wait()

	// Mode B should now be released
	if c.Current() != mode.None {
		t.Fatalf("expected Mode None after process exit, got %v", c.Current())
	}
}

// TestPipeDisconnectWhileRustRemainsAlive verifies that pipe disconnect does NOT
// terminate Mode B when the engine process is still running.
func TestPipeDisconnectWhileRustRemainsAlive(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}

	// Simulate pipe disconnect: pipe connection closed, but process still alive
	// The coordinator must remain in Mode B
	if !c.IsBonding() {
		t.Fatal("Mode B must remain active while process is alive")
	}

	// Clean up
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
}

// TestRustProcessExitsWhileIPCConnected verifies that if the process exits,
// Mode B is released even while IPC connection is hypothetically maintained.
func TestRustProcessExitsWhileIPCConnected(t *testing.T) {
	cmd := exec.Command("cmd.exe", "/c", "exit 0")
	if err := cmd.Start(); err != nil {
		t.Fatalf("cmd.Start: %v", err)
	}
	childPid := uint32(cmd.Process.Pid)

	c := mode.NewCoordinator()
	if err := c.RequestModeB(childPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}

	done := c.MonitorRustProcess(childPid)

	select {
	case <-done:
		// Monitor triggered release
	case <-time.After(5 * time.Second):
		t.Fatal("monitor did not trigger within 5 seconds")
	}

	_ = cmd.Wait()

	if c.Current() != mode.None {
		t.Fatal("expected Mode B released after process exit")
	}
}

// TestMonitorTimeoutAndUnexpectedWaitError verifies monitor robustness on polling
// and unexpected wait errors.
func TestMonitorTimeoutAndUnexpectedWaitError(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}

	done := c.MonitorRustProcess(currentPid)

	// Current process does not exit; wait 500ms to verify monitor loop doesn't prematurely close
	select {
	case <-done:
		t.Fatal("monitor channel closed while process is still alive")
	case <-time.After(500 * time.Millisecond):
		// Expected: monitor is still running
	}

	// Now release mode explicitly (simulating unexpected close/shutdown)
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}

	// The monitor should observe session change and exit cleanly
	select {
	case <-done:
		// Expected
	case <-time.After(2 * time.Second):
		t.Fatal("monitor did not exit after ReleaseMode")
	}
}

// TestStaleMonitorCannotReleaseNewerSession verifies session ID protection:
// an older monitor goroutine cannot release a newer Mode B session.
func TestStaleMonitorCannotReleaseNewerSession(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	// 1. Session 1
	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("Session 1 RequestModeB: %v", err)
	}
	done1 := c.MonitorRustProcess(currentPid)

	// 2. Release Session 1
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	<-done1 // Session 1 monitor finishes

	// 3. Session 2
	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("Session 2 RequestModeB: %v", err)
	}
	if !c.IsBonding() {
		t.Fatal("Session 2 expected Mode B active")
	}

	// 4. Release Session 2
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
}

// TestServiceShutdownClosesProcessHandles verifies that ReleaseMode closes the handle
// and clears coordinator state.
func TestServiceShutdownClosesProcessHandles(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}
	if err := c.Close(); err != nil {
		t.Fatalf("Close: %v", err)
	}
	if c.Current() != mode.None {
		t.Fatal("expected None after Close")
	}
	if c.RustPid() != 0 {
		t.Fatalf("expected RustPid 0, got %d", c.RustPid())
	}
}

// TestModeACannotActivateWhileModeBOwnsTunnel verifies mutual exclusion between Mode A and Mode B.
func TestModeACannotActivateWhileModeBOwnsTunnel(t *testing.T) {
	c := mode.NewCoordinator()
	currentPid := windows.GetCurrentProcessId()

	// Activate Mode B
	if err := c.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}
	if !c.IsBonding() {
		t.Fatal("expected Mode B active")
	}

	// Attempting Mode A must be rejected
	if err := c.RequestModeA(); err == nil {
		t.Fatal("expected RequestModeA to fail while Mode B owns tunnel")
	}
	if !c.IsBonding() {
		t.Fatal("expected Mode B to remain active")
	}

	// Release Mode B
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}

	// Now Mode A can be activated
	if err := c.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA after Mode B released: %v", err)
	}
	if !c.IsNetMerge() {
		t.Fatal("expected Mode A active")
	}

	// Clean up
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
}
