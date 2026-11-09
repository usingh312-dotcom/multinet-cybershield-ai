// Package lifecycle manages the Net Merge service process lifecycle:
// start, stop, health, and safe shutdown with route restoration.
//
// The service owns the Wintun adapter and capture routes exclusively in Mode
// A. MultiNet never touches routes or Wintun directly.
package lifecycle

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"time"
)

// Service manages a single Net Merge service instance.
type Service struct {
	mu      sync.Mutex
	cmd     *exec.Cmd
	pipe    string
	state   State
	started time.Time
	cfg     Config // captured config for stop/restart
}

type State int

const (
	Stopped State = iota
	Starting
	Running
	Stopping
	Failed
)

// Config holds service startup parameters.
type Config struct {
	Binary      string // path to net-merge-service.exe
	Pipe        string
	StateDir    string
	LogFile     string
	Adapters    []string
	Weights     map[string]float64
	AutoWeight  bool
	IPv6        string
	MTU         uint16
}

// New creates a Service (not started).
func New(cfg Config) *Service {
	return &Service{pipe: cfg.Pipe, state: Stopped, cfg: cfg}
}

// Start launches the net-merge-service binary in "run" mode and waits for the
// pipe to become reachable.
func (s *Service) Start(ctx context.Context, cfg Config) error {
	s.mu.Lock()
	if s.state == Running {
		return fmt.Errorf("service already running")
	}
	s.state = Starting
	s.cmd = nil
	s.cfg = cfg
	s.mu.Unlock()

	args := []string{
		"run",
		"-state-dir", cfg.StateDir,
		"-pipe", cfg.Pipe,
		"-log", cfg.LogFile,
		"-stats", "0",
	}
	if len(cfg.Adapters) > 0 {
		args = append(args, "-include", join(cfg.Adapters, ","))
	}
	for name, w := range cfg.Weights {
		args = append(args, "-weight", fmt.Sprintf("%s=%g", name, w))
	}
	if !cfg.AutoWeight {
		args = append(args, "-auto-weight=false")
	}
	args = append(args, "-ipv6", cfg.IPv6)
	args = append(args, "-mtu", fmt.Sprintf("%d", cfg.MTU))

	cmd := exec.CommandContext(ctx, cfg.Binary, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		s.mu.Lock()
		s.state = Failed
		s.mu.Unlock()
		return fmt.Errorf("start net-merge-service: %w", err)
	}

	s.mu.Lock()
	s.cmd = cmd
	s.started = time.Now()
	s.state = Running
	s.mu.Unlock()

	// Wait for the named pipe to accept connections (service is ready).
	deadline := time.Now().Add(30 * time.Second)
	for time.Now().Before(deadline) {
		if ctx.Err() != nil {
			return ctx.Err()
		}
		if pipeReady(cfg.Pipe) {
			return nil
		}
		time.Sleep(200 * time.Millisecond)
	}
	return fmt.Errorf("service did not become ready on pipe %s within 30s", cfg.Pipe)
}

// Stop requests a graceful shutdown and waits for the process to exit. On
// shutdown the service removes capture routes and restores weak-host settings.
func (s *Service) Stop(ctx context.Context) error {
	s.mu.Lock()
	if s.state == Stopped || s.cmd == nil {
		s.state = Stopped
		s.mu.Unlock()
		return nil
	}
	s.state = Stopping
	cmd := s.cmd
	s.mu.Unlock()

	// Send "off" via the control marker (state file) so the running instance
	// teardowns cleanly and restores routes.
	if err := sendOffMarker(s.cfg.StateDir); err != nil {
		return fmt.Errorf("send off marker: %w", err)
	}

	// Wait for process exit.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case err := <-done:
		s.mu.Lock()
		s.cmd = nil
		s.state = Stopped
		s.mu.Unlock()
		return err
	case <-ctx.Done():
		cmd.Process.Kill()
		<-done
		s.mu.Lock()
		s.cmd = nil
		s.state = Failed
		s.mu.Unlock()
		return ctx.Err()
	}
}

// State returns the current lifecycle state.
func (s *Service) State() State {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.state
}

// pipeReady checks whether the named pipe accepts a connection.
func pipeReady(pipe string) bool {
	_, err := os.Open(pipe)
	return err == nil
}

func sendOffMarker(stateDir string) error {
	// Writes an "off" marker file read by the running instance. The marker
	// path convention mirrors upstream net-merge state handling.
	p := filepath.Join(stateDir, "off")
	return os.WriteFile(p, []byte("off\n"), 0o600)
}

func join(ss []string, sep string) string {
	out := ""
	for i, s := range ss {
		if i > 0 {
			out += sep
		}
		out += s
	}
	return out
}
