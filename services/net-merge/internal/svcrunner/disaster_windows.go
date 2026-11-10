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
	"sync"
	"syscall"
)

var (
	weakHostStateMu   sync.Mutex
	weakHostStateFile = filepath.Join(os.TempDir(), "net-merge-service-weakhost.txt")
)

// SetWeakHostStateFileForTesting overrides the state file path for unit tests.
func SetWeakHostStateFileForTesting(p string) {
	weakHostStateMu.Lock()
	defer weakHostStateMu.Unlock()
	weakHostStateFile = p
}

// GetWeakHostStateFile returns the currently configured state file path.
func GetWeakHostStateFile() string {
	weakHostStateMu.Lock()
	defer weakHostStateMu.Unlock()
	return weakHostStateFile
}

// cmdExecutor defines the signature for running Windows management CLI commands.
type cmdExecutor func(name string, args ...string) ([]byte, error)

var defaultExecutor cmdExecutor = func(name string, args ...string) ([]byte, error) {
	cmd := exec.Command(name, args...)
	cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
	return cmd.CombinedOutput()
}

var (
	execMu      sync.Mutex
	execCommand = defaultExecutor
)

// SetCommandExecutorForTesting allows tests to mock command execution.
func SetCommandExecutorForTesting(fn cmdExecutor) {
	execMu.Lock()
	defer execMu.Unlock()
	if fn == nil {
		execCommand = defaultExecutor
	} else {
		execCommand = fn
	}
}

func runCmd(name string, args ...string) ([]byte, error) {
	execMu.Lock()
	fn := execCommand
	execMu.Unlock()
	return fn(name, args...)
}

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
		return fmt.Errorf("disaster recovery failed: %s", strings.Join(errs, "; "))
	}

	log.Println("[DISASTER-RECOVERY] Network safety scan complete. Host state healthy.")
	return nil
}

// restoreWeakHostCrashFile checks for the presence of the crash state file and restores
// original weakhostsend / forwarding flags on physical network interfaces.
func restoreWeakHostCrashFile() error {
	filePath := GetWeakHostStateFile()
	if _, err := os.Stat(filePath); os.IsNotExist(err) {
		return nil
	}

	f, err := os.Open(filePath)
	if err != nil {
		return fmt.Errorf("open weakhost file %s: %w", filePath, err)
	}

	scanner := bufio.NewScanner(f)
	var (
		failedLines   []string
		failureErrors []string
		restoredCount int
		lineNum       int
	)

	for scanner.Scan() {
		lineNum++
		rawLine := scanner.Text()
		line := strings.TrimSpace(rawLine)
		if line == "" {
			continue
		}

		parts := strings.Fields(line)
		if len(parts) < 4 {
			errMsg := fmt.Sprintf("line %d: malformed state record %q (expected: family index weakhostsend forwarding)", lineNum, line)
			log.Printf("[DISASTER-RECOVERY] Error: %s", errMsg)
			failedLines = append(failedLines, rawLine)
			failureErrors = append(failureErrors, errMsg)
			continue
		}

		// Format: family (e.g. IPv4) index weakhostsend forwarding
		family, idx, weakHost, forwarding := parts[0], parts[1], parts[2], parts[3]
		proto := "ipv4"
		if strings.EqualFold(family, "IPv6") {
			proto = "ipv6"
		}

		weakHostVal := strings.TrimPrefix(strings.ToLower(weakHost), "weakhostsend=")
		forwardingVal := strings.TrimPrefix(strings.ToLower(forwarding), "forwarding=")

		out, cmdErr := runCmd("netsh", "interface", proto, "set", "interface",
			"interface="+idx, "weakhostsend="+weakHostVal, "forwarding="+forwardingVal)
		if cmdErr != nil {
			outStr := strings.TrimSpace(string(out))
			errMsg := fmt.Sprintf("interface %s %s (weakhostsend=%s, forwarding=%s): %v (%s)", proto, idx, weakHostVal, forwardingVal, cmdErr, outStr)
			log.Printf("[DISASTER-RECOVERY] Error restoring %s", errMsg)
			failedLines = append(failedLines, rawLine)
			failureErrors = append(failureErrors, errMsg)
		} else {
			restoredCount++
		}
	}

	_ = f.Close()

	if scanErr := scanner.Err(); scanErr != nil {
		failureErrors = append(failureErrors, fmt.Sprintf("scanner error reading %s: %v", filePath, scanErr))
	}

	// Atomic state file management
	if len(failedLines) == 0 && len(failureErrors) == 0 {
		if err := os.Remove(filePath); err != nil {
			return fmt.Errorf("remove resolved weakhost file %s: %w", filePath, err)
		}
		if restoredCount > 0 {
			log.Printf("[DISASTER-RECOVERY] Successfully restored %d interface weak-host configurations from crash file.", restoredCount)
		}
		return nil
	}

	// Partial restoration: rewrite remaining failed entries atomically
	if len(failedLines) > 0 {
		tmpPath := filePath + fmt.Sprintf(".tmp-%d", os.Getpid())
		content := strings.Join(failedLines, "\n") + "\n"
		if writeErr := os.WriteFile(tmpPath, []byte(content), 0o600); writeErr == nil {
			_ = os.Rename(tmpPath, filePath)
		}
	}

	return fmt.Errorf("weak-host restoration failed for %d entry/entries (restored %d): %s", len(failedLines), restoredCount, strings.Join(failureErrors, "; "))
}

// cleanOrphanedCaptureRoutes checks if capture routes (0.0.0.0/1 and 128.0.0.0/1) exist
// pointing to an inactive or missing interface, and deletes them to restore internet access.
func cleanOrphanedCaptureRoutes() error {
	// Look up if our Wintun adapter is present and UP
	wintunIface, err := net.InterfaceByName("net-merge-service")
	if err == nil && wintunIface != nil {
		if wintunIface.Flags&net.FlagUp != 0 {
			// Wintun is currently active and up; do not delete routes
			return nil
		}
	}

	// Check existing routes via netsh
	routesOut, _ := runCmd("netsh", "interface", "ipv4", "show", "route")
	routesTable := string(routesOut)

	capturePrefixes := []struct {
		prefix string
		dest   string
		mask   string
	}{
		{"0.0.0.0/1", "0.0.0.0", "128.0.0.0"},
		{"128.0.0.0/1", "128.0.0.0", "128.0.0.0"},
	}

	var errors []string

	for _, cp := range capturePrefixes {
		// Only attempt deletion if the capture route is detected in routing table
		if strings.Contains(routesTable, cp.prefix) {
			log.Printf("[DISASTER-RECOVERY] Detected orphaned capture route %s; purging...", cp.prefix)
			out, cmdErr := runCmd("route", "delete", cp.dest, "mask", cp.mask)
			if cmdErr != nil {
				outStr := strings.TrimSpace(string(out))
				errors = append(errors, fmt.Sprintf("route delete %s: %v (%s)", cp.prefix, cmdErr, outStr))
			}
		}
	}

	// Verify routes are actually absent after deletion
	postCheckOut, _ := runCmd("netsh", "interface", "ipv4", "show", "route")
	postTable := string(postCheckOut)
	for _, cp := range capturePrefixes {
		if strings.Contains(postTable, cp.prefix) {
			errors = append(errors, fmt.Sprintf("orphaned capture route %s still present after deletion attempt", cp.prefix))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("route cleanup failed: %s", strings.Join(errors, "; "))
	}

	return nil
}
