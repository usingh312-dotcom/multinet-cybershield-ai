// Package mode enforces Mode A / Mode B mutual exclusion across the product.
// Mode A (Net Merge connection-level load balancing) and Mode B (Rust packet-
// level bonding engine) must never be active simultaneously.
//
// NM-06 architecture: the elevated Go service is the authoritative mode
// coordinator. It owns the kernel named mutex for the entire lifetime of the
// active mode. The unelevated Rust bonding engine requests Mode B through
// authenticated IPC; it never touches the kernel mutex directly. The coordinator
// tracks the Rust bonding-engine process and releases Mode B when the process
// exits (monitored via WaitForSingleObject on a validated process handle).
//
// The Rust process identity is bound to the authenticated named-pipe client.
// The service obtains the client's PID via GetNamedPipeClientProcessId() on the
// pipe connection handle — not from a caller-supplied value. This prevents PID
// reuse attacks and impersonation: a malicious IPC caller cannot supply an
// arbitrary PID and have the service monitor the wrong process.
package mode

import (
	"fmt"
	"sync"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/modeowner"
)

// kernel32 procs used for process-lifetime monitoring.
var (
	kernel32              = windows.NewLazySystemDLL("kernel32.dll")
	procWaitForSingleObject = kernel32.NewProc("WaitForSingleObject")
	procCloseHandle       = kernel32.NewProc("CloseHandle")
)

// PROCESS_SYNCHRONIZE is required for WaitForSingleObject on a process handle.
const PROCESS_SYNCHRONIZE = 0x00100000

// Coordinator is the authoritative Mode A / Mode B coordinator owned by the
// elevated Go service. It holds the kernel named mutex and tracks mode state
// plus the validated Rust bonding-engine process handle for lifetime monitoring.
type Coordinator struct {
	mu      sync.Mutex
	owner   *modeowner.Owner // kernel mutex for cross-process exclusion
	mode    Mode
	rustPid uint32                    // PID of the Rust bonding engine when Mode B is active
	rustHandle windows.Handle            // validated process handle for lifetime monitoring
}

// NewCoordinator creates a coordinator that does not yet own the mutex.
func NewCoordinator() *Coordinator {
	return &Coordinator{
		owner: modeowner.New(),
	}
}

// Current returns the active mode.
func (c *Coordinator) Current() Mode {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.mode
}

// IsNetMerge returns true if Mode A is active.
func (c *Coordinator) IsNetMerge() bool {
	return c.Current() == NetMerge
}

// IsBonding returns true if Mode B is active.
func (c *Coordinator) IsBonding() bool {
	return c.Current() == Bonding
}

// RustPid returns the tracked Rust bonding-engine PID, or 0 if none.
func (c *Coordinator) RustPid() uint32 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.rustPid
}

// RequestModeA attempts to activate Mode A (Net Merge). It fails if Mode B is
// active or if the kernel mutex cannot be acquired. The mutex is held for the
// entire lifetime of Mode A and released on ReleaseMode or process exit.
func (c *Coordinator) RequestModeA() error {
	c.mu.Lock()
	if c.mode == Bonding {
		c.mu.Unlock()
		return fmt.Errorf("cannot activate Net Merge (Mode A) while bonding engine (Mode B) is active")
	}
	if c.mode == NetMerge {
		c.mu.Unlock()
		return nil // already active
	}
	c.mu.Unlock()

	owned, err := c.owner.TryAcquire(5 * time.Second)
	if err != nil {
		return fmt.Errorf("mode mutex acquire: %w", err)
	}
	if !owned {
		return fmt.Errorf("another process owns the networking tunnel; refusing Mode A")
	}

	c.mu.Lock()
	if c.mode == Bonding {
		// Race: another goroutine activated Mode B while we waited. Release.
		c.mu.Unlock()
		if err := c.owner.Release(); err != nil {
			return fmt.Errorf("mode mutex release after race: %w", err)
		}
		return fmt.Errorf("cannot activate Net Merge (Mode A) while bonding engine (Mode B) is active")
	}
	c.mode = NetMerge
	c.mu.Unlock()
	return nil
}

// RequestModeB attempts to activate Mode B (Rust bonding). It fails if Mode A is
// active or if the kernel mutex cannot be acquired. rustPid must be the
// validated PID of the pipe client (obtained via GetNamedPipeClientProcessId),
// not a caller-supplied value. The coordinator stores this PID and monitors the
// corresponding process handle, releasing Mode B when the process exits.
func (c *Coordinator) RequestModeB(rustPid uint32) error {
	c.mu.Lock()
	if c.mode == NetMerge {
		c.mu.Unlock()
		return fmt.Errorf("cannot activate bonding engine (Mode B) while Net Merge (Mode A) is active")
	}
	if c.mode == Bonding {
		c.mu.Unlock()
		return nil // already active
	}
	c.mu.Unlock()

	owned, err := c.owner.TryAcquire(5 * time.Second)
	if err != nil {
		return fmt.Errorf("mode mutex acquire: %w", err)
	}
	if !owned {
		return fmt.Errorf("another process owns the networking tunnel; refusing Mode B")
	}

	c.mu.Lock()
	if c.mode == NetMerge {
		c.mu.Unlock()
		if err := c.owner.Release(); err != nil {
			return fmt.Errorf("mode mutex release after race: %w", err)
		}
		return fmt.Errorf("cannot activate bonding engine (Mode B) while Net Merge (Mode A) is active")
	}
	c.mode = Bonding
	c.rustPid = rustPid
	c.mu.Unlock()
	return nil
}

// ReleaseMode deactivates the current mode and releases the kernel mutex. Safe
// to call when no mode is active. Mode B ownership is released only after the
// validated Rust process handle is closed.
func (c *Coordinator) ReleaseMode() error {
	c.mu.Lock()
	if c.mode == None {
		c.mu.Unlock()
		return nil
	}
	c.mode = None
	c.rustPid = 0
	if c.rustHandle != 0 {
		windows.CloseHandle(c.rustHandle)
		c.rustHandle = 0
	}
	c.mu.Unlock()

	if err := c.owner.Release(); err != nil {
		return fmt.Errorf("mode mutex release: %w", err)
	}
	return nil
}

// SetRustProcessHandle binds a validated process handle to the active Mode B
// session. The handle is obtained by the service from
// GetNamedPipeClientProcessId() on the authenticated pipe connection, NOT from
// a caller-supplied value. This prevents PID reuse and impersonation attacks.
func (c *Coordinator) SetRustProcessHandle(rustPid uint32, handle windows.Handle) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.mode != Bonding {
		return fmt.Errorf("cannot set Rust process handle: Mode B is not active")
	}
	if c.rustHandle != 0 {
		windows.CloseHandle(c.rustHandle)
	}
	c.rustPid = rustPid
	c.rustHandle = handle
	return nil
}

// MonitorRustProcess starts a goroutine that watches the validated Rust
// bonding-engine process handle and calls ReleaseMode when it exits. If pid is
// 0, no monitoring is started. Returns a channel that is closed when the
// process is observed to have exited.
func (c *Coordinator) MonitorRustProcess(pid uint32) <-chan struct{} {
	done := make(chan struct{})
	if pid == 0 {
		close(done)
		return done
	}

	go func() {
		defer close(done)

		c.mu.Lock()
		h := c.rustHandle
		c.mu.Unlock()

		if h == 0 {
			// No validated handle; treat as exited.
			_ = c.ReleaseMode()
			return
		}

		// Wait with a bounded timeout so we poll rather than block forever.
		for {
			r1, _, _ := procWaitForSingleObject.Call(
				uintptr(h),
				uintptr(5*time.Second/time.Millisecond),
			)
			if r1 == uintptr(windows.WAIT_OBJECT_0) {
				// Process exited.
				_ = c.ReleaseMode()
				return
			}
			if r1 == uintptr(windows.WAIT_TIMEOUT) {
				// Still alive; poll again.
				continue
			}
			// Unexpected error; assume the process is gone and release.
			_ = c.ReleaseMode()
			return
		}
	}()
	return done
}

var _ = time.Second
