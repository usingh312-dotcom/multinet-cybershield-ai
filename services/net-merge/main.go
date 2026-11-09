// main is the entry point for the Net Merge service executable
// (net-merge-service.exe). It is an independent, elevated Go process that owns
// the Wintun adapter and capture routes (Mode A). MultiNet (Tauri, unelevated)
// controls it exclusively over the versioned named-pipe IPC.
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
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"multinet.ai/netmerge-service/internal/engineadapter"
	"multinet.ai/netmerge-service/internal/ipc"
	"multinet.ai/netmerge-service/internal/mode"
)

var (
	pipeName   = flag.String("pipe", ipc.DefaultPipeName, "named-pipe endpoint")
	stateDir   = flag.String("state-dir", "", "service state directory")
	logFile    = flag.String("log", "", "log file (default: stderr)")
	mtu        = flag.Uint("mtu", 9000, "TUN MTU")
	ipv6       = flag.String("ipv6", "auto", "IPv6 mode: auto/on/off")
	acquireSec = flag.Uint("acquire-timeout", 5, "seconds to wait for mode mutex before failing")
)

func main() {
	flag.Parse()

	if *logFile != "" {
		f, err := os.OpenFile(*logFile, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0o644)
		if err == nil {
			log.SetOutput(f)
		}
	}

	log.Println("net-merge service starting")

	// The Coordinator is the authoritative Mode A / Mode B coordinator. It owns
	// the kernel named mutex for the entire lifetime of the active mode.
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
	log.Println("net-merge service stopped")
}

// tryAcquireWithTimeout attempts to acquire the mode mutex, retrying every
// 200ms until timeout. Returns true if acquired.
//
// Deprecated: mode acquisition is now handled inside engineadapter.Start()
// via coord.RequestModeA(). This helper is retained for callers that need
// to pre-acquire the mutex before starting the server.
func tryAcquireWithTimeout(coord *mode.Coordinator, timeout time.Duration) (bool, error) {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if err := coord.RequestModeA(); err != nil {
			// Mode B is active or mutex is held; retry.
			time.Sleep(200 * time.Millisecond)
			continue
		}
		return true, nil
	}
	return false, nil
}
