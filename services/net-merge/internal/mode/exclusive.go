// Package mode enforces Mode A / Mode B mutual exclusion across the product.
// Mode A (Net Merge connection-level load balancing) and Mode B (Rust packet-
// level bonding engine) must never be active simultaneously.
package mode

import (
	"fmt"
	"sync"
)

// Tracker is a cross-process mode guard backed by the IPC server's ModeState.
type Tracker struct {
	mu   sync.Mutex
	mode Mode
}

type Mode int

const (
	None Mode = iota
	NetMerge // Mode A
	Bonding  // Mode B
)

func NewTracker() *Tracker { return &Tracker{} }

// ActivateNetMerge switches to Mode A. Fails if Mode B is active.
func (t *Tracker) ActivateNetMerge() error {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.mode == Bonding {
		return fmt.Errorf("cannot activate Net Merge (Mode A) while bonding engine (Mode B) is active")
	}
	t.mode = NetMerge
	return nil
}

// ActivateBonding switches to Mode B. Fails if Mode A is active.
func (t *Tracker) ActivateBonding() error {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.mode == NetMerge {
		return fmt.Errorf("cannot activate bonding engine (Mode B) while Net Merge (Mode A) is active")
	}
	t.mode = Bonding
	return nil
}

// SetBonding records whether Mode B is active without the ActivateBonding
// mutual-exclusion error. Used when the Rust bonding-engine side reports its
// state over IPC. If Mode B is activated while Mode A is current, Mode A is
// deactivated first to preserve exclusivity.
func (t *Tracker) SetBonding(active bool) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if active {
		t.mode = Bonding
	} else if t.mode == Bonding {
		t.mode = None
	}
}

// SetNetMerge records whether Mode A is active without the ActivateNetMerge
// mutual-exclusion error. Used when the Net Merge side reports its state.
func (t *Tracker) SetNetMerge(active bool) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if active {
		t.mode = NetMerge
	} else if t.mode == NetMerge {
		t.mode = None
	}
}

// Deactivate clears the active mode.
func (t *Tracker) Deactivate() error {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.mode = None
	return nil
}

// Current returns the active mode.
func (t *Tracker) Current() Mode {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.mode
}

// IsNetMerge returns true if Mode A is active.
func (t *Tracker) IsNetMerge() bool {
	return t.Current() == NetMerge
}

// IsBonding returns true if Mode B is active.
func (t *Tracker) IsBonding() bool {
	return t.Current() == Bonding
}
