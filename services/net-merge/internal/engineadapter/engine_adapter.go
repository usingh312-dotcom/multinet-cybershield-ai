// Package engineadapter adapts the pinned upstream Net Merge engine
// (commit 2b333a3ad820996ba337588adfa8f7259a45baec, Release 0.3.7) to the
// ipc.Engine interface used by the MultiNet named-pipe service.
//
// Upstream attribution: net-merge is MIT licensed (Copyright 2026 Rohit Mahto).
// The upstream packages internal/engine, internal/nics, internal/tun and
// internal/platform are vendored under this module with their original
// copyright and license preserved. See LICENSE and THIRD_PARTY_NOTICES.md.
//
// This adapter is the single owner of the Wintun adapter, capture routes and
// per-adapter sockets (Mode A). It is deliberately separated from the Rust
// bonding engine (Mode B); the two modes are mutually exclusive and coordinated
// by the mode.Coordinator held by the service.
package engineadapter

import (
	"fmt"
	"sync"

	"golang.org/x/sys/windows"

	netengine "multinet.ai/netmerge-service/internal/engine"
	"multinet.ai/netmerge-service/internal/ipc"
	"multinet.ai/netmerge-service/internal/mode"
	"multinet.ai/netmerge-service/internal/nics"
	"multinet.ai/netmerge-service/internal/platform"
	"multinet.ai/netmerge-service/internal/tun"
)

// engine adapts upstream Net Merge to the ipc.Engine interface.
type engine struct {
	mu     sync.Mutex
	running bool
	sched  *netengine.Scheduler
	tun    *tun.Tun
	coord  *mode.Coordinator // authoritative mode A/B coordinator

	// Last scheduler snapshot for status reporting.
	lastSnapshot []netengine.SlotStats
}

// Compile-time interface check.
var _ ipc.Engine = (*engine)(nil)

// New creates an engine adapter. The caller must ensure the process is
// elevated (Wintun, routes and IP_UNICAST_IF require admin).
func New(coord *mode.Coordinator) *engine {
	return &engine{coord: coord}
}

// Discover enumerates physical network adapters using upstream nics discovery.
// Returns adapters eligible for merging plus skipped adapters with reasons.
func (e *engine) Discover() ([]ipc.AdapterInfo, []ipc.Skip, error) {
	nicsList, skipped, err := nics.DiscoverNicsDetailed(nil, nil)
	if err != nil {
		return nil, nil, fmt.Errorf("adapter discovery: %w", err)
	}
	out := make([]ipc.AdapterInfo, 0, len(nicsList))
	for _, n := range nicsList {
		out = append(out, ipc.AdapterInfo{
			Name:   n.Name,
			Index:  n.Index,
			IPv4:   ipAddrString(n.IP),
			IPv6:   ipAddrString(n.IP6),
			Weight: n.Weight,
		})
	}
	sk := make([]ipc.Skip, 0, len(skipped))
	for _, s := range skipped {
		sk = append(sk, ipc.Skip{Name: s.Name, Reason: s.Reason})
	}
	return out, sk, nil
}

// Start activates Mode A: acquires the mode mutex, creates the Wintun adapter,
// installs capture routes, starts health probes and the 1s throughput tick.
// Transactional: on failure everything started so far is rolled back and the
// mode mutex is released.
func (e *engine) Start(cfg ipc.StartConfig) error {
	// Acquire the authoritative mode mutex before touching any system state.
	if err := e.coord.RequestModeA(); err != nil {
		return err
	}
	modeAcquired := true
	defer func() {
		if !modeAcquired {
			_ = e.coord.ReleaseMode()
		}
	}()

	e.mu.Lock()
	if e.running {
		e.mu.Unlock()
		return fmt.Errorf("net-merge engine already running")
	}
	e.mu.Unlock()

	// Verify Wintun driver is present before taking any system action.
	if err := platform.CheckDriver(); err != nil {
		return fmt.Errorf("wintun driver check: %w", err)
	}

	// Discover adapters.
	nicsList, _, err := nics.DiscoverNicsDetailed(cfg.Adapters, cfg.Weights)
	if err != nil {
		return fmt.Errorf("adapter discovery: %w", err)
	}
	if len(nicsList) == 0 {
		return fmt.Errorf("no usable IPv4 adapters found")
	}

	// Build scheduler and balancer (upstream).
	sched := netengine.NewScheduler(nicsList)
	sched.SetAuto(cfg.AutoWeight && len(cfg.Weights) == 0)
	bal := netengine.NewBalancer(sched)
	bal.SetV6Mode(parseV6Mode(cfg.IPv6))

	// Create the Wintun TUN device and userland stack. On() creates the
	// adapter, configures address/MTU and installs capture routes.
	t := tun.New(bal)
	t.MTU = int(cfg.MTU)
	if err := t.On(); err != nil {
		return fmt.Errorf("tun start: %w", err)
	}
	rolledBack := false
	defer func() {
		if rolledBack {
			t.Close()
		}
	}()

	// Start background health probes.
	sched.RunProbes(make(chan struct{}))

	// Commit.
	e.mu.Lock()
	e.running = true
	e.sched = sched
	e.tun = t
	e.lastSnapshot = sched.Snapshot()
	e.mu.Unlock()
	rolledBack = false
	modeAcquired = false // mode is now owned by the running engine; Stop() releases it
	return nil
}

// Stop deactivates Mode A: removes capture routes, tears down the TUN,
// restores weak-host settings, and releases the mode mutex. Safe to call
// when already stopped.
func (e *engine) Stop() error {
	e.mu.Lock()
	if !e.running {
		e.mu.Unlock()
		return nil
	}
	t := e.tun
	e.running = false
	e.sched = nil
	e.tun = nil
	e.lastSnapshot = nil
	e.mu.Unlock()

	if t != nil {
		t.Close()
	}

	// Release the authoritative mode mutex.
	return e.coord.ReleaseMode()
}

// Status returns the current Mode A snapshot.
func (e *engine) Status() (ipc.Status, error) {
	e.mu.Lock()
	defer e.mu.Unlock()
	st := ipc.Status{
		On:      e.running,
		ModeA:   e.running,
		ModeB:   e.coord.IsBonding(),
		V6State: "auto",
	}
	for _, slot := range e.lastSnapshot {
		st.Adapters = append(st.Adapters, ipc.AdapterStatus{
			Name:         slot.Name,
			IPv4:         slot.IP,
			IPv6:         slot.IP6,
			State:        slot.State,
			Weight:       slot.Weight,
			ActiveFlows:  uint64(slot.Active),
			TotalFlows:   uint64(slot.Total),
			FailedFlows:  uint64(slot.Failed),
			BytesIn:      uint64(slot.BytesIn),
			BytesOut:     uint64(slot.BytesOut),
			RateInBps:    slot.RateIn,
			RateOutBps:   slot.RateOut,
			LimitMbps:    slot.LimitMbps,
		})
	}
	return st, nil
}

// SetLimit applies a per-adapter speed cap in Mbit/s.
func (e *engine) SetLimit(adapter string, mbps float64) error {
	e.mu.Lock()
	sched := e.sched
	e.mu.Unlock()
	if sched == nil {
		return fmt.Errorf("engine not running")
	}
	sched.SetLimit(adapter, mbps)
	return nil
}

// ModeBActive reports whether Mode B (Rust bonding) is active, excluding Mode A.
func (e *engine) ModeBActive() bool {
	return e.coord.IsBonding()
}

// SetModeB records the Mode B state. When active is true, the Rust bonding
// engine (identified by rustPid) is granted Mode B and the kernel mutex is
// acquired with the validated process handle. When false, Mode B is released
// and the mutex is freed. If Mode B is activated while Mode A is running, Mode
// A is stopped to preserve exclusivity.
func (e *engine) SetModeB(active bool, rustPid uint32, rustHandle windows.Handle) error {
	e.mu.Lock()
	if active {
		if e.running {
			// Mode A is running: stop it to preserve exclusivity.
			t := e.tun
			e.running = false
			e.sched = nil
			e.tun = nil
			e.lastSnapshot = nil
			e.mu.Unlock()
			if t != nil {
				t.Close()
			}
		} else {
			e.mu.Unlock()
		}
		// Grant Mode B and acquire the kernel mutex, monitoring the Rust process.
		if err := e.coord.RequestModeB(rustPid, rustHandle); err != nil {
			return err
		}
		e.coord.MonitorRustProcess(rustPid)
		return nil
	}
	// Deactivate Mode B: release the kernel mutex.
	return e.coord.ReleaseMode()
}

// ipAddrString renders a net.IP or "" if nil.
func ipAddrString(ip interface{ String() string }) string {
	if ip == nil {
		return ""
	}
	return ip.String()
}

// parseV6Mode maps the IPC ipv6 string to an engine V6Mode.
func parseV6Mode(s string) netengine.V6Mode {
	switch s {
	case "on":
		return netengine.V6On
	case "off":
		return netengine.V6Off
	default:
		return netengine.V6Auto
	}
}
