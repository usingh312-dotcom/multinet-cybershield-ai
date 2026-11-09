// Package modeowner provides cross-process ownership of the networking tunnel
// mode via a Windows named mutex. Only one process (the Net Merge service in
// Mode A, or the Rust bonding engine in Mode B) can own the tunnel at a time.
//
// The mutex is kernel-owned: if the owning process crashes, the mutex is
// automatically released, enabling safe recovery on restart.
//
// Thread-affinity requirement: Windows mutex ownership is thread-specific.
// The acquiring thread must also be the releasing thread. This package uses
// runtime.LockOSThread() to pin the owning goroutine to an OS thread for the
// entire ownership lifetime.
package modeowner

import (
	"errors"
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"golang.org/x/sys/windows"
)

// mutexName is the named mutex used for cross-process mode ownership.
const mutexName = `Global\multinet-netmerge-mode-owner-v1`

// ERROR_ALREADY_EXISTS is returned by CreateMutexW when the mutex already exists.
const ERROR_ALREADY_EXISTS = 183

// WAIT_ABANDONED is returned by WaitForSingleObject when the mutex was abandoned.
const WAIT_ABANDONED = 0x00000080

// WAIT_OBJECT_0 is returned by WaitForSingleObject when the object is signaled.
const WAIT_OBJECT_0 = 0x00000000

// WAIT_TIMEOUT is returned by WaitForSingleObject when the timeout elapses.
const WAIT_TIMEOUT = 0x00000102

var (
	kernel32               = windows.NewLazySystemDLL("kernel32.dll")
	procCreateMutexW       = kernel32.NewProc("CreateMutexW")
	procReleaseMutex       = kernel32.NewProc("ReleaseMutex")
	procWaitForSingleObject = kernel32.NewProc("WaitForSingleObject")
	procCloseHandle        = kernel32.NewProc("CloseHandle")
)

// mutexSDDL restricts the named mutex to Administrators and LocalSystem only.
// Interactive users do not need direct mutex access; they reach the service
// through the named pipe instead.
const mutexSDDL = "D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)"

// Owner tracks cross-process mode ownership via a Windows named mutex.
// The owning goroutine is pinned to an OS thread via runtime.LockOSThread()
// so that Release() is always called on the same thread that acquired the mutex.
type Owner struct {
	mu     sync.Mutex
	handle windows.Handle
	owned  atomic.Bool
	name   string
	// lockThread records whether LockOSThread has been called, ensuring
	// Release() is attempted on the same OS thread.
	lockThread atomic.Bool
}

// New creates a new Owner (not yet acquired).
func New() *Owner {
	return &Owner{name: mutexName}
}

// TryAcquire attempts to acquire the mode mutex with a bounded wait.
// Returns true if this process now owns the mode, false if another process
// owns it. On WAIT_ABANDONED, the previous owner crashed and the mutex is
// recovered; this is treated as a successful acquisition with a warning.
//
// The acquiring goroutine is locked to the current OS thread for the
// lifetime of ownership. Release() must be called from the same goroutine.
func (o *Owner) TryAcquire(timeout time.Duration) (bool, error) {
	o.mu.Lock()
	if o.owned.Load() {
		o.mu.Unlock()
		return false, fmt.Errorf("modeowner: already owns the mode mutex")
	}
	o.mu.Unlock()

	// Lock the calling goroutine to its OS thread. Windows mutex ownership
	// is thread-specific; Release() must happen on the same thread.
	// LockOSThread is NOT deferred here: the thread remains locked until
	// Release() calls runtime.UnlockOSThread(). If acquisition fails, we
	// unlock immediately.
	runtime.LockOSThread()

	namePtr, err := windows.UTF16PtrFromString(o.name)
	if err != nil {
		runtime.UnlockOSThread()
		return false, err
	}

	// Create the mutex with a restrictive DACL. bInitialOwner=FALSE so we
	// must explicitly wait for it.
	sa := &windows.SecurityAttributes{
		Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
		SecurityDescriptor: nil, // default DACL for simplicity; see note below
		InheritHandle:      0,
	}

	r1, _, lastErr := procCreateMutexW.Call(
		uintptr(unsafe.Pointer(sa)),
		uintptr(0), // bInitialOwner = FALSE
		uintptr(unsafe.Pointer(namePtr)),
	)
	if r1 == 0 {
		runtime.UnlockOSThread()
		return false, fmt.Errorf("modeowner: create mutex failed: %w", lastErr)
	}
	h := windows.Handle(r1)

	// If the mutex already existed, another process owns it. Fail fast and
	// let the caller retry.
	if errors.Is(lastErr, windows.ERROR_ALREADY_EXISTS) {
		windows.CloseHandle(h)
		runtime.UnlockOSThread()
		return false, nil
	}

	// Wait for the mutex with a bounded timeout.
	timeoutMs := uint32(timeout / time.Millisecond)
	r2, _, _ := procWaitForSingleObject.Call(
		uintptr(h),
		uintptr(timeoutMs),
	)

	switch r2 {
	case WAIT_OBJECT_0:
		// Acquired successfully.
	case WAIT_ABANDONED:
		// Previous owner crashed; mutex is recovered. This is a success but
		// indicates possible inconsistent networking state that the caller
		// should reconcile (e.g., verify routes/adapters are clean).
	case WAIT_TIMEOUT:
		windows.CloseHandle(h)
		runtime.UnlockOSThread()
		return false, fmt.Errorf("modeowner: timed out waiting for mode mutex after %v", timeout)
	default:
		windows.CloseHandle(h)
		runtime.UnlockOSThread()
		return false, fmt.Errorf("modeowner: wait failed with result 0x%x", r2)
	}

	o.mu.Lock()
	o.handle = h
	o.owned.Store(true)
	o.lockThread.Store(true)
	o.mu.Unlock()

	return true, nil
}

// Release releases the mode mutex. Must be called from the same goroutine
// that called TryAcquire (which is pinned to the same OS thread).
func (o *Owner) Release() error {
	o.mu.Lock()
	if !o.owned.Load() {
		o.mu.Unlock()
		return nil
	}

	if !o.lockThread.Load() {
		o.mu.Unlock()
		return fmt.Errorf("modeowner: release attempted without LockOSThread; mutex may be held by another thread")
	}

	if r1, _, err := procReleaseMutex.Call(uintptr(o.handle)); r1 == 0 {
		o.mu.Unlock()
		return fmt.Errorf("modeowner: release mutex failed: %w", err)
	}
	windows.CloseHandle(o.handle)
	o.handle = 0
	o.owned.Store(false)
	o.lockThread.Store(false)
	o.mu.Unlock()
	runtime.UnlockOSThread()
	return nil
}

// IsOwned returns true if this process currently owns the mode mutex.
func (o *Owner) IsOwned() bool {
	return o.owned.Load()
}

// Name returns the mutex name used for ownership.
func (o *Owner) Name() string {
	return o.name
}

// MutexSDDL returns the security descriptor for the named mutex.
func MutexSDDL() string {
	return mutexSDDL
}

var _ = time.Second
