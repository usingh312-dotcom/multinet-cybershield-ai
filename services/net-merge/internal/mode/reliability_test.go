// Package mode_test contains automated reliability and fault-injection tests
// for the Mode A / Mode B coordinator and Windows named mutex.
package mode_test

import (
	"fmt"
	"os"
	"os/exec"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/mode"
	"multinet.ai/netmerge-service/internal/modeowner"
)

// TestRepeatedAlternatingModeSwitching verifies 20 consecutive transitions
// between Mode A and Mode B with full cleanup and zero leaks.
func TestRepeatedAlternatingModeSwitching(t *testing.T) {
	c := newTestCoordinator(t, "repeated-switch")
	currentPid := windows.GetCurrentProcessId()

	const iterations = 20
	for i := 0; i < iterations; i++ {
		// 1. Activate Mode A
		if err := c.RequestModeA(); err != nil {
			t.Fatalf("[iter %d] RequestModeA: %v", i, err)
		}
		if !c.IsNetMerge() {
			t.Fatalf("[iter %d] expected NetMerge mode active", i)
		}

		// Mode B must be rejected while Mode A is active
		if err := c.RequestModeB(currentPid); err == nil {
			t.Fatalf("[iter %d] expected RequestModeB to be rejected while Mode A active", i)
		}

		// Deactivate Mode A
		if err := c.ReleaseMode(); err != nil {
			t.Fatalf("[iter %d] ReleaseMode after Mode A: %v", i, err)
		}
		if c.Current() != mode.None {
			t.Fatalf("[iter %d] expected None mode, got %v", i, c.Current())
		}

		// 2. Activate Mode B
		if err := c.RequestModeB(currentPid); err != nil {
			t.Fatalf("[iter %d] RequestModeB: %v", i, err)
		}
		if !c.IsBonding() {
			t.Fatalf("[iter %d] expected Bonding mode active", i)
		}

		// Mode A must be rejected while Mode B is active
		if err := c.RequestModeA(); err == nil {
			t.Fatalf("[iter %d] expected RequestModeA to be rejected while Mode B active", i)
		}

		// Deactivate Mode B
		if err := c.ReleaseMode(); err != nil {
			t.Fatalf("[iter %d] ReleaseMode after Mode B: %v", i, err)
		}
		if c.Current() != mode.None {
			t.Fatalf("[iter %d] expected None mode, got %v", i, c.Current())
		}
	}
}

// TestRapidModeContentionConcurrent tests that concurrent goroutines requesting
// Mode A and Mode B never violate mutual exclusivity.
func TestRapidModeContentionConcurrent(t *testing.T) {
	c := newTestCoordinator(t, "mode-contention")
	currentPid := windows.GetCurrentProcessId()

	const goroutines = 12
	const operationsPerGoroutine = 5
	var wg sync.WaitGroup
	var activeBoth atomic.Int32

	for i := 0; i < goroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for op := 0; op < operationsPerGoroutine; op++ {
				if id%2 == 0 {
					// Request Mode A
					err := c.RequestModeA()
					if err == nil {
						// Verify invariant: Bonding must NOT be active
						if c.IsBonding() {
							activeBoth.Add(1)
						}
						time.Sleep(5 * time.Millisecond)
						_ = c.ReleaseMode()
					}
				} else {
					// Request Mode B
					err := c.RequestModeB(currentPid)
					if err == nil {
						// Verify invariant: NetMerge must NOT be active
						if c.IsNetMerge() {
							activeBoth.Add(1)
						}
						time.Sleep(5 * time.Millisecond)
						_ = c.ReleaseMode()
					}
				}
			}
		}(i)
	}

	wg.Wait()
	if activeBoth.Load() > 0 {
		t.Fatalf("INVARIANT VIOLATION: Mode A and Mode B active simultaneously %d times", activeBoth.Load())
	}

	_ = c.ReleaseMode()
	if c.Current() != mode.None {
		t.Fatalf("expected None after all goroutines finish, got %v", c.Current())
	}
}

// TestFaultInjectionBondingEngineCrashesUnexpectedly simulates an unexpected
// crash of the Rust bonding engine process and verifies automatic release of Mode B.
func TestFaultInjectionBondingEngineCrashesUnexpectedly(t *testing.T) {
	c := newTestCoordinator(t, "engine-crash")

	// Start a short-lived process that terminates quickly
	cmd := exec.Command("cmd.exe", "/c", "timeout /t 1 >nul")
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start simulated engine process: %v", err)
	}
	childPid := uint32(cmd.Process.Pid)

	// Activate Mode B bound to this child process
	if err := c.RequestModeB(childPid); err != nil {
		_ = cmd.Process.Kill()
		t.Fatalf("RequestModeB: %v", err)
	}
	if !c.IsBonding() {
		_ = cmd.Process.Kill()
		t.Fatal("expected Mode B active")
	}

	// Kill the child process unexpectedly to simulate a crash
	_ = cmd.Process.Kill()
	_ = cmd.Wait()

	// Wait for process monitor to detect termination and release Mode B
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		if !c.IsBonding() {
			break
		}
		time.Sleep(50 * time.Millisecond)
	}

	if c.IsBonding() {
		t.Fatal("Mode B was not released after simulated bonding engine crash")
	}

	// Mode A must now be acquirable
	if err := c.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA after crash recovery: %v", err)
	}
	_ = c.ReleaseMode()
}

// TestFaultInjectionAbandonedMutexStress tests that an ungracefully terminated
// process holding the mutex leaves the mutex in an abandoned state which the
// coordinator recovers via WAIT_ABANDONED.
func TestFaultInjectionAbandonedMutexStress(t *testing.T) {
	mutexName := fmt.Sprintf(`Global\test-abandon-stress-%d-%d`, os.Getpid(), time.Now().UnixNano())
	owner := modeowner.NewWithName(mutexName)
	c := mode.NewCoordinatorWithOwner(owner)
	defer c.Close()

	// Helper process acquires the mutex and exits abruptly
	cmd := exec.Command(os.Args[0], "-test.run=TestHelper_AbandonMutex")
	cmd.Env = append(os.Environ(), "TEST_ABANDON_MUTEX_NAME="+mutexName)
	if err := cmd.Start(); err != nil {
		t.Skipf("cannot start helper process: %v", err)
	}
	// Wait for subprocess to finish (it exits without calling ReleaseMutex)
	_ = cmd.Wait()

	// Coordinator must acquire the abandoned mutex cleanly via WAIT_ABANDONED
	if err := c.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA should recover abandoned mutex: %v", err)
	}
	if !c.IsNetMerge() {
		t.Fatal("expected Mode A active after abandoned mutex recovery")
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
}
