// Package svcrunner implements the Windows Service Control Manager (SCM) runner,
// event logger, and service management for the Net Merge service.
package svcrunner

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"golang.org/x/sys/windows"
	"golang.org/x/sys/windows/svc"
	"golang.org/x/sys/windows/svc/eventlog"
	"golang.org/x/sys/windows/svc/mgr"

	"multinet.ai/netmerge-service/internal/engineadapter"
	"multinet.ai/netmerge-service/internal/ipc"
	"multinet.ai/netmerge-service/internal/mode"
)

const (
	ServiceName        = "MultiNetNetMerge"
	ServiceDisplayName = "MultiNet Net-Merge Service"
	ServiceDescription = "Manages virtual network adapter bonding, tunnel exclusivity, and packet routing for MultiNet CyberShield AI."

	// Event IDs for Windows Event Log
	EventServiceStarting   = 1001
	EventServiceStarted    = 1002
	EventServiceStopping   = 1003
	EventServiceStopped    = 1004
	EventModeTransition    = 2001
	EventDisasterRecovery  = 2002
	EventServiceError      = 3001
	EventSecurityViolation = 3002
)

// NetMergeService implements svc.Handler for SCM execution.
type NetMergeService struct {
	pipeName string
	server   *ipc.Server
	engine   ipc.Engine
	coord    *mode.Coordinator
	elog     *eventlog.Log
}

// NewNetMergeService constructs a new service instance.
func NewNetMergeService(pipeName string, elog *eventlog.Log) *NetMergeService {
	coord := mode.NewCoordinator()
	eng := engineadapter.New(coord)
	server := ipc.NewServer(pipeName, eng)
	server.RegisterHandlers()

	return &NetMergeService{
		pipeName: pipeName,
		server:   server,
		engine:   eng,
		coord:    coord,
		elog:     elog,
	}
}

// LogInfo writes an informational event to the Windows Event Log if available.
func (s *NetMergeService) LogInfo(eid uint32, msg string) {
	log.Printf("[INFO] %s", msg)
	if s.elog != nil {
		_ = s.elog.Info(eid, msg)
	}
}

// LogWarn writes a warning event to the Windows Event Log if available.
func (s *NetMergeService) LogWarn(eid uint32, msg string) {
	log.Printf("[WARN] %s", msg)
	if s.elog != nil {
		_ = s.elog.Warning(eid, msg)
	}
}

// LogError writes an error event to the Windows Event Log if available.
func (s *NetMergeService) LogError(eid uint32, msg string) {
	log.Printf("[ERROR] %s", msg)
	if s.elog != nil {
		_ = s.elog.Error(eid, msg)
	}
}

// Execute is called by the Windows SCM dispatcher.
func (s *NetMergeService) Execute(args []string, r <-chan svc.ChangeRequest, changes chan<- svc.Status) (ssec bool, errno uint32) {
	const cmdsAccepted = svc.AcceptStop | svc.AcceptShutdown
	changes <- svc.Status{State: svc.StartPending}

	s.LogInfo(EventServiceStarting, fmt.Sprintf("%s is starting", ServiceDisplayName))

	// Run startup route & adapter disaster recovery checks before accepting clients
	if err := RunDisasterRecovery(); err != nil {
		s.LogWarn(EventDisasterRecovery, fmt.Sprintf("disaster recovery scan: %v", err))
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	errCh := make(chan error, 1)
	go func() {
		errCh <- s.server.Run(ctx)
	}()

	changes <- svc.Status{State: svc.Running, Accepts: cmdsAccepted}
	s.LogInfo(EventServiceStarted, fmt.Sprintf("%s started successfully on pipe %s", ServiceDisplayName, s.pipeName))

	for {
		select {
		case err := <-errCh:
			if err != nil && err != context.Canceled {
				s.LogError(EventServiceError, fmt.Sprintf("IPC server error: %v", err))
				changes <- svc.Status{State: svc.StopPending}
				s.cleanup()
				return false, 1
			}
			changes <- svc.Status{State: svc.StopPending}
			s.cleanup()
			return false, 0

		case req := <-r:
			switch req.Cmd {
			case svc.Interrogate:
				changes <- req.CurrentStatus

			case svc.Stop, svc.Shutdown:
				s.LogInfo(EventServiceStopping, fmt.Sprintf("%s received %v request, stopping", ServiceDisplayName, req.Cmd))
				changes <- svc.Status{State: svc.StopPending}

				cancel()
				_ = s.server.Close()
				s.cleanup()

				s.LogInfo(EventServiceStopped, fmt.Sprintf("%s stopped cleanly", ServiceDisplayName))
				return false, 0

			default:
				s.LogWarn(EventServiceError, fmt.Sprintf("unexpected SCM control request #%d", req.Cmd))
			}
		}
	}
}

// cleanup performs safe shutdown: stops the engine (removing capture routes and restoring weak-host)
// and closes mode handles.
func (s *NetMergeService) cleanup() {
	if s.engine != nil {
		if err := s.engine.Stop(); err != nil {
			s.LogWarn(EventServiceError, fmt.Sprintf("engine stop warning during shutdown: %v", err))
		}
	}
	if s.coord != nil {
		_ = s.coord.Close()
	}
}

// OpenEventLog opens or installs the Windows Event Log source for the service.
func OpenEventLog() (*eventlog.Log, error) {
	elog, err := eventlog.Open(ServiceName)
	if err != nil {
		// Attempt to register the event source if not present
		_ = eventlog.InstallAsEventCreate(ServiceName, eventlog.Error|eventlog.Warning|eventlog.Info)
		elog, err = eventlog.Open(ServiceName)
		if err != nil {
			return nil, fmt.Errorf("open event log: %w", err)
		}
	}
	return elog, nil
}

// RunService runs the service under SCM or in console.
func RunService(pipeName string, console bool) error {
	isWindowsService, err := svc.IsWindowsService()
	if err != nil {
		return fmt.Errorf("failed to detect service mode: %w", err)
	}

	elog, _ := OpenEventLog()
	if elog != nil {
		defer elog.Close()
	}

	if isWindowsService && !console {
		// Running under Windows Service Control Manager
		s := NewNetMergeService(pipeName, elog)
		return svc.Run(ServiceName, s)
	}

	// Running interactively in console / test mode
	log.Printf("%s running interactively (console mode)", ServiceDisplayName)
	s := NewNetMergeService(pipeName, elog)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Safe startup disaster recovery check
	if err := RunDisasterRecovery(); err != nil {
		s.LogWarn(EventDisasterRecovery, fmt.Sprintf("disaster recovery scan: %v", err))
	}

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	errCh := make(chan error, 1)
	go func() {
		errCh <- s.server.Run(ctx)
	}()

	log.Printf("net-merge service listening on %s (press Ctrl+C to terminate)", pipeName)

	select {
	case err := <-errCh:
		s.cleanup()
		return err
	case sig := <-sigCh:
		log.Printf("received signal %v, shutting down", sig)
		cancel()
		_ = s.server.Close()
		s.cleanup()
		return nil
	}
}

// SCM Management Functions --------------------------------------------------

// Install registers MultiNetNetMerge as an auto-starting Windows Service with recovery actions.
func Install() error {
	exePath, err := os.Executable()
	if err != nil {
		return fmt.Errorf("get executable path: %w", err)
	}
	exePath, err = filepath.Abs(exePath)
	if err != nil {
		return fmt.Errorf("abs executable path: %w", err)
	}

	m, err := mgr.Connect()
	if err != nil {
		return fmt.Errorf("connect to service control manager: %w", err)
	}
	defer m.Disconnect()

	s, err := m.OpenService(ServiceName)
	if err == nil {
		s.Close()
		return fmt.Errorf("service %s already exists", ServiceName)
	}

	cfg := mgr.Config{
		ServiceType:      windows.SERVICE_WIN32_OWN_PROCESS,
		StartType:        mgr.StartAutomatic,
		ErrorControl:     mgr.ErrorNormal,
		DisplayName:      ServiceDisplayName,
		Description:      ServiceDescription,
		DelayedAutoStart: true,
	}

	s, err = m.CreateService(ServiceName, exePath, cfg)
	if err != nil {
		return fmt.Errorf("create service %s: %w", ServiceName, err)
	}
	defer s.Close()

	// Install Windows Event Log source
	_ = eventlog.InstallAsEventCreate(ServiceName, eventlog.Error|eventlog.Warning|eventlog.Info)

	// Configure Service Recovery Actions: restart on failure (delay: 5000ms)
	recoveryActions := []mgr.RecoveryAction{
		{Type: mgr.ServiceRestart, Delay: 5 * time.Second},
		{Type: mgr.ServiceRestart, Delay: 10 * time.Second},
		{Type: mgr.NoAction, Delay: 0},
	}
	if err := s.SetRecoveryActions(recoveryActions, 86400); err != nil {
		log.Printf("warning: could not set recovery actions: %v", err)
	}

	log.Printf("Service %s installed successfully.", ServiceName)
	return nil
}

// Uninstall stops and unregisters the MultiNetNetMerge service.
func Uninstall() error {
	m, err := mgr.Connect()
	if err != nil {
		return fmt.Errorf("connect to service control manager: %w", err)
	}
	defer m.Disconnect()

	s, err := m.OpenService(ServiceName)
	if err != nil {
		return fmt.Errorf("open service %s: %w", ServiceName, err)
	}
	defer s.Close()

	// Stop service if running
	status, err := s.Query()
	if err == nil && status.State != svc.Stopped {
		_, _ = s.Control(svc.Stop)
		// Wait for stopped
		for i := 0; i < 20; i++ {
			time.Sleep(250 * time.Millisecond)
			status, err = s.Query()
			if err == nil && status.State == svc.Stopped {
				break
			}
		}
	}

	if err := s.Delete(); err != nil {
		return fmt.Errorf("delete service %s: %w", ServiceName, err)
	}

	_ = eventlog.Remove(ServiceName)
	log.Printf("Service %s uninstalled successfully.", ServiceName)
	return nil
}

// Start starts the registered MultiNetNetMerge service via SCM.
func Start() error {
	m, err := mgr.Connect()
	if err != nil {
		return fmt.Errorf("connect to service control manager: %w", err)
	}
	defer m.Disconnect()

	s, err := m.OpenService(ServiceName)
	if err != nil {
		return fmt.Errorf("open service %s: %w", ServiceName, err)
	}
	defer s.Close()

	if err := s.Start(); err != nil {
		return fmt.Errorf("start service %s: %w", ServiceName, err)
	}

	log.Printf("Service %s started successfully.", ServiceName)
	return nil
}

// Stop stops the running MultiNetNetMerge service via SCM.
func Stop() error {
	m, err := mgr.Connect()
	if err != nil {
		return fmt.Errorf("connect to service control manager: %w", err)
	}
	defer m.Disconnect()

	s, err := m.OpenService(ServiceName)
	if err != nil {
		return fmt.Errorf("open service %s: %w", ServiceName, err)
	}
	defer s.Close()

	status, err := s.Control(svc.Stop)
	if err != nil {
		return fmt.Errorf("stop service %s: %w", ServiceName, err)
	}

	for status.State != svc.Stopped {
		time.Sleep(300 * time.Millisecond)
		status, err = s.Query()
		if err != nil {
			return fmt.Errorf("query service status: %w", err)
		}
	}

	log.Printf("Service %s stopped successfully.", ServiceName)
	return nil
}

// Status queries and prints the current status of the service in SCM.
func Status() (string, error) {
	m, err := mgr.Connect()
	if err != nil {
		return "Unknown", fmt.Errorf("connect to service control manager: %w", err)
	}
	defer m.Disconnect()

	s, err := m.OpenService(ServiceName)
	if err != nil {
		return "NotInstalled", nil
	}
	defer s.Close()

	status, err := s.Query()
	if err != nil {
		return "Unknown", fmt.Errorf("query service %s: %w", ServiceName, err)
	}

	switch status.State {
	case svc.Stopped:
		return "Stopped", nil
	case svc.StartPending:
		return "StartPending", nil
	case svc.StopPending:
		return "StopPending", nil
	case svc.Running:
		return "Running", nil
	case svc.ContinuePending:
		return "ContinuePending", nil
	case svc.PausePending:
		return "PausePending", nil
	case svc.Paused:
		return "Paused", nil
	default:
		return fmt.Sprintf("Unknown(%d)", status.State), nil
	}
}
