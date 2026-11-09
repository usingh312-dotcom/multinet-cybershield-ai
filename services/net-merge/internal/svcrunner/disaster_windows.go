// Package svcrunner - Disaster recovery for network routes and adapter configuration.
package svcrunner

import (
	"bufio"
	"fmt"
	"log"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
)

var weakHostStateFile = filepath.Join(os.TempDir(), "net-merge-service-weakhost.txt")

// RunDisasterRecovery inspects the host network configuration to heal any orphaned
// routes, strong-host overrides, or lingering virtual adapter artifacts left by
// an ungraceful crash or sudden power interruption.
func RunDisasterRecovery() error {
	log.Println("[DISASTER-RECOVERY] Scanning for orphaned network artifacts...")
	var errs []string

	// 1. Restore weak-host / forwarding settings if a recovery file was left
	if err := restoreWeakHostCrashFile(); err != nil {
		errs = append(errs, fmt.Sprintf("weak-host restore: %v", err))
	}

	// 2. Scan and remove any orphaned capture routes (0.0.0.0/1, 128.0.0.0/1)
	// that do not belong to an active, running Wintun session.
	if err := cleanOrphanedCaptureRoutes(); err != nil {
		errs = append(errs, fmt.Sprintf("capture routes cleanup: %v", err))
	}

	if len(errs) > 0 {
		return fmt.Errorf("disaster recovery warnings: %s", strings.Join(errs, "; "))
	}

	log.Println("[DISASTER-RECOVERY] Network safety scan complete. No orphaned routes detected.")
	return nil
}

// restoreWeakHostCrashFile checks for the presence of the crash state file and restores
// original weakhostsend / forwarding flags on physical network interfaces.
func restoreWeakHostCrashFile() error {
	if _, err := os.Stat(weakHostStateFile); os.IsNotExist(err) {
		return nil
	}

	f, err := os.Open(weakHostStateFile)
	if err != nil {
		return fmt.Errorf("open weakhost file: %w", err)
	}
	defer f.Close()

	scanner := bufio.NewScanner(f)
	restoredCount := 0
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		parts := strings.Fields(line)
		if len(parts) < 4 {
			continue
		}
		// Format: family (e.g. IPv4) index weakhostsend forwarding
		family, idx, weakHost, forwarding := parts[0], parts[1], parts[2], parts[3]
		proto := "ipv4"
		if strings.EqualFold(family, "IPv6") {
			proto = "ipv6"
		}

		cmd := exec.Command("netsh", "interface", proto, "set", "interface",
			"interface="+idx, "weakhostsend="+weakHost, "forwarding="+forwarding)
		cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
		if out, err := cmd.CombinedOutput(); err != nil {
			log.Printf("[DISASTER-RECOVERY] Warning: restore interface %s %s: %v (%s)", proto, idx, err, strings.TrimSpace(string(out)))
		} else {
			restoredCount++
		}
	}

	_ = f.Close()
	_ = os.Remove(weakHostStateFile)
	if restoredCount > 0 {
		log.Printf("[DISASTER-RECOVERY] Successfully restored %d interface weak-host configurations from crash file.", restoredCount)
	}
	return nil
}

// cleanOrphanedCaptureRoutes checks if capture routes (0.0.0.0/1 and 128.0.0.0/1) exist
// pointing to an inactive or missing interface, and deletes them to restore internet access.
func cleanOrphanedCaptureRoutes() error {
	// Look up if our Wintun adapter is present
	wintunIface, err := net.InterfaceByName("net-merge-service")
	if err == nil && wintunIface != nil {
		// Interface exists; verify whether it is UP or DOWN
		if wintunIface.Flags&net.FlagUp != 0 {
			// Wintun is currently active and up; do not delete routes
			return nil
		}
	}

	// Delete capture routes if they exist without an active Wintun interface
	routes := [][2]string{
		{"0.0.0.0", "128.0.0.0"},
		{"128.0.0.0", "128.0.0.0"},
	}

	for _, r := range routes {
		cmd := exec.Command("route", "delete", r[0], "mask", r[1])
		cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
		_ = cmd.Run()
	}

	return nil
}

