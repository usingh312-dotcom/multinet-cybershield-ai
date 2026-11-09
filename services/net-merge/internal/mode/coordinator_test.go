// Package mode_test tests the Mode A / Mode B coordinator, including kernel
// mutex acquisition, mutual exclusion, and process-lifetime monitoring.
package mode_test

import (
	"errors"
	"testing"
	"time"

	"multinet.ai/netmerge-service/internal/mode"
)

func TestCoordinatorRequestModeA(t *testing.T) {
	c := mode.NewCoordinator()
	if err := c.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA: %v", err)
	}
	if !c.IsNetMerge() {
		t.Fatal("expected Mode A active")
	}
	if c.IsBonding() {
		t.Fatal("expected Mode B not active")
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if c.Current() != mode.None {
		t.Fatal("expected None after release")
	}
}

func TestCoordinatorRequestModeB(t *testing.T) {
	c := mode.NewCoordinator()
	if err := c.RequestModeB(4242); err != nil {
		t.Fatalf("RequestModeB: %v", err)
	}
	if !c.IsBonding() {
		t.Fatal("expected Mode B active")
	}
	if c.RustPid() != 4242 {
		t.Fatalf("expected RustPid 4242, got %d", c.RustPid())
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	if c.Current() != mode.None {
		t.Fatal("expected None after release")
	}
}

func TestCoordinatorMutualExclusion(t *testing.T) {
	c := mode.NewCoordinator()
	if err := c.RequestModeA(); err != nil {
		t.Fatalf("RequestModeA: %v", err)
	}
	// Mode B must be rejected while Mode A is active.
	if err := c.RequestModeB(1); err == nil {
		t.Fatal("expected RequestModeB to fail while Mode A is active")
	}
	if !c.IsNetMerge() {
		t.Fatal("expected Mode A to remain active")
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
	// After release, Mode B should succeed.
	if err := c.RequestModeB(2); err != nil {
		t.Fatalf("RequestModeB after release: %v", err)
	}
	if !c.IsBonding() {
		t.Fatal("expected Mode B active after release")
	}
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode: %v", err)
	}
}

func TestCoordinatorReleaseWhenNone(t *testing.T) {
	c := mode.NewCoordinator()
	if err := c.ReleaseMode(); err != nil {
		t.Fatalf("ReleaseMode when None should be no-op: %v", err)
	}
}

func TestCoordinatorMonitorRustProcessZeroPid(t *testing.T) {
	c := mode.NewCoordinator()
	done := c.MonitorRustProcess(0)
	select {
	case <-done:
		// expected: channel closed immediately for pid 0
	case <-time.After(time.Second):
		t.Fatal("expected done channel to close immediately for pid 0")
	}
}

var _ = errors.New
