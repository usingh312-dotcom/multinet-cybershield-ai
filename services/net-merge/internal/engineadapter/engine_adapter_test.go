// Package engineadapter_test tests the engine adapter lifecycle and mode
// exclusivity using real implementation paths (not mocked success responses).
package engineadapter_test

import (
	"errors"
	"fmt"
	"os"
	"testing"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/engineadapter"
	"multinet.ai/netmerge-service/internal/ipc"
	"multinet.ai/netmerge-service/internal/mode"
	"multinet.ai/netmerge-service/internal/modeowner"
)

func newTestCoordinator(t *testing.T, prefix string) *mode.Coordinator {
	name := fmt.Sprintf(`Global\test-eng-coord-%s-%d-%d`, prefix, os.Getpid(), time.Now().UnixNano())
	o := modeowner.NewWithName(name)
	c := mode.NewCoordinatorWithOwner(o)
	t.Cleanup(func() {
		_ = c.Close()
	})
	return c
}

// mockEngine implements ipc.Engine for testing mode exclusivity without
// touching Wintun or routes.
type mockEngine struct {
	running bool
	modeB   bool
}

func (m *mockEngine) Discover() ([]ipc.AdapterInfo, []ipc.Skip, error) {
	return nil, nil, nil
}
func (m *mockEngine) Start(cfg ipc.StartConfig) error {
	m.running = true
	return nil
}
func (m *mockEngine) Stop() error {
	m.running = false
	return nil
}
func (m *mockEngine) Status() (ipc.Status, error) {
	return ipc.Status{On: m.running, ModeA: m.running, ModeB: m.modeB}, nil
}
func (m *mockEngine) SetLimit(adapter string, mbps float64) error { return nil }
func (m *mockEngine) ModeBActive() bool                           { return m.modeB }
func (m *mockEngine) SetModeB(active bool, rustPid uint32, rustHandle windows.Handle) error {
	m.modeB = active
	return nil
}

func TestModeExclusivityStartRejectsModeB(t *testing.T) {
	coord := newTestCoordinator(t, "start-rejects-mode-b")
	currentPid := windows.GetCurrentProcessId()
	if err := coord.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}
	eng := engineadapter.New(coord)
	// Start should reject because Mode B is active.
	if err := eng.Start(ipc.StartConfig{}); err == nil {
		t.Fatal("expected start to reject when Mode B is active")
	}
	if !coord.IsBonding() {
		t.Fatal("coordinator should report Mode B active")
	}
}

func TestModeExclusivityActivations(t *testing.T) {
	coord := newTestCoordinator(t, "activations")
	currentPid := windows.GetCurrentProcessId()
	if err := coord.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA: %v", err)
	}
	if !coord.IsNetMerge() {
		t.Fatal("expected Mode A active")
	}
	// Switching to Mode B while Mode A is active must fail.
	if err := coord.RequestModeB(currentPid); err == nil {
		t.Fatal("expected mode conflict activating Mode B over Mode A")
	}
	if err := coord.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if coord.Current() != mode.None {
		t.Fatal("expected None after release")
	}
}

func TestSetBondingAndSetNetMerge(t *testing.T) {
	coord := newTestCoordinator(t, "set-bonding-and-net-merge")
	currentPid := windows.GetCurrentProcessId()
	if err := coord.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA: %v", err)
	}
	if !coord.IsNetMerge() {
		t.Fatal("expected Mode A after RequestModeA(true)")
	}
	if err := coord.RequestModeB(currentPid); err == nil {
		t.Fatal("expected RequestModeB to fail while Mode A is active")
	}
	if !coord.IsNetMerge() {
		t.Fatal("expected Mode A to remain active after failed Mode B request")
	}
	if err := coord.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if coord.Current() != mode.None {
		t.Fatal("expected None after release")
	}
	if err := coord.RequestModeB(currentPid); err != nil {
		t.Fatalf("RequestModeB after release: %v", err)
	}
	if !coord.IsBonding() || coord.RustPid() != currentPid {
		t.Fatalf("expected Mode B with pid %d", currentPid)
	}
	if err := coord.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if coord.Current() != mode.None {
		t.Fatal("expected None after second release")
	}
}

var _ = errors.New // keep import used
