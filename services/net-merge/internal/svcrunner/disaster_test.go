package svcrunner_test

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"multinet.ai/netmerge-service/internal/svcrunner"
)

func setupTestRecoveryFile(t *testing.T, content string) string {
	t.Helper()
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "test-weakhost.txt")
	if content != "" {
		if err := os.WriteFile(filePath, []byte(content), 0o600); err != nil {
			t.Fatalf("failed to write test recovery file: %v", err)
		}
	}
	svcrunner.SetWeakHostStateFileForTesting(filePath)
	t.Cleanup(func() {
		svcrunner.SetWeakHostStateFileForTesting(filepath.Join(os.TempDir(), "net-merge-service-weakhost.txt"))
		svcrunner.SetCommandExecutorForTesting(nil)
	})
	return filePath
}

// 1. Complete success: All state entries are restored and file is cleanly removed
func TestDisasterRecoveryCompleteSuccess(t *testing.T) {
	stateContent := "IPv4 10 enabled disabled\nIPv4 11 enabled disabled\n"
	statePath := setupTestRecoveryFile(t, stateContent)

	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		if name == "netsh" && len(args) > 4 && args[0] == "interface" && args[3] == "set" {
			return []byte("Ok.\n"), nil
		}
		if name == "netsh" && len(args) > 3 && args[3] == "route" {
			return []byte("No capture routes present\n"), nil
		}
		return []byte(""), nil
	})

	err := svcrunner.RunDisasterRecovery()
	if err != nil {
		t.Fatalf("expected RunDisasterRecovery to succeed, got: %v", err)
	}

	if _, err := os.Stat(statePath); !os.IsNotExist(err) {
		t.Fatalf("expected state file %s to be removed on complete success", statePath)
	}
}

// 2. Partial success: Succeeded entries are pruned; failed entries are atomically retained
func TestDisasterRecoveryPartialSuccess(t *testing.T) {
	stateContent := "IPv4 10 enabled disabled\nIPv4 20 enabled disabled\n"
	statePath := setupTestRecoveryFile(t, stateContent)

	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		if name == "netsh" && len(args) > 4 && args[0] == "interface" && args[3] == "set" {
			for _, a := range args {
				if a == "interface=10" {
					return []byte("Ok.\n"), nil
				}
				if a == "interface=20" {
					return []byte("The parameter is incorrect.\n"), errors.New("exit status 1")
				}
			}
		}
		if name == "netsh" && len(args) > 3 && args[3] == "route" {
			return []byte(""), nil
		}
		return []byte(""), nil
	})

	err := svcrunner.RunDisasterRecovery()
	if err == nil {
		t.Fatal("expected error on partial restoration failure, got nil")
	}

	// State file must still exist
	data, readErr := os.ReadFile(statePath)
	if readErr != nil {
		t.Fatalf("expected state file to remain for failed entries: %v", readErr)
	}

	retained := string(data)
	if strings.Contains(retained, "interface=10") || strings.Contains(retained, "10 enabled") {
		t.Errorf("successful entry (interface 10) was not pruned from state file: %q", retained)
	}
	if !strings.Contains(retained, "20") {
		t.Errorf("failed entry (interface 20) was not retained in state file: %q", retained)
	}
}

// 3. Malformed state records: Malformed lines are reported and retained
func TestDisasterRecoveryMalformedStateRecord(t *testing.T) {
	stateContent := "malformed_token_without_enough_fields\nIPv4 10 enabled disabled\n"
	statePath := setupTestRecoveryFile(t, stateContent)

	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		return []byte("Ok.\n"), nil
	})

	err := svcrunner.RunDisasterRecovery()
	if err == nil {
		t.Fatal("expected error due to malformed state record, got nil")
	}

	if !strings.Contains(err.Error(), "malformed state record") {
		t.Errorf("error did not identify malformed state record: %v", err)
	}

	data, readErr := os.ReadFile(statePath)
	if readErr != nil {
		t.Fatalf("expected state file to remain: %v", readErr)
	}

	if !strings.Contains(string(data), "malformed_token_without_enough_fields") {
		t.Errorf("malformed entry was not retained: %q", string(data))
	}
}

// 4. netsh failure: Command failure propagates diagnostic details and preserves file
func TestDisasterRecoveryNetshFailure(t *testing.T) {
	stateContent := "IPv4 99 enabled disabled\n"
	statePath := setupTestRecoveryFile(t, stateContent)

	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		if name == "netsh" && len(args) > 4 && args[3] == "set" {
			return []byte("Element not found.\n"), errors.New("exit status 1")
		}
		return []byte(""), nil
	})

	err := svcrunner.RunDisasterRecovery()
	if err == nil {
		t.Fatal("expected error on netsh failure, got nil")
	}

	if !strings.Contains(err.Error(), "Element not found") && !strings.Contains(err.Error(), "99") {
		t.Errorf("expected error to preserve diagnostic details (interface and reason), got: %v", err)
	}

	if _, err := os.Stat(statePath); os.IsNotExist(err) {
		t.Fatal("state file was deleted despite netsh failure")
	}
}

// 5. Route deletion failure: Orphaned route cleanup fails and reports error
func TestDisasterRecoveryRouteDeletionFailure(t *testing.T) {
	setupTestRecoveryFile(t, "") // No weakhost file

	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		if name == "netsh" && len(args) > 3 && args[3] == "route" {
			// Report that capture route 0.0.0.0/1 is in routing table
			return []byte("0.0.0.0/1   128.0.0.0   192.168.1.1\n"), nil
		}
		if name == "route" && len(args) > 1 && args[0] == "delete" {
			return []byte("The requested operation requires elevation.\n"), errors.New("exit status 1")
		}
		return []byte(""), nil
	})

	err := svcrunner.RunDisasterRecovery()
	if err == nil {
		t.Fatal("expected error on route deletion failure, got nil")
	}

	if !strings.Contains(err.Error(), "route cleanup failed") {
		t.Errorf("expected route cleanup error, got: %v", err)
	}
}

// 6. Successful retry: After initial partial failure, a subsequent recovery call succeeds
func TestDisasterRecoverySuccessfulRetry(t *testing.T) {
	stateContent := "IPv4 10 enabled disabled\nIPv4 20 enabled disabled\n"
	statePath := setupTestRecoveryFile(t, stateContent)

	// Step 1: Interface 20 fails
	step := 1
	svcrunner.SetCommandExecutorForTesting(func(name string, args ...string) ([]byte, error) {
		if name == "netsh" && len(args) > 4 && args[3] == "set" {
			for _, a := range args {
				if a == "interface=10" {
					return []byte("Ok.\n"), nil
				}
				if a == "interface=20" {
					if step == 1 {
						return []byte("Device unavailable\n"), errors.New("exit status 1")
					}
					// Step 2: Retry succeeds
					return []byte("Ok.\n"), nil
				}
			}
		}
		return []byte(""), nil
	})

	err1 := svcrunner.RunDisasterRecovery()
	if err1 == nil {
		t.Fatal("expected step 1 to fail")
	}

	// Verify state file contains only interface 20
	data1, _ := os.ReadFile(statePath)
	if !strings.Contains(string(data1), "20") || strings.Contains(string(data1), "10") {
		t.Fatalf("unexpected state file content after step 1: %q", string(data1))
	}

	// Step 2: Retry recovery
	step = 2
	err2 := svcrunner.RunDisasterRecovery()
	if err2 != nil {
		t.Fatalf("expected step 2 retry to succeed, got: %v", err2)
	}

	// State file should now be completely deleted
	if _, err := os.Stat(statePath); !os.IsNotExist(err) {
		t.Fatalf("expected state file to be deleted after successful retry")
	}
}
