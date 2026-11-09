// Package modeowner provides cross-process ownership of the networking tunnel
// mode via a Windows named mutex. Only one process (the Net Merge service in
// Mode A, or the Rust bonding engine in Mode B) can own the tunnel at a time.
//
// The mutex is kernel-owned: if the owning process crashes, the mutex is
// automatically released, enabling safe recovery on restart.
//
// Thread-affinity requirement: Windows mutex ownership is thread-specific.
// The acquiring thread must also be the releasing thread. To guarantee this in
// Go's M:N scheduler, an Owner instance delegates all Win32 mutex operations
// to a dedicated worker goroutine that is locked to a single OS thread via
// runtime.LockOSThread() for its entire lifetime.
package modeowner

import (
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"golang.org/x/sys/windows"
)

// mutexName is the default named mutex used for cross-process mode ownership.
const mutexName = `Global\multinet-netmerge-mode-owner-v1`

// mutexSDDL restricts the named mutex to Administrators and LocalSystem only.
const mutexSDDL = "D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)"

// MutexSDDL returns the security descriptor for the named mutex.
func MutexSDDL() string {
	return mutexSDDL
}

type opType int

const (
	opAcquire opType = iota
	opRelease
	opClose
)

type mutexReq struct {
	op      opType
	timeout time.Duration
	resp    chan mutexResp
}

type mutexResp struct {
	owned bool
	err   error
}

// Owner tracks cross-process mode ownership via a Windows named mutex.
// All Win32 mutex calls (CreateMutexW, WaitForSingleObject, ReleaseMutex, CloseHandle)
// are executed on a dedicated OS thread locked with runtime.LockOSThread().
type Owner struct {
	name       string
	cmdCh      chan mutexReq
	ownedState atomic.Bool
	closed     atomic.Bool
	closeOnce  sync.Once
}

// New creates a new Owner using the production global mutex name.
func New() *Owner {
	return NewWithName(mutexName)
}

// NewWithName creates a new Owner with a custom or test mutex name.
func NewWithName(name string) *Owner {
	o := &Owner{
		name:  name,
		cmdCh: make(chan mutexReq),
	}
	go o.loop()
	return o
}

// loop runs on a dedicated OS thread locked with runtime.LockOSThread().
// It executes every Win32 mutex call for this Owner instance.
func (o *Owner) loop() {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	var handle windows.Handle
	var isHeld bool

	cleanup := func() {
		if isHeld && handle != 0 {
			_ = windows.ReleaseMutex(handle)
			isHeld = false
			o.ownedState.Store(false)
		}
		if handle != 0 {
			_ = windows.CloseHandle(handle)
			handle = 0
		}
	}
	defer cleanup()

	for req := range o.cmdCh {
		switch req.op {
		case opAcquire:
			if isHeld {
				req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: already owns the mode mutex")}
				continue
			}

			if handle == 0 {
				sd, err := windows.SecurityDescriptorFromString(mutexSDDL)
				if err != nil {
					req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: security descriptor: %w", err)}
					continue
				}
				sa := &windows.SecurityAttributes{
					Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
					SecurityDescriptor: sd,
					InheritHandle:      0,
				}
				namePtr, err := windows.UTF16PtrFromString(o.name)
				if err != nil {
					req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: utf16 conversion: %w", err)}
					continue
				}

				h, err := windows.CreateMutex(sa, false, namePtr)
				if h == 0 || (err != nil && err != windows.ERROR_ALREADY_EXISTS) {
					req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: create mutex failed: %w", err)}
					continue
				}
				handle = h
			}

			// Wait for ownership with the requested timeout.
			timeoutMs := uint32(req.timeout / time.Millisecond)
			event, err := windows.WaitForSingleObject(handle, timeoutMs)

			switch event {
			case uint32(windows.WAIT_OBJECT_0):
				isHeld = true
				o.ownedState.Store(true)
				req.resp <- mutexResp{owned: true, err: nil}

			case uint32(windows.WAIT_ABANDONED):
				// The previous owning thread terminated without releasing the mutex.
				// Mutex ownership is transferred to this thread.
				isHeld = true
				o.ownedState.Store(true)
				req.resp <- mutexResp{owned: true, err: nil}

			case uint32(windows.WAIT_TIMEOUT):
				req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: timed out waiting for mode mutex after %v", req.timeout)}

			default:
				req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: wait failed: event=0x%x, err=%w", event, err)}
			}

		case opRelease:
			if !isHeld {
				req.resp <- mutexResp{owned: false, err: nil}
				continue
			}
			err := windows.ReleaseMutex(handle)
			isHeld = false
			o.ownedState.Store(false)
			if err != nil {
				req.resp <- mutexResp{owned: false, err: fmt.Errorf("modeowner: release mutex failed: %w", err)}
			} else {
				req.resp <- mutexResp{owned: false, err: nil}
			}

		case opClose:
			cleanup()
			req.resp <- mutexResp{owned: false, err: nil}
			return
		}
	}
}

// TryAcquire attempts to acquire the mode mutex within timeout.
func (o *Owner) TryAcquire(timeout time.Duration) (bool, error) {
	if o.closed.Load() {
		return false, fmt.Errorf("modeowner: owner is closed")
	}
	respCh := make(chan mutexResp, 1)
	o.cmdCh <- mutexReq{
		op:      opAcquire,
		timeout: timeout,
		resp:    respCh,
	}
	res := <-respCh
	return res.owned, res.err
}

// Release releases the mode mutex on the dedicated owning OS thread.
// Safe to call even if not currently owned (no-op).
func (o *Owner) Release() error {
	if o.closed.Load() {
		return nil
	}
	respCh := make(chan mutexResp, 1)
	o.cmdCh <- mutexReq{
		op:   opRelease,
		resp: respCh,
	}
	res := <-respCh
	return res.err
}

// Close releases any held mutex, closes the handle, and terminates the dedicated thread.
func (o *Owner) Close() error {
	var err error
	o.closeOnce.Do(func() {
		o.closed.Store(true)
		respCh := make(chan mutexResp, 1)
		o.cmdCh <- mutexReq{
			op:   opClose,
			resp: respCh,
		}
		res := <-respCh
		close(o.cmdCh)
		err = res.err
	})
	return err
}

// IsOwned reports whether the mutex is currently owned by this Owner.
func (o *Owner) IsOwned() bool {
	return o.ownedState.Load()
}

// Name returns the mutex name.
func (o *Owner) Name() string {
	return o.name
}
