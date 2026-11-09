// Package modeowner_test tests cross-process mode ownership via the Windows
// named mutex.
package modeowner_test

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"sync"
	"testing"
	"time"
	"unsafe"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/modeowner"
)

func testMutexName(prefix string) string {
	return fmt.Sprintf(`Global\%s-%d-%d`, prefix, os.Getpid(), time.Now().UnixNano())
}

// TestNewMutexAcquisition tests acquiring a brand new named mutex.
func TestNewMutexAcquisition(t *testing.T) {
	name := testMutexName("test-new")
	o := modeowner.NewWithName(name)
	defer o.Close()

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
		t.Fatal("expected IsOwned to be true")
	}

	if err := o.Release(); err != nil {
		t.Fatalf("Release: %v", err)
	}
	if o.IsOwned() {
		t.Fatal("expected not owned after Release")
	}
}

// TestExistingUnownedMutexAcquisition tests acquiring a mutex that was previously
// created by another handle but is not currently held.
func TestExistingUnownedMutexAcquisition(t *testing.T) {
	name := testMutexName("test-existing")
	o1 := modeowner.NewWithName(name)
	defer o1.Close()

	// o1 acquires and releases (leaving the kernel mutex existing but unowned)
	owned1, err := o1.TryAcquire(5 * time.Second)
	if err != nil || !owned1 {
		t.Fatalf("o1 TryAcquire: owned=%v, err=%v", owned1, err)
	}
	if err := o1.Release(); err != nil {
		t.Fatalf("o1 Release: %v", err)
	}

	// o2 should now successfully acquire the existing mutex
	o2 := modeowner.NewWithName(name)
	defer o2.Close()

	owned2, err := o2.TryAcquire(5 * time.Second)
	if err != nil {
		t.Fatalf("o2 TryAcquire: %v", err)
	}
	if !owned2 {
		t.Fatal("expected o2 to acquire existing unowned mutex")
	}
	if err := o2.Release(); err != nil {
		t.Fatalf("o2 Release: %v", err)
	}
}

// TestContendedMutexTimeout tests that acquiring an already-held mutex times out.
func TestContendedMutexTimeout(t *testing.T) {
	name := testMutexName("test-contended")
	o1 := modeowner.NewWithName(name)
	defer o1.Close()

	owned1, err := o1.TryAcquire(5 * time.Second)
	if err != nil || !owned1 {
		t.Fatalf("o1 TryAcquire: owned=%v, err=%v", owned1, err)
	}

	o2 := modeowner.NewWithName(name)
	defer o2.Close()

	start := time.Now()
	owned2, err := o2.TryAcquire(250 * time.Millisecond)
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("expected error on contended mutex timeout")
	}
	if owned2 {
		t.Fatal("expected owned2 to be false")
	}
	if elapsed < 200*time.Millisecond {
		t.Fatalf("expected timeout to be respected, elapsed: %v", elapsed)
	}

	if err := o1.Release(); err != nil {
		t.Fatalf("o1 Release: %v", err)
	}
}

// TestReleaseFromDifferentGoroutine tests that Release called from an arbitrary
// goroutine is safely routed to the dedicated OS thread owner.
func TestReleaseFromDifferentGoroutine(t *testing.T) {
	name := testMutexName("test-diff-thread")
	o := modeowner.NewWithName(name)
	defer o.Close()

	owned, err := o.TryAcquire(5 * time.Second)
	if err != nil || !owned {
		t.Fatalf("TryAcquire: owned=%v, err=%v", owned, err)
	}

	var releaseErr error
	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		// Called from a different goroutine / OS thread
		releaseErr = o.Release()
	}()
	wg.Wait()

	if releaseErr != nil {
		t.Fatalf("Release from different goroutine failed: %v", releaseErr)
	}
	if o.IsOwned() {
		t.Fatal("expected mutex not owned after release")
	}
}

// TestAbandonedMutexRecovery tests that if a process holding the named mutex terminates
// without calling ReleaseMutex, the parent process detects WAIT_ABANDONED and recovers ownership.
func TestAbandonedMutexRecovery(t *testing.T) {
	if os.Getenv("GO_TEST_HELPER_ABANDON_MUTEX") == "1" {
		mutexName := os.Getenv("GO_TEST_MUTEX_NAME")
		sd, err := windows.SecurityDescriptorFromString(modeowner.MutexSDDL())
		if err != nil {
			os.Exit(2)
		}
		sa := &windows.SecurityAttributes{
			Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
			SecurityDescriptor: sd,
			InheritHandle:      0,
		}
		namePtr, err := windows.UTF16PtrFromString(mutexName)
		if err != nil {
			os.Exit(3)
		}
		h, err := windows.CreateMutex(sa, false, namePtr)
		if h == 0 {
			os.Exit(4)
		}
		// Acquire mutex
		event, _ := windows.WaitForSingleObject(h, 5000)
		if event != uint32(windows.WAIT_OBJECT_0) {
			os.Exit(5)
		}
		// Terminate immediately without releasing mutex or closing handle
		os.Exit(0)
	}

	name := testMutexName("test-abandoned")

	// Launch helper child process to acquire and abandon the mutex
	cmd := exec.Command(os.Args[0], "-test.run=TestAbandonedMutexRecovery")
	cmd.Env = append(os.Environ(),
		"GO_TEST_HELPER_ABANDON_MUTEX=1",
		"GO_TEST_MUTEX_NAME="+name,
	)
	if err := cmd.Run(); err != nil {
		t.Fatalf("helper process failed: %v", err)
	}

	// The child process exited while holding the mutex.
	// Now the parent attempts to acquire it; Windows should report WAIT_ABANDONED and grant ownership.
	o := modeowner.NewWithName(name)
	defer o.Close()

	owned, err := o.TryAcquire(5 * time.Second)
	if err != nil {
		t.Fatalf("TryAcquire after abandonment failed: %v", err)
	}
	if !owned {
		t.Fatal("expected to recover and acquire abandoned mutex")
	}
	if !o.IsOwned() {
		t.Fatal("expected IsOwned to be true on recovered mutex")
	}

	if err := o.Release(); err != nil {
		t.Fatalf("Release recovered mutex: %v", err)
	}
}

// TestOwnerCloseAndCleanup tests closing an Owner instance releases handles.
func TestOwnerCloseAndCleanup(t *testing.T) {
	name := testMutexName("test-close")
	o := modeowner.NewWithName(name)

	owned, err := o.TryAcquire(5 * time.Second)
	if err != nil || !owned {
		t.Fatalf("TryAcquire: %v", err)
	}

	if err := o.Close(); err != nil {
		t.Fatalf("Close failed: %v", err)
	}
	if o.IsOwned() {
		t.Fatal("expected IsOwned false after Close")
	}

	// Subsequent TryAcquire on closed owner should return error
	_, err = o.TryAcquire(100 * time.Millisecond)
	if err == nil {
		t.Fatal("expected error acquiring on closed owner")
	}
}

func TestMutexSDDLRestrictive(t *testing.T) {
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
