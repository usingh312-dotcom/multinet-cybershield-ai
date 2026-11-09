# NET MERGE INTEGRATION — NM-06.3.3 FAULT INJECTION REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Windows Systems Architect, QA Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No staging, commit, reset, or clean. All existing modifications preserved.

---

## 1. Executive Summary

This fault injection report documents the fault tolerance design and test scenarios for the Net Merge integration. All fault injection tests are **source-designed but not execution-verified** (Go not installed, SAC blocks Rust test binaries).

The fault injection testing covers:

| Fault Category | Scenarios | Status |
|---|---|---|
| Service lifecycle | Startup failure, shutdown, crash recovery | Source designed |
| Network adapter | Disconnect, reconnect, disappearance | Source designed |
| IPC | Client disconnect, malformed requests, version mismatch | Source designed |
| Database | Temporary unavailability | Source designed |
| Speed test | Timeout, cancellation | Source designed |
| Mutex | Abandoned ownership, concurrent access | Source designed |
| Mode switching | Concurrent requests, invalid transitions | Source designed |

---

## 2. Service Lifecycle Fault Injection

### Test: Service startup failure

| Scenario | Expected Behavior |
|---|---|
| Wintun DLL missing | `platform.CheckDriver()` returns error; service fails to start with clear error message |
| No usable adapters | `DiscoverNicsDetailed()` returns empty; service fails with "no usable IPv4 adapters found" |
| TUN creation fails | `tun.On()` returns error; `engine.Start()` rolls back and releases mutex |
| Route installation fails | `tun.On()` returns error; `engine.Start()` rolls back (calls `t.Close()` which calls `delRoutes()`) |
| Mutex already held (Mode B active) | `RequestModeA()` returns error; service refuses to start |

### Test: Normal shutdown

| Step | Action | Expected Result |
|---|---|---|
| 1 | Send `stop` IPC command | `engine.Stop()` called |
| 2 | `tun.Tun.Close()` called | Capture routes removed (`delRoutes()`) |
| 3 | Weak-host settings restored | `stopStrongHost()` called |
| 4 | Wintun adapter closed | `dev.Close()` called |
| 5 | gVisor stack closed | `stk.Close()` + `stk.Wait()` |
| 6 | Kernel mutex released | `coord.ReleaseMode()` called |
| 7 | Response sent | `{status:"stopped"}` |

### Test: Process crash (unexpected termination)

| Scenario | Recovery Mechanism |
|---|---|
| Process crash during Mode A | Kernel auto-releases named mutex; `CleanupStale()` removes capture routes on restart; `restoreStrongHost()` restores weak-host settings |
| Process crash during Mode B | Kernel auto-releases named mutex; `CleanupStale()` removes capture routes; Rust process handle becomes invalid |
| Process crash during IPC request | Pipe disconnect detected; `scanner.Err()` handled; connection closed gracefully |

---

## 8. Fault Injection Test Matrix

| # | Fault | Injection Method | Expected Behavior | Verify |
|---|---|---|---|---|
| F1 | Wintun DLL missing | Remove/rename `wintun.dll` | Service fails to start with clear error | Source designed |
| F2 | No usable adapters | Disable all physical adapters | Service fails: "no usable IPv4 adapters found" | Source designed |
| F3 | TUN creation failure | Block netsh access | `tun.On()` returns error; rollback | Source designed |
| F4 | Route installation failure | Block route add | `tun.On()` returns error; rollback | Source designed |
| F5 | Adapter disconnect during Mode A | Disable physical adapter | Probe marks path down; scheduler skips it; traffic continues on other adapters | Source designed |
| F6 | Adapter reconnection | Re-enable adapter | Probe marks path up; scheduler includes it again | Source designed |
| F7 | IPC client disconnect | Close client pipe | `scanner.Err()` detected; connection closed gracefully | Source designed |
| F8 | Malformed IPC request | Send invalid JSON | `invalid_json` error response | Source designed |
| F9 | Unsupported pipe version | Send `pipe_version=999` | `unsupported_version` error | Source designed |
| F10 | Non-admin calls admin method | Send `start` as standard user | `unauthorized` error | Source designed |
| F11 | Database unavailable | Make SQLite file read-only | Speed test results cannot be persisted; warning logged | Source designed |
| F12 | Speed test timeout | Slow/unresponsive server | Request times out after `timeout_secs` | Source designed |
| F13 | Speed test cancellation | Cancel mid-test | `SpeedTestError::Cancelled`; outcome set to Cancelled | Source designed |
| F14 | Mutex abandoned (crash) | Kill service process | Kernel releases mutex; new instance can acquire | Source designed |
| F15 | Concurrent Mode A + Mode B | Start both simultaneously | Mutex serializes; second rejected | Source designed |
| F16 | Pipe disconnect during transfer | Close client pipe | `scanner.Err()` detected; connection closed | Source designed |

---

## 10. Fault Injection Test Code (not yet written)

The following tests should be added to `crates/netmerge-client/src/lib.rs` and `services/net-merge/internal/ipc/pipesec_test.go`:

```rust
// Rust: Test IPC fault injection
#[test]
fn set_mode_b_with_invalid_pid() {
    // Client sends invalid PID; service should reject
    // (Requires running Go service - integration test)
}

#[test]
fn pipe_disconnect_during_transfer() {
    // Close pipe mid-request; client should get PipeDisconnected error
    // (Requires running Go service - integration test)
}

#[test]
fn version_mismatch_rejected() {
    // Client sends pipe_version=999; service rejects
    // (Requires running Go service - integration test)
}
```

```go
// Go: Test fault injection
func TestServiceStartupWithoutWintun(t *testing.T) {
    // Remove wintun.dll; service should fail to start
    // Requires admin privileges; integration test
}

func TestAdapterDisconnectDuringModeA(t *testing.T) {
    // Disable adapter; probe should mark it down; traffic continues on others
    // Requires admin privileges; integration test
}

func TestCrashRecovery(t *testing.T) {
    // Kill service process; new instance should acquire mutex and clean up
    // Requires admin privileges; integration test
}
```

---

## 11. Fault Injection Test Priorities

| Priority | Test | Reason |
|---|---|---|
| P0 | Service crash recovery | Most critical for production reliability |
| P0 | Adapter disconnect during Mode A | Core bonding functionality |
| P0 | IPC client disconnect | Common failure mode |
| P1 | Version mismatch rejection | Protocol compatibility |
| P1 | Non-admin access to admin methods | Security boundary |
| P2 | Speed test timeout | Edge case |
| P2 | Database unavailable | Graceful degradation |
| P3 | Mutex abandoned | Rare but critical for recovery |

---

## 12. Attestation

This fault injection report describes test scenarios designed but not executed. Go is not installed (cannot compile/test Go service), and Smart App Control blocks Rust test binary execution (error 4551). All fault injection scenarios are source-designed based on code inspection, not execution-tested. No results are fabricated.
