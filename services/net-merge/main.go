// main is the entry point for the Net Merge service executable
// (net-merge-service.exe). It is an elevated Go process that owns
// the Wintun adapter and capture routes (Mode A). MultiNet (Tauri, unelevated)
// controls it exclusively over the versioned named-pipe IPC.
//
// Dual-mode execution:
//   - When invoked by the Windows Service Control Manager (services.exe), it
//     automatically registers via svc.Run as Windows Service "MultiNetNetMerge".
//   - When invoked interactively or with -console, it runs in the current console.
//   - When invoked with -action (install/uninstall/start/stop/status), it manages
//     the service configuration in the Windows SCM.
//
// Cross-process mode ownership: a Windows named mutex
// (Global\multinet-netmerge-mode-owner-v1) ensures only one process owns the
// networking tunnel at a time. Mode A (this service) and Mode B (Rust bonding
// engine) are mutually exclusive. The elevated Go service is the authoritative
// coordinator: it holds the kernel mutex for the entire lifetime of the active
// mode. The unelevated Rust bonding engine requests Mode B through authenticated
// IPC; it never touches the kernel mutex directly. The mutex is kernel-released
// on process crash, enabling safe recovery.
//
// Build: go build -o net-merge-service.exe .
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"golang.org/x/sys/windows/svc"

	"multinet.ai/netmerge-service/internal/engineadapter"
	"multinet.ai/netmerge-service/internal/ipc"
	"multinet.ai/netmerge-service/internal/mode"
	"multinet.ai/netmerge-service/internal/svcrunner"
)

var (
	pipeName   = flag.String("pipe", ipc.DefaultPipeName, "named-pipe endpoint")
	stateDir   = flag.String("state-dir", "", "service state directory")
	logFile    = flag.String("log", "", "log file (default: stderr)")
	mtu        = flag.Uint("mtu", 9000, "TUN MTU")
	ipv6       = flag.String("ipv6", "auto", "IPv6 mode: auto/on/off")
	acquireSec = flag.Uint("acquire-timeout", 5, "seconds to wait for mode mutex before failing")
	console    = flag.Bool("console", false, "run interactively in console instead of Windows Service")
	action     = flag.String("action", "", "service action: install, uninstall, start, stop, status, recover")
)

var versionFlag = flag.Bool("version", false, "display version and build information")

const AppVersion = "0.1.0-beta.1"

func main() {
	flag.Parse()

	if *versionFlag {
		fmt.Printf("MultiNet Net-Merge Service v%s (Milestone NM-07.0)\n", AppVersion)
		return
	}

	if *logFile != "" {
		f, err := os.OpenFile(*logFile, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
		if err == nil {
			log.SetOutput(f)
		}
	}

	// Handle SCM management actions
	if *action != "" {
		var err error
		switch *action {
		case "install":
			err = svcrunner.Install()
		case "uninstall":
			err = svcrunner.Uninstall()
		case "start":
			err = svcrunner.Start()
		case "stop":
			err = svcrunner.Stop()
		case "status":
			var s string
			s, err = svcrunner.Status()
			if err == nil {
				fmt.Printf("Service %s status: %s\n", svcrunner.ServiceName, s)
			}
		case "recover":
			err = svcrunner.RunDisasterRecovery()
			if err == nil {
				fmt.Println("Disaster recovery completed successfully.")
			}
		default:
			fmt.Fprintf(os.Stderr, "Unknown action: %s (supported: install, uninstall, start, stop, status, recover)\n", *action)
			os.Exit(1)
		}
		if err != nil {
			fmt.Fprintf(os.Stderr, "Action %s failed: %v\n", *action, err)
			os.Exit(1)
		}
		return
	}

	// Detect whether running under Windows Service Control Manager
	isService, err := svc.IsWindowsService()
	if err == nil && isService && !*console {
		if err := svcrunner.RunService(*pipeName, false); err != nil {
			log.Fatalf("service execution failed: %v", err)
		}
		return
	}

	// Interactive / Console mode
	log.Println("net-merge service starting (console mode)")

	// Run disaster recovery scan on startup
	if err := svcrunner.RunDisasterRecovery(); err != nil {
		log.Printf("disaster recovery note: %v", err)
	}

	coord := mode.NewCoordinator()
	eng := engineadapter.New(coord)
	server := ipc.NewServer(*pipeName, eng)
	server.RegisterHandlers()

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	go func() {
		<-ctx.Done()
		_ = server.Close()
	}()

	if err := server.Run(ctx); err != nil && err != context.Canceled {
		log.Fatalf("ipc server: %v", err)
	}

	// Safe shutdown: stop the engine (removes capture routes, restores weak-host)
	// and releases the mode mutex.
	if err := eng.Stop(); err != nil {
		log.Printf("engine shutdown warning: %v", err)
	}
	_ = coord.Close()
	log.Println("net-merge service stopped")
}
