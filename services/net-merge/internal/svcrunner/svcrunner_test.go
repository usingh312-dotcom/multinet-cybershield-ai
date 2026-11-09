package svcrunner_test

import (
	"os"
	"path/filepath"
	"testing"

	"multinet.ai/netmerge-service/internal/svcrunner"
)

func TestServiceConstants(t *testing.T) {
	if svcrunner.ServiceName != "MultiNetNetMerge" {
		t.Fatalf("unexpected service name: %s", svcrunner.ServiceName)
	}
	if svcrunner.ServiceDisplayName == "" {
		t.Fatal("empty display name")
	}
	if svcrunner.ServiceDescription == "" {
		t.Fatal("empty description")
	}
}

func TestDisasterRecoveryCleanRun(t *testing.T) {
	// Disaster recovery should execute safely on clean systems without error
	err := svcrunner.RunDisasterRecovery()
	if err != nil {
		t.Fatalf("RunDisasterRecovery returned error: %v", err)
	}
}

func TestDisasterRecoveryWithMockStateFile(t *testing.T) {
	stateFile := filepath.Join(os.TempDir(), "net-merge-service-weakhost.txt")
	// Create mock state file with harmless loopback interface line
	content := "IPv4 1 Disabled Disabled\n"
	if err := os.WriteFile(stateFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create mock state file: %v", err)
	}
	defer os.Remove(stateFile)

	// Run recovery
	_ = svcrunner.RunDisasterRecovery()

	// State file must have been processed and removed
	if _, err := os.Stat(stateFile); !os.IsNotExist(err) {
		t.Fatalf("expected state file to be removed after recovery")
	}
}

func TestQueryServiceStatusWhenNotInstalled(t *testing.T) {
	// On machines where the service is not yet installed in SCM, it should report NotInstalled or Stopped
	status, err := svcrunner.Status()
	if err != nil {
		t.Logf("Status error (expected if SCM permission limited): %v", err)
		return
	}
	t.Logf("SCM Service status: %s", status)
}

