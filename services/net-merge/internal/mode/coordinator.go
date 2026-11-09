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
//
// Process-handle lifecycle:
//   1. RequestModeB() acquires or validates a process handle via OpenProcess()
//   2. GetProcessId() is called to verify the PID matches (PID reuse check)
//   3. The validated handle is stored in the coordinator
//   4. MonitorRustProcess() waits on the stored handle (never re-opens by PID)
//   5. ReleaseMode() closes the handle and releases the kernel mutex
//
// A session ID (generation counter) protects against stale monitor callbacks:
//   - Each Mode B activation increments the session ID
//   - MonitorRustProcess() records the session ID when it starts
//   - ReleaseMode() increments the session ID, causing stale monitors to exit
package mode

import (
	"fmt"
	"sync"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/modeowner"
)

// PROCESS_QUERY_LIMITED_INFORMATION is the access right used to open a process
// for exit-status queries. PROCESS_SYNCHRONIZE is required for WaitForSingleObject
// on a process handle.
const (
	PROCESS_QUERY_LIMITED_INFORMATION = 0x00001000
	PROCESS_SYNCHRONIZE               = 0x00100000
)

// Coordinator is the authoritative Mode A / Mode B coordinator owned by the
// elevated Go service. It holds the kernel named mutex and tracks mode state
// plus the validated Rust bonding-engine process handle for lifetime monitoring.
type Coordinator struct {
	mu         sync.Mutex
	owner      *modeowner.Owner // kernel mutex for cross-process exclusion
	mode       Mode
	rustPid    uint32         // PID of the Rust bonding engine when Mode B is active
	rustHandle windows.Handle // validated process handle for lifetime monitoring
	session    uint64         // generation counter for stale monitor detection
}

// NewCoordinator creates a coordinator that does not yet own the mutex.
func NewCoordinator() *Coordinator {
	return &Coordinator{
		owner: modeowner.New(),
	}
}

// NewCoordinatorWithOwner creates a coordinator with an injected Owner (e.g. for isolated test mutexes).
func NewCoordinatorWithOwner(owner *modeowner.Owner) *Coordinator {
	return &Coordinator{
		owner: owner,
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

// sessionID returns the current session ID for stale-monitor detection.
// Must be called with c.mu held.
func (c *Coordinator) sessionID() uint64 {
	return c.session
}

// bumpSession invalidates all outstanding monitor goroutines by advancing the
// session ID. Any monitor started with an older session ID will exit on its
// next poll without calling ReleaseMode().
func (c *Coordinator) bumpSession() {
	c.session++
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
// active or if the kernel mutex cannot be acquired. rustPid is the validated
// PID of the engine process. An existing validated handle may optionally be
// provided; if omitted or zero, a new handle is opened with query and synchronize
// rights. Mode B is rejected if a valid process handle cannot be established.
func (c *Coordinator) RequestModeB(rustPid uint32, handle ...windows.Handle) error {
	if rustPid == 0 {
		return fmt.Errorf("invalid PID 0: Mode B requires a valid process")
	}

	c.mu.Lock()
	if c.mode == NetMerge {
		c.mu.Unlock()
		return fmt.Errorf("cannot activate bonding engine (Mode B) while Net Merge (Mode A) is active")
	}
	if c.mode == Bonding && c.rustPid == rustPid && c.rustHandle != 0 {
		c.mu.Unlock()
		return nil // already active with valid handle
	}
	c.mu.Unlock()

	var h windows.Handle
	var openedHere bool
	if len(handle) > 0 && handle[0] != 0 && handle[0] != windows.InvalidHandle {
		h = handle[0]
	} else {
		// Open process handle with query + synchronize rights
		var err error
		h, err = windows.OpenProcess(
			windows.PROCESS_QUERY_LIMITED_INFORMATION|windows.SYNCHRONIZE,
			false,
			rustPid,
		)
		if err != nil {
			return fmt.Errorf("cannot open process %d: %w", rustPid, err)
		}
		openedHere = true
	}

	// Verify the process ID matches the requested PID (PID reuse check)
	actualPid, err := windows.GetProcessId(h)
	if err != nil {
		if openedHere {
			windows.CloseHandle(h)
		}
		return fmt.Errorf("get process ID: %w", err)
	}
	if actualPid != rustPid {
		if openedHere {
			windows.CloseHandle(h)
		}
		return fmt.Errorf("process ID mismatch: requested %d, actual %d (PID reuse detected)", rustPid, actualPid)
	}

	// Acquire cross-process tunnel exclusion mutex
	owned, err := c.owner.TryAcquire(5 * time.Second)
	if err != nil {
		if openedHere {
			windows.CloseHandle(h)
		}
		return fmt.Errorf("mode mutex acquire: %w", err)
	}
	if !owned {
		if openedHere {
			windows.CloseHandle(h)
		}
		return fmt.Errorf("another process owns the networking tunnel; refusing Mode B")
	}

	c.mu.Lock()
	if c.mode == NetMerge {
		c.mu.Unlock()
		if openedHere {
			windows.CloseHandle(h)
		}
		if err := c.owner.Release(); err != nil {
			return fmt.Errorf("mode mutex release after race: %w", err)
		}
		return fmt.Errorf("cannot activate bonding engine (Mode B) while Net Merge (Mode A) is active")
	}

	// If there was an old handle, close it
	if c.rustHandle != 0 && c.rustHandle != h {
		windows.CloseHandle(c.rustHandle)
	}
	c.mode = Bonding
	c.rustPid = rustPid
	c.rustHandle = h
	c.bumpSession()
	c.mu.Unlock()
	return nil
}

// ReleaseMode deactivates the current mode and releases the kernel mutex. Safe
// to call when no mode is active. Mode B ownership is released only after the
// validated Rust process handle is closed. Any outstanding monitor
// goroutines are invalidated via session ID bumping.
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
	c.bumpSession()
	c.mu.Unlock()

	if err := c.owner.Release(); err != nil {
		return fmt.Errorf("mode mutex release: %w", err)
	}
	return nil
}

// Close deactivates mode and closes all held handles.
func (c *Coordinator) Close() error {
	err := c.ReleaseMode()
	if c.owner != nil {
		if oErr := c.owner.Close(); oErr != nil && err == nil {
			err = oErr
		}
	}
	return err
}

// SetRustProcessHandle binds a validated process handle to the active Mode B
// session. Used after obtaining the handle on the authenticated pipe client.
func (c *Coordinator) SetRustProcessHandle(rustPid uint32, handle windows.Handle) error {
	if rustPid == 0 || handle == 0 || handle == windows.InvalidHandle {
		return fmt.Errorf("invalid PID or handle")
	}
	actualPid, err := windows.GetProcessId(handle)
	if err != nil {
		return fmt.Errorf("get process ID: %w", err)
	}
	if actualPid != rustPid {
		return fmt.Errorf("PID mismatch: requested %d, handle has %d", rustPid, actualPid)
	}

	c.mu.Lock()
	defer c.mu.Unlock()
	if c.mode != Bonding {
		return fmt.Errorf("cannot set Rust process handle: Mode B is not active")
	}
	if c.rustHandle != 0 && c.rustHandle != handle {
		windows.CloseHandle(c.rustHandle)
	}
	c.rustPid = rustPid
	c.rustHandle = handle
	return nil
}

// MonitorRustProcess starts a goroutine that watches the validated Rust
// bonding-engine process handle and calls ReleaseMode when it exits. The
// session ID is captured at startup; if ReleaseMode() is called by another
// goroutine (e.g., mode switch or service shutdown), the session ID changes
// and this monitor exits without calling ReleaseMode() again.
func (c *Coordinator) MonitorRustProcess(rustPid uint32) <-chan struct{} {
	done := make(chan struct{})

	c.mu.Lock()
	h := c.rustHandle
	session := c.session
	currentMode := c.mode
	c.mu.Unlock()

	if h == 0 || currentMode != Bonding {
		close(done)
		return done
	}

	go func() {
		defer close(done)

		for {
			event, err := windows.WaitForSingleObject(h, 200)
			c.mu.Lock()
			if c.session != session || c.mode != Bonding {
				// Stale monitor or mode was released by another call; exit cleanly.
				c.mu.Unlock()
				return
			}
			c.mu.Unlock()

			if event == uint32(windows.WAIT_OBJECT_0) {
				// Process exited
				_ = c.ReleaseMode()
				return
			}
			if event == uint32(windows.WAIT_TIMEOUT) {
				// Polling interval expired; check condition again
				continue
			}
			// Unexpected wait error (e.g. handle closed or invalid)
			if err != nil || event == uint32(windows.WAIT_FAILED) {
				c.mu.Lock()
				if c.session == session && c.mode == Bonding {
					c.mu.Unlock()
					_ = c.ReleaseMode()
				} else {
					c.mu.Unlock()
				}
				return
			}
		}
	}()
	return done
}
