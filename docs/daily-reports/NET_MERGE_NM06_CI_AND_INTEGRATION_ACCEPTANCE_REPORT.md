# NET MERGE INTEGRATION — NM-06 CI AND INTEGRATION ACCEPTANCE REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Windows Systems Architect, Senior Go Engineer, Rust Systems Engineer, Windows Security Engineer, QA Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved. Rust bonding-engine code untouched. Database migrations preserved. NM-01 through NM-05 reports preserved.

---

## STATUS SUMMARY

| Workstream | Status | Evidence |
|---|---|---|
| Rust IPC client (`netmerge-client`) | **COMPILED** | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client unit tests | **SOURCE IMPLEMENTED** | Tests written; execution blocked by SAC error 4551 |
| Go IPC service + secured named pipe | **SOURCE IMPLEMENTED** | **NOT COMPILED** — Go not installed on this host |
| Go unit tests | **SOURCE IMPLEMENTED** | Tests written; cannot compile/run (Go not installed) |
| CI workflow | **SOURCE IMPLEMENTED** | `.github/workflows/windows-build.yml` created; not executed (no CI runner) |
| Named-pipe security (CreateNamedPipe + PIPE_REJECT_REMOTE_CLIENTS + SDDL + client validation + per-method auth) | **SOURCE IMPLEMENTED** | Restrictive DACL, impersonation, CheckTokenMembership, Authorizer policy |
| Cross-process mode ownership (service-held mutex + thread affinity + WAIT_ABANDONED + process tracking) | **SOURCE IMPLEMENTED** | `mode.Coordinator` + `modeowner.Owner` |
| Mode A / Mode B coordination protocol | **SOURCE IMPLEMENTED** | Coordinator owns kernel mutex; Rust requests via IPC |
| Rust-to-Go named-pipe round trip | **NOT EXECUTED** | Neither side runnable on this host |
| Windows lifecycle testing | **NOT EXECUTED** | Go not installed; SAC blocks test binaries |
| Hardware verification | **NOT EXECUTED** | No three-interface test performed |
| Code signing / SAC runtime | **NOT EXECUTED** | No signing identity |

**NM-06 is not accepted.** Per the acceptance criteria, NM-06 requires the Go service to build, Go and Rust tests to execute, named-pipe authorization tested, remote pipe connections rejected, mode ownership enforced across processes, and startup/shutdown/recovery tested on Windows. None of these can be confirmed on this host because Go is not installed and SAC blocks Rust test-binary execution. No results are fabricated.

---

## 1. Baseline Audit

Inspected NM-05 report and all existing Go/Rust source. Git status recorded before and after work. No files staged, committed, reset, or cleaned. `D:\git\browser` never touched.

Baseline state (from NM-05):
- Rust `netmerge-client` crate compiles (`cargo check` EXIT 0)
- Go service source written but uncompilable (Go not installed)
- Pipe ACL was `D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)` (broad Users group)
- Mode ownership used direct `modeowner.Owner` by both engine and service (no service-mediated coordination)
- No CI workflow existed

---

## 2. Windows Go Build Environment

### Environment recorded on this host

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 Home Single Language |
| OS Version | 10.0.26200 Build 26200 |
| Architecture | x64-based PC |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Cargo | cargo 1.99.1 (5f94df478 2026-08-27) |
| Go | **NOT INSTALLED** |
| Go version required by `services/net-merge/go.mod` | go 1.27 |
| Smart App Control | ENABLED (error 4551 blocking test binaries) |

### Build-environment limitations

1. **Go is not installed.** The `services/net-merge/` Go module cannot be compiled or tested. Move to an approved Windows build/test environment with Go 1.27.
2. **Smart App Control blocks test-binary execution.** `cargo test` binaries compile but cannot run (os error 4551).
3. **No code-signing identity.** Unsigned Go binaries will be blocked by SAC at runtime.
4. **External Go dependencies not resolved.** gVisor, tun2socks, Wintun bindings must be resolved via `go mod download`.

### CI workflow created

Created `.github/workflows/windows-build.yml` with:
- Go ${{ env.GO_VERSION }} (1.27) setup
- Rust stable toolchain setup
- `go mod download`, `go mod verify`
- `go fmt ./...`, `go vet ./...`
- `go build -o net-merge-service.exe .`
- `go test ./...`
- `cargo check --workspace`
- `cargo test --workspace`
- Artifact upload of Go binary

**Classification:** CI evidence (when executed on a runner), NOT SAC runtime verification.

---

## 3. Commands (Exact)

### Go (not executed — Go not installed)

```powershell
cd D:\git\multinet-cybershield-ai\services\net-merge
go mod download
go mod verify
go fmt ./...
go vet ./...
go build -o net-merge-service.exe .
go test ./... -v
```

### Rust (executed on this host)

```powershell
cargo check -p netmerge-client      # EXIT 0
cargo check -p bonding-engine       # EXIT 0
cargo test -p netmerge-client       # COMPILES, execution blocked by SAC 4551
cargo test -p bonding-engine        # COMPILES, execution blocked by SAC 4551
cargo check --workspace             # BLOCKED: rustls build script (SAC 4551)
```

---

## 4. Token Identity and Authorization Audit

### Changes to `pipesec_windows.go`

**Before (NM-05):**
- `GetTokenUser()` used to get the user SID, compared directly against well-known SIDs
- UAC-filtered admin tokens: `GetTokenUser()` returns the standard-user SID, so admins running under UAC would NOT be recognized as administrators
- No group-membership verification

**After (NM-06):**
- `CheckTokenMembership` (via `advapi32!CheckTokenMembership`) used to check token group membership
- UAC-filtered tokens correctly handled: admin SID is present in the token's group list even when `GetTokenUser()` returns the standard-user SID
- `TokenGroups` enumeration not needed; `CheckTokenMembership` is the correct API
- Token handle always closed via `defer windows.CloseHandle(token)`
- Impersonation always reverted via `defer windows.RevertToSelf()` on all error paths

### Authorizer policy (per-method)

| Method | Requires Admin | Standard User |
|---|---|---|
| `health` | No | Yes |
| `status` | No | Yes |
| `adapters` | No | Yes |
| `start` | Yes | No |
| `stop` | Yes | No |
| `set_limit` | Yes | No |
| `set_mode_b` | Yes | No |
| `mode_b_status` | No | Yes |

### Test coverage (Go, not executed)

| Test | File | Coverage |
|---|---|---|
| `TestPipeSDDLIsRestrictive` | `pipesec_test.go` | SDDL excludes Everyone/Anonymous |
| `TestStringToSecurityDescriptor` | `pipesec_test.go` | SDDL converts to valid security descriptor |
| `TestAuthorizerAdminAllowed` | `pipesec_test.go` | Admin can call privileged methods |
| `TestAuthorizerSystemAllowed` | `pipesec_test.go` | LocalSystem can call privileged methods |
| `TestAuthorizerUserAllowedForReadOnly` | `pipesec_test.go` | Standard user can call health/status/adapters |
| `TestAuthorizerUserDeniedForAdminMethods` | `pipesec_test.go` | Standard user denied for start/stop/set_limit/set_mode_b |
| `TestAuthorizerUnknownIdentityDenied` | `pipesec_test.go` | Unknown identity denied for all methods |

---

## 5. Named-Pipe Local-Only Enforcement

### `PIPE_REJECT_REMOTE_CLIENTS`

Added `PIPE_REJECT_REMOTE_CLIENTS` (0x00000008) to `CreateNamedPipeW` flags in `securePipe()`. This ensures remote (SMB) clients cannot connect to the local control interface. Only local processes can open the pipe.

### Pipe DACL

```
D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)(A;;GRGW;;;S-1-5-18)
```

| ACE | Principal | Access |
|---|---|---|
| `(A;;GA;;;BA)` | Built-in Administrators | Full Control |
| `(A;;GRGW;;;S-1-5-32-545)` | Built-in Users | Read/Write |
| `(A;;GRGW;;;S-1-5-18)` | LocalSystem | Read/Write |
| Everyone (S-1-1-0) | — | EXCLUDED |
| Anonymous Logon (S-1-5-7) | — | EXCLUDED |

### Mutex DACL

```
D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)
```

Only Administrators and LocalSystem can open the mode mutex. Interactive users do not need direct mutex access — they reach the service through the named pipe.

**NOT execution-verified:** Actual pipe DACL verification, authorized/unauthorized client connection tests, remote connection rejection tests require a compiled Go binary and Windows privileges.

---

## 6. Authoritative Mode Coordinator

### Architecture (Option A from NM-06 section 5)

Both engine processes acquire and hold the same kernel ownership mutex, but **only the elevated Go service touches it directly**. The unelevated Rust bonding engine requests Mode B through authenticated IPC.

| Component | Role |
|---|---|
| Go service (elevated) | Owns `mode.Coordinator`, which holds `modeowner.Owner` (kernel mutex). Authoritative coordinator. |
| Rust bonding engine (unelevated) | Requests Mode B via IPC `set_mode_b(active=true, rust_pid)`. Never touches the kernel mutex. |
| `mode.Coordinator` | Holds the mutex, tracks current mode and Rust PID, monitors Rust process lifetime |
| `modeowner.Owner` | Windows named mutex `Global\multinet-netmerge-mode-owner-v1` |

### Mode transitions

| Transition | Mechanism |
|---|---|
| Mode A start | `coord.RequestModeA()` → acquires kernel mutex, sets mode to NetMerge |
| Mode B start | Rust client sends IPC `set_mode_b(true, pid)` → `coord.RequestModeB(pid)` → acquires mutex, sets mode to Bonding, records PID |
| Mode A stop | `coord.ReleaseMode()` → releases mutex, sets mode to None |
| Mode B stop | IPC `set_mode_b(false, 0)` → `coord.ReleaseMode()` → releases mutex, sets mode to None |
| Mode conflict | `RequestModeA()` rejects if Mode B active; `RequestModeB()` rejects if Mode A active |
| Simultaneous requests | Mutex serializes acquisition; second caller gets `ERROR_ALREADY_EXISTS` or `WAIT_TIMEOUT` |
| Process crash | Kernel auto-releases mutex; `WAIT_ABANDONED` detected on next acquisition |
| Rust process exit | `MonitorRustProcess(pid)` polls `WaitForSingleObject` on process handle; calls `ReleaseMode()` when exited |

### Thread affinity

Windows mutex ownership is thread-specific. `TryAcquire()` calls `runtime.LockOSThread()` to pin the goroutine to an OS thread. `Release()` calls `runtime.UnlockOSThread()` after `ReleaseMutex`. A `lockThread` atomic flag tracks whether `LockOSThread` is active, preventing cross-thread release attempts.

### Bounded waits

`TryAcquire(timeout)` passes the timeout to `WaitForSingleObject`. `WAIT_TIMEOUT` returns an error. `tryAcquireWithTimeout` retries every 200ms up to a configurable `--acquire-timeout` (default 5s).

### Abandoned mutex recovery

`WAIT_ABANDONED` (0x80) from `WaitForSingleObject` indicates the previous owner crashed. Treated as successful acquisition with a warning. The coordinator does not automatically clean up the previous mode's resources (Wintun/routes for Mode A, bonding sockets for Mode B) — this is a known limitation documented for NM-07.

**NOT execution-verified:** Mutex acquisition, contention, release, thread-affinity, WAIT_ABANDONED recovery, process-lifetime monitoring all require a compiled Go binary and actual process execution.

---

## 7. Go/Rust Integration Tests

### Go tests (written, not compiled — Go not installed)

| Test file | Tests |
|---|---|
| `internal/ipc/contracts_test.go` | Request marshal roundtrip, invalid JSON rejection, unknown method, version mismatch, OK/error response shapes, MaxMessageSize constant, SetModeBParams roundtrip |
| `internal/ipc/pipesec_test.go` | SDDL restrictiveness, security descriptor creation, invalid SDDL rejection, authorizer policy (admin/system/user/unknown) |
| `internal/modeowner/modeowner_test.go` | Mutex acquire/release, release-when-not-owned, mutex name, acquire timeout, abandoned mutex recovery, mutex SDDL restrictiveness |
| `internal/mode/coordinator_test.go` | RequestModeA/ReleaseMode, RequestModeB with PID, mutual exclusion, release-when-none, MonitorRustProcess with pid=0 |

### Rust tests (compile, not executed — SAC 4551)

| Test | File | Coverage |
|---|---|---|
| `mode_conflict_prevented` | `lib.rs` | Mode A↔B conflicts rejected |
| `default_pipe_name_has_prefix` | `lib.rs` | Pipe name format |
| `pipe_version_is_one` | `lib.rs` | Protocol version |
| `default_request_timeout_is_ten_seconds` | `lib.rs` | Timeout constant |
| `set_mode_b_updates_local_tracking` | `lib.rs` | Local mode tracking with PID |
| `set_mode_b_conflict_with_mode_a` | `lib.rs` | Mode B activation while Mode A active |
| `io_error_classified_as_pipe_disconnected` | `error.rs` | ERROR_NO_DATA (232) → PipeDisconnected |
| `io_error_with_pipe_in_message_classified_as_disconnected` | `error.rs` | Broken pipe → PipeDisconnected |
| `generic_io_error_remains_io` | `error.rs` | Permission denied → Io |
| `health_request_roundtrips` | `contracts.rs` | Request serialization |
| `start_params_roundtrip` | `contracts.rs` | Start params serialization |
| `response_ok_shape` | `contracts.rs` | Response structure |
| `response_error_shape` | `contracts.rs` | Error response structure |
| `version_mismatch_detected` | `contracts.rs` | Pipe version validation |
| `all_methods_have_unique_names` | `contracts.rs` | Method name uniqueness |
| `start_params_default_weights_none` | `contracts.rs` | Default weights |
| `adapter_status_roundtrips` | `contracts.rs` | Adapter status serialization |
| `health_result_roundtrips` | `contracts.rs` | Health result serialization |

### Integration test (NOT EXECUTED)

A real Go-service/Rust-client named-pipe round trip requires:
1. Go service compiled and running (Go not installed)
2. Rust client compiled and able to connect (SAC blocks test binaries)
3. Named pipe accessible (requires Windows privileges)

The IPC contract is identical on both sides: `pipe_version = 1`, methods `health`/`status`/`adapters`/`start`/`stop`/`set_limit`/`mode_b_status`/`set_mode_b`, JSON request/response framing.

---

## 8. SAC and Signing

### Smart App Control status

SAC remains ENABLED throughout this session. No security policy was weakened.

### Code signing

No code-signing identity available. Unsigned Go binaries will be blocked by SAC at runtime (error 4551), as observed with Rust test binaries.

### SAC runtime verification

NOT EXECUTED. No signed binary produced, no execution test performed.

**Do not assume that a signed binary is automatically accepted by SAC.** Actual execution testing with SAC enabled is required in an approved environment.

---

## 9. Acceptance Criteria — Not Met

| Criterion | Status | Evidence |
|---|---|---|
| Go service builds | NOT MET | Go not installed |
| Go and Rust tests execute successfully | NOT MET | Go not installed; SAC 4551 |
| Named-pipe authorization tested | NOT MET | Source implemented, not execution-tested |
| Remote pipe connections rejected | NOT MET | PIPE_REJECT_REMOTE_CLIENTS in source, not tested |
| Elevated service reliably coordinates both modes | NOT MET | Source implemented, not execution-tested |
| Mode A/B exclusion verified through real processes | NOT MET | Requires two processes and compiled Go binary |
| Startup, shutdown, recovery pass Windows integration tests | NOT MET | Requires compiled Go binary + Wintun + admin |

**Three-interface hardware operation and SAC-enabled release verification remain separate acceptance gates.**

**Do not report successful compilation as proof of operational networking. Do not claim hardware or SAC verification without direct evidence.**

---

## 10. Remaining Blockers and Next Steps

| # | Blocker | Severity | Remediation |
|---|---|---|---|
| 1 | Go not installed | Critical | Install Go 1.27 in approved Windows build/test environment; run CI workflow |
| 2 | SAC blocks Rust test binaries (4551) | Critical | Move test execution to approved environment where SAC permits; or code-sign test binaries |
| 3 | Go service not compiled | Critical | Complete Go build once Go is available |
| 4 | No code-signing identity | High | Obtain code-signing cert; sign `net-merge-service.exe`; verify SAC execution |
| 5 | Pipe DACL and remote connection rejection not execution-tested | High | Test in approved environment with admin privileges |
| 6 | Cross-process mode ownership not tested | High | Test with Go service + Rust bonding engine as separate processes |
| 7 | Engine lifecycle not tested | High | Test Wintun init, route install/remove, weak-host restore on real Windows |
| 8 | Rust-to-Go IPC round trip not tested | High | Compile both sides, connect over named pipe, exchange requests |
| 9 | Abandoned mutex recovery not tested | High | Simulate process crash during Mode A/B, verify recovery |
| 10 | No three-interface hardware test | High | Execute hardware test plan |
| 11 | External Go deps not resolved | Medium | `go mod download` in build environment |

---

## 11. Repository Protection

- `D:\git\browser` was never touched.
- No `git reset`, `git clean`, or destructive cleanup performed.
- Nothing staged or committed.
- Existing database migrations preserved.
- NM-01 through NM-05 reports preserved.
- Rust bonding-engine code not overwritten.
- All pre-existing uncommitted modifications preserved.

---

## 12. Files Added or Changed

### Created

| File | Purpose |
|---|---|
| `.github/workflows/windows-build.yml` | Reproducible CI workflow |
| `services/net-merge/internal/ipc/pipesec_test.go` | SDDL, authorizer, token identity tests |
| `services/net-merge/internal/mode/coordinator.go` | Service-held mode coordinator with process tracking |
| `services/net-merge/internal/mode/coordinator_test.go` | Coordinator unit tests |
| `services/net-merge/internal/modeowner/modeowner_test.go` | Mutex tests (from NM-05, retained) |
| `docs/daily-reports/NET_MERGE_NM06_CI_AND_INTEGRATION_ACCEPTANCE_REPORT.md` | This report |

### Modified

| File | Change |
|---|---|
| `services/net-merge/main.go` | Uses `mode.Coordinator` instead of separate `mode.Tracker` + `modeowner.Owner`; deprecated `tryAcquireWithTimeout` |
| `services/net-merge/internal/ipc/pipesec_windows.go` | Added `PIPE_REJECT_REMOTE_CLIENTS`, `CheckTokenMembership`-based admin verification, `ClientIdentity`, `Authorizer`, per-method auth in `HandleConn` |
| `services/net-merge/internal/ipc/server.go` | Added `auth *Authorizer` field, per-method authorization check, `SetModeB` passes `rustPid` |
| `services/net-merge/internal/ipc/contracts.go` | Added `RustPid` field to `SetModeBParams` |
| `services/net-merge/internal/mode/exclusive.go` | Unchanged (simple tracker retained for compatibility) |
| `services/net-merge/internal/engineadapter/engine_adapter.go` | Uses `*mode.Coordinator` instead of `*mode.Tracker`; `Start()` calls `coord.RequestModeA()`, `Stop()` calls `coord.ReleaseMode()`, `SetModeB(active, rustPid)` calls `coord.RequestModeB()`/`coord.ReleaseMode()` |
| `services/net-merge/internal/engineadapter/engine_adapter_test.go` | Updated to use `mode.NewCoordinator()` and new `SetModeB(active, rustPid)` signature |
| `crates/netmerge-client/src/lib.rs` | `set_mode_b(active, rust_pid u32)` with PID parameter |
| `crates/netmerge-client/src/contracts.rs` | `SetModeBParams { active, rust_pid }`; `Request::set_mode_b(active, rust_pid)`; additional tests |

### Not touched

- `D:\git\browser` (never)
- Existing database migrations (preserved)
- Rust bonding-engine code (`crates/bonding-engine/*`) — not overwritten
- NM-01 through NM-05 reports — preserved
- All pre-existing uncommitted modifications — preserved

---

## Attestation

This report is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` and the current Multinet working tree. The Go source code is written but cannot be compiled or tested on this host because Go is not installed. Rust test binaries compile but cannot execute because Smart App Control (error 4551) remains enabled and was not modified. No results were fabricated; where compilation/execution could not be observed, that limitation is stated explicitly. NM-06 is **not accepted**.
