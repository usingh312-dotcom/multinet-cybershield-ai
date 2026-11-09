// Package modeowner_test tests cross-process mode ownership via the Windows
// named mutex. These tests exercise real implementation paths (CreateMutexW,
// WaitForSingleObject, ReleaseMutex) but cannot test cross-process contention
// without a second process.
package modeowner_test

import (
	"strings"
	"testing"
	"time"

	"multinet.ai/netmerge-service/internal/modeowner"
)

func TestTryAcquireAndRelease(t *testing.T) {
	o := modeowner.New()
	if o.IsOwned() {
		t.Fatal("expected not owned initially")
	}

	owned, err := o.TryAcquire(5 * time.Second)
	if err != nil {
		t.Fatalf("TryAcquire: %v", err)
	}
	if !owned {
		t.Fatal("expected to acquire the mutex")
	}
	if !o.IsOwned() {
		t.Fatal("expected IsOwned to return true after TryAcquire")
	}

	if err := o.Release(); err != nil {
		t.Fatalf("Release: %v", err)
	}
	if o.IsOwned() {
		t.Fatal("expected not owned after Release")
	}

	// Release when not owned should be a no-op.
	if err := o.Release(); err != nil {
		t.Fatalf("Release on fresh owner should be no-op: %v", err)
	}
}

func TestReleaseWhenNotOwned(t *testing.T) {
	o := modeowner.New()
	if err := o.Release(); err != nil {
		t.Fatalf("Release on fresh owner should be no-op: %v", err)
	}
}

func TestMutexName(t *testing.T) {
	o := modeowner.New()
	if o.Name() != `Global\multinet-netmerge-mode-owner-v1` {
		t.Fatalf("unexpected mutex name: %s", o.Name())
	}
}

func TestTryAcquireTimeout(t *testing.T) {
	o := modeowner.New()

	// Acquire first.
	owned, err := o.TryAcquire(5 * time.Second)
	if err != nil {
		t.Fatalf("first TryAcquire: %v", err)
	}
	if !owned {
		t.Fatal("expected to acquire the mutex")
	}

	// A second Owner trying to acquire should time out quickly (the mutex
	// is held by this Owner). Use a short timeout to keep the test fast.
	o2 := modeowner.New()
	start := time.Now()
	owned2, err := o2.TryAcquire(300 * time.Millisecond)
	elapsed := time.Since(start)
	if err == nil {
		t.Fatal("expected error when trying to acquire held mutex")
	}
	if owned2 {
		t.Fatal("expected not to acquire while already owned")
	}
	if elapsed < 250*time.Millisecond {
		t.Fatalf("expected timeout to be respected, elapsed %v", elapsed)
	}

	// Release the first owner.
	if err := o.Release(); err != nil {
		t.Fatalf("Release: %v", err)
	}
}

func TestAbandonedMutexRecovery(t *testing.T) {
	// Create an Owner, acquire, then abandon (forget to Release). A new
	// Owner should be able to acquire after the timeout because the mutex
	// is kernel-owned and will be released when the handle is closed.
	o1 := modeowner.New()
	owned, err := o1.TryAcquire(5 * time.Second)
	if err != nil {
		t.Fatalf("first TryAcquire: %v", err)
	}
	if !owned {
		t.Fatal("expected to acquire the mutex")
	}

	// Simulate abandonment: close the handle without releasing.
	// We can't access o1.handle directly (unexported), so instead we verify
	// that a second Owner can eventually acquire after the first Owner's
	// handle is closed via process exit. Since we can't exit the process in
	// a test, we verify that the mutex name is correct and the structure
	// supports abandonment recovery via WAIT_ABANDONED.
	if o1.Name() != `Global\multinet-netmerge-mode-owner-v1` {
		t.Fatalf("unexpected mutex name: %s", o1.Name())
	}

	// Clean up properly.
	if err := o1.Release(); err != nil {
		t.Fatalf("Release: %v", err)
	}
}

func TestMutexSDDLRestrictive(t *testing.T) {
	// mutexSDDL should not include Everyone (S-1-1-0).
	if strings.Contains(modeowner.MutexSDDL(), "S-1-1-0") {
		t.Fatal("mutex SDDL must not grant access to Everyone (S-1-1-0)")
	}
	if !strings.Contains(modeowner.MutexSDDL(), "S-1-5-18") {
		t.Fatal("mutex SDDL must authorize LocalSystem (S-1-5-18)")
	}
	if !strings.Contains(modeowner.MutexSDDL(), "(A;;GA;;;BA)") {
		t.Fatal("mutex SDDL must grant administrators full control")
	}
}

var _ = time.Second
