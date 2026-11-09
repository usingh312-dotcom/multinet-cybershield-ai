# NET MERGE INTEGRATION — NM-05 BUILD SECURITY & AUDIT REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Windows Systems Architect, Senior Go Engineer, Rust Systems Engineer, Windows Security Engineer, QA Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved. Rust bonding-engine code untouched. Database migrations preserved. NM-01 through NM-04 reports preserved.

---

## STATUS SUMMARY

| Workstream | Status | Evidence |
|---|---|---|
| Rust IPC client (`netmerge-client`) | **COMPILED** | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client unit tests | **SOURCE IMPLEMENTED** | Tests written; execution blocked by SAC error 4551 |
| Go IPC service + secured named pipe | **SOURCE IMPLEMENTED** | **NOT COMPILED** — Go not installed on this host |
| Go unit tests | **SOURCE IMPLEMENTED** | Tests written; cannot compile/run (Go not installed) |
| Named-pipe security (CreateNamedPipe + SDDL + client validation + per-method auth) | **SOURCE IMPLEMENTED** | Restrictive DACL, impersonation, Authorizer policy |
| Cross-process mode ownership (named mutex + thread affinity + WAIT_ABANDONED) | **SOURCE IMPLEMENTED** | `Global\multinet-netmerge-mode-owner-v1`, LockOSThread |
| Mode A / Mode B coordination protocol | **SOURCE IMPLEMENTED** | Mutex + tracker mutual exclusion |
| Rust-to-Go named-pipe round trip | **NOT EXECUTED** | Neither side runnable on this host |
| Windows lifecycle testing (startup, rollback, stop, Wintun, routes) | **NOT EXECUTED** | Go not installed; SAC blocks test binaries |
| Hardware verification | **NOT EXECUTED** | No three-interface test performed |
| Code signing / SAC runtime | **NOT EXECUTED** | No signing identity |

**NM-05 is not accepted.** Per the acceptance criteria, NM-05 requires the Go service to build, Go and Rust tests to execute, restrictive verified pipe DACL, correct mutex thread-affinity, both engines in authoritative ownership, real Go/Rust IPC round trip, and mode conflict/recovery tests. None of these can be confirmed on this host because Go is not installed and SAC blocks Rust test-binary execution. No results are fabricated.

---

## 1. Build Environment and Toolchain

Recorded on this host (Smart App Control remains enabled):

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 Home Single Language |
| OS Version | 10.0.26200 Build 26200 |
| Architecture | x64-based PC |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Cargo | cargo 1.99.1 (5f94df478 2026-08-27) |
| Rust target | stable-x86_64-pc-windows-msvc |
| Go | **NOT INSTALLED** — `go` command not found |
| Go version required by `services/net-merge/go.mod` | go 1.27 |
| Smart App Control | ENABLED (error 4551 observed blocking Rust test binaries) |

### Build-environment limitations (explicit)

1. **Go is not installed.** The `services/net-merge/` Go module cannot be compiled, and Go tests cannot be run. This is a hard blocker for NM-05 acceptance criteria 1–2.
2. **Smart App Control blocks test-binary execution.** `cargo test` binaries compile but cannot run (os error 4551). This affects `netmerge-client` and `bonding-engine` tests.
3. **No code-signing identity available.** Unsigned Go binaries will be blocked by SAC at runtime.
4. **External Go dependencies not resolved.** gVisor, tun2socks, Wintun bindings must be resolved via `go mod download` in the build environment.

---

## 2. CI Workflow

No CI workflow file exists in the repository. A reproducible Windows CI workflow should be added in NM-06+ with the following steps:

```yaml
# Proposed .github/workflows/windows-build.yml
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: '1.27'  # match services/net-merge/go.mod
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
      - name: Go build and test
        working-directory: services/net-merge
        run: |
          go mod download
          go fmt ./...
          go vet ./...
          go build -o net-merge-service.exe .
          go test ./...
      - name: Rust check
        run: |
          cargo check --workspace
      - name: Rust test
        run: cargo test --workspace
```

**Classification:** CI evidence, NOT SAC runtime verification.

---

## 3. Go and Rust Compilation Results

### Rust compilation results

| Crate | Command | Exit Code | Warnings | Notes |
|---|---|---|---|---|
| `netmerge-client` | `cargo check -p netmerge-client` | 0 | None | Compiles cleanly |
| `bonding-engine` | `cargo check -p bonding-engine` | 0 | None | Pre-existing, unchanged |

### Go compilation results

| Package | Command | Exit Code | Notes |
|---|---|---|---|
| All `services/net-merge/...` | `go build` | NOT EXECUTED | Go not installed |

---

## 4. Test Counts and Failures

### Rust unit-test results

| Test | Location | Status | Evidence |
|---|---|---|---|
| `mode_conflict_prevented` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `default_pipe_name_has_prefix` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `pipe_version_is_one` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `default_request_timeout_is_ten_seconds` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `set_mode_b_updates_local_tracking` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `set_mode_b_conflict_with_mode_a` | `netmerge-client::lib` | Compiled, not executed | SAC 4551 |
| `io_error_classified_as_pipe_disconnected` | `netmerge-client::error` | Compiled, not executed | SAC 4551 |
| `io_error_with_pipe_in_message_classified_as_disconnected` | `netmerge-client::error` | Compiled, not executed | SAC 4551 |
| `generic_io_error_remains_io` | `netmerge-client::error` | Compiled, not executed | SAC 4551 |
| `health_request_roundtrips` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `start_params_roundtrip` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `response_ok_shape` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `response_error_shape` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `version_mismatch_detected` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `all_methods_have_unique_names` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `start_params_default_weights_none` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `adapter_status_roundtrips` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |
| `health_result_roundtrips` | `netmerge-client::contracts` | Compiled, not executed | SAC 4551 |

### Go unit-test results (not compiled — Go not installed)

| Test file | Status | Evidence |
|---|---|---|
| `internal/ipc/contracts_test.go` | NOT COMPILED | Go not installed |
| `internal/ipc/pipesec_test.go` | NOT COMPILED | Go not installed |
| `internal/modeowner/modeowner_test.go` | NOT COMPILED | Go not installed |
| `internal/engineadapter/engine_adapter_test.go` | NOT COMPILED | Go not installed |

**Total Rust tests written:** 18 (all compiled, none executed due to SAC)
**Total Go tests written:** 11 (none compiled due to missing Go toolchain)

---

## 5. Final Pipe DACL and Identity Policy

### Named-pipe SDDL

```
D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)(A;;GRGW;;;S-1-5-18)
```

| ACE | Principal | Access | Purpose |
|---|---|---|---|
| `(A;;GA;;;BA)` | Built-in Administrators | Full Control | Service administration |
| `(A;;GRGW;;;S-1-5-32-545)` | Built-in Users | Read/Write | Interactive controller (Tauri) |
| `(A;;GRGW;;;S-1-5-18)` | LocalSystem | Read/Write | Service operation |
| **Everyone (S-1-1-0)** | — | **EXCLUDED** | No anonymous/network access |
| **Anonymous Logon (S-1-5-7)** | — | **EXCLUDED** | No anonymous access |

### Mutex SDDL

```
D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)
```

| ACE | Principal | Access | Purpose |
|---|---|---|---|
| `(A;;GA;;;BA)` | Built-in Administrators | Full Control | Mode ownership administration |
| `(A;;GA;;;S-1-5-18)` | LocalSystem | Full Control | Service operation |

### Per-method authorization policy (`Authorizer`)

| Method | Requires Admin | Standard User Allowed |
|---|---|---|
| `health` | No | Yes |
| `status` | No | Yes |
| `adapters` | No | Yes |
| `start` | Yes | No |
| `stop` | Yes | No |
| `set_limit` | Yes | No |
| `set_mode_b` | Yes | No |
| `mode_b_status` | No | Yes |

### Client identity validation

After `ConnectNamedPipe`, the server:
1. `ImpersonateNamedPipeClient` — impersonates the client
2. `OpenThreadToken` + `GetTokenUser` — retrieves the user SID
3. Compares against `WinBuiltinAdministratorsSid`, `WinBuiltinUsersSid`, `WinLocalSystemSid`
4. `RevertToSelf` — restores service identity
5. `Authorizer.Authorize(method, identity)` — checks per-method policy

### Remote-pipe rejection

Named pipes with `\\.\pipe\` are local-only by design. Remote SMB named pipes (`\\server\pipe\`) use a different path and are not supported by this service. The pipe ACL further restricts connect access to local identities only.

**NOT execution-verified:** All pipe security tests require a compiled Go binary and Windows admin privileges.

---

## 6. Mutex Thread-Affinity Design

### Design

Windows mutex ownership is thread-specific: the thread that acquires a mutex must be the one that releases it. The `modeowner.Owner` type enforces this:

1. `TryAcquire(timeout)` calls `runtime.LockOSThread()` to pin the goroutine to an OS thread
2. The thread lock is **not** released when `TryAcquire` returns — it is held until `Release()` is called
3. `Release()` calls `runtime.UnlockOSThread()` after `ReleaseMutex`
4. If `TryAcquire` fails (mutex owned by another process, or timeout), `runtime.UnlockOSThread()` is called immediately before returning
5. `lockThread` atomic flag tracks whether `LockOSThread` is active, preventing double-unlock or release without lock

### WAIT_ABANDONED handling

`WaitForSingleObject` can return `WAIT_ABANDONED` (0x80) when the previous owner terminated without releasing. The code:
- Treats `WAIT_ABANDONED` as a successful acquisition
- Logs that previous owner crashed, indicating possible inconsistent networking state
- Caller should reconcile (verify routes/adapters are clean)

### Bounded waits

`TryAcquire` accepts a `timeout` parameter passed to `WaitForSingleObject`. The `tryAcquireWithTimeout` helper in `main.go` retries every 200ms up to a configurable `--acquire-timeout` (default 5s).

### Mutex DACL

`D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)` — Administrators and LocalSystem only. Interactive users do not need direct mutex access; they reach the service through the named pipe.

**NOT execution-verified:** Thread-affinity, WAIT_ABANDONED recovery, and bounded waits require a compiled Go binary and actual process execution.

---

## 7. Mode Ownership Protocol

### Protocol design (Option A: both engines acquire the same kernel mutex)

| Aspect | Detail |
|---|---|
| Mutex name | `Global\multinet-netmerge-mode-owner-v1` |
| Owner A | Net Merge Go service (connection-level load balancing, Wintun + routes) |
| Owner B | Rust bonding engine (packet-level UDP bonding) |
| Acquisition | `CreateMutexW` + `WaitForSingleObject` with bounded timeout |
| Release | `ReleaseMutex` + `CloseHandle` on graceful shutdown |
| Crash recovery | Kernel auto-releases mutex on process termination |
| Stale ownership | `WAIT_ABANDONED` detected; treated as recovered but with warning |

### Mutual exclusion enforcement

| Layer | Mechanism |
|---|---|
| Kernel | Named mutex — only one process can hold it |
| Go service | `engine.Start()` rejects if `mode.IsBonding()`; `handleStart()` checks `ModeBActive()` |
| Rust client | `NetMergeClient::set_mode()` rejects A↔B conflicts; `set_mode_b()` sends IPC |
| Shared tracker | `mode.Tracker` with `SetBonding`/`SetNetMerge` for state reporting |

### Startup and cleanup safety

- Mode A start: verify Wintun driver (`CheckDriver`), discover adapters, create Wintun + routes, start probes
- Mode A stop: `tun.Close()` → `delRoutes()` + `stopStrongHost()` (restores weak-host settings)
- Mode B start: managed by Rust bonding engine (separate process, acquires same mutex)
- Mode B stop: managed by Rust bonding engine (releases mutex)
- Crash: mutex auto-released; `CleanupStale()` removes leftover routes on next start
- **Never deletes another mode's resources:** Each mode only cleans up what it created

**NOT execution-verified:** All mode ownership tests require two processes and a compiled Go binary.

---

## 8. Real IPC Round-Trip Evidence

**NOT EXECUTED.** A real Go-service/Rust-client named-pipe round trip requires:
1. Go service compiled and running (Go not installed)
2. Rust client compiled and able to connect (SAC blocks test binaries)
3. Named pipe accessible (requires Windows privileges)

The IPC contract is defined identically on both sides:
- Rust `crates/netmerge-client/src/contracts.rs`: `Request`/`Response` with `pipe_version`, `method`, `params`
- Go `services/net-merge/internal/ipc/contracts.go`: `Request`/`Response` with identical JSON tags

Both use `pipe_version = 1` and the same method set: `health`, `status`, `adapters`, `start`, `stop`, `set_limit`, `mode_b_status`, `set_mode_b`.

---

## 9. SAC Status and Unresolved Blockers

| Item | Status | Evidence |
|---|---|---|
| Smart App Control | ENABLED | Not modified |
| Rust test binary execution | BLOCKED | `os error 4551` observed directly |
| Go binary execution | NOT TESTED | Go not installed |
| Code signing | NOT EXECUTED | No signing identity available |
| Signed binary SAC test | NOT EXECUTED | No signed binary produced |

**Conclusion:** SAC runtime verification cannot be performed on this host. A code-signed binary must be tested in an approved environment where SAC permits execution.

### Remaining blockers

| # | Blocker | Severity | Remediation |
|---|---|---|---|
| 1 | Go not installed | Critical | Install Go 1.27 in approved Windows build/test environment |
| 2 | SAC blocks Rust test binaries (4551) | Critical | Move test execution to approved environment; or code-sign test binaries |
| 3 | Go service not compiled | Critical | Complete Go build once Go is available |
| 4 | No code-signing identity | High | Obtain code-signing cert; sign `net-merge-service.exe` |
| 5 | Pipe DACL not execution-tested | High | Test in approved environment with admin privileges |
| 6 | Cross-process mode ownership not tested | High | Test with two concurrent processes |
| 7 | Engine lifecycle not tested | High | Test Wintun init, route install/remove, weak-host restore |
| 8 | Rust-to-Go IPC round trip not tested | High | Compile both sides, connect over named pipe |
| 9 | No three-interface hardware test | High | Execute hardware test plan |
| 10 | External Go deps not resolved | Medium | `go mod download` in build environment |

---

## 10. Evidence Label Classification

| Result | Label | Evidence |
|---|---|---|
| Rust IPC client source | SOURCE IMPLEMENTED | `crates/netmerge-client/` compiles |
| Rust IPC client compiles | COMPILED | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client tests executed | NOT EXECUTED | SAC 4551 blocks test binary |
| Go engine adapter source | SOURCE IMPLEMENTED | `services/net-merge/internal/engineadapter/` |
| Go service compiles | NOT COMPILED | Go not installed |
| Go unit tests executed | NOT EXECUTED | Go not installed |
| Pipe DACL restrictive | SOURCE IMPLEMENTED | SDDL excludes Everyone/Anonymous |
| Per-method authorization | SOURCE IMPLEMENTED | `Authorizer` policy in `pipesec_windows.go` |
| Mutex thread-affinity | SOURCE IMPLEMENTED | `runtime.LockOSThread` in `modeowner_windows.go` |
| WAIT_ABANDONED handling | SOURCE IMPLEMENTED | Handled in `TryAcquire` |
| Mode ownership protocol | SOURCE IMPLEMENTED | Named mutex + tracker mutual exclusion |
| Go/Rust IPC round trip | NOT EXECUTED | Neither side runnable here |
| Mode conflict/recovery tests | NOT EXECUTED | Requires compiled binaries |
| Hardware verification | NOT EXECUTED | No three-interface test |
| SAC runtime verified | NOT EXECUTED | No signed binary tested |

---

## 11. Acceptance Criteria — Not Met

| Criterion | Status |
|---|---|
| Go service builds | NOT MET — Go not installed |
| Go and Rust automated tests execute | NOT MET — Go not installed; SAC 4551 |
| Named pipe has restrictive, verified access controls | NOT MET — source implemented, not execution-tested |
| Mutex acquisition/release correct across OS threads | NOT MET — source implemented, not execution-tested |
| Both engines participate in authoritative mode ownership | NOT MET — source implemented, not execution-tested |
| Real Go/Rust IPC round trip succeeds | NOT MET — neither side runnable here |
| Mode conflict and recovery tests pass | NOT MET — requires compiled binaries |

**Do not report successful compilation as proof of operational networking. Do not claim hardware or SAC verification without direct evidence.**

---

## 12. Repository Protection

- `D:\git\browser` was never touched.
- No `git reset`, `git clean`, or destructive cleanup performed.
- Nothing staged or committed.
- Existing database migrations preserved.
- NM-01 through NM-04 reports preserved.
- Rust bonding-engine code not overwritten.
- All pre-existing uncommitted modifications preserved.

---

## 13. Files Added or Changed

### Created

| File | Purpose |
|---|---|
| `services/net-merge/internal/ipc/pipesec_test.go` | SDDL restrictiveness, authorizer policy tests |
| `services/net-merge/internal/modeowner/modeowner_test.go` | Mutex acquire/release, timeout, SDDL tests |
| `docs/daily-reports/NET_MERGE_NM05_BUILD_AND_SECURITY_AUDIT_REPORT.md` | This report |

### Modified

| File | Change |
|---|---|
| `services/net-merge/internal/ipc/pipesec_windows.go` | Narrowed ACL (LocalSystem added), fixed SD use-after-free, added `ClientIdentity`, `Authorizer`, per-method auth in `HandleConn` |
| `services/net-merge/internal/ipc/server.go` | Added `auth *Authorizer` field, per-method authorization check in `HandleConn` |
| `services/net-merge/internal/modeowner/modeowner_windows.go` | Rewrote with `runtime.LockOSThread` affinity, `WaitForSingleObject` + bounded timeout, `WAIT_ABANDONED` handling, mutex SDDL |
| `services/net-merge/main.go` | Updated `tryAcquireWithTimeout` to pass timeout to `TryAcquire` |
| `crates/netmerge-client/src/contracts.rs` | Added tests: `response_error_shape`, `version_mismatch_detected`, `all_methods_have_unique_names`, `start_params_default_weights_none`, `adapter_status_roundtrips`, `health_result_roundtrips` |

### Not touched

- `D:\git\browser` (never)
- Existing database migrations (preserved)
- Rust bonding-engine code (`crates/bonding-engine/*`) — not overwritten
- NM-01 through NM-04 reports — preserved
- All pre-existing uncommitted modifications — preserved

---

## Attestation

This report is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` and the current Multinet working tree. The Go source code is written but cannot be compiled or tested on this host because Go is not installed. Rust test binaries compile but cannot execute because Smart App Control (error 4551) remains enabled and was not modified. No results were fabricated; where compilation/execution could not be observed, that limitation is stated explicitly. NM-05 is **not accepted**.
