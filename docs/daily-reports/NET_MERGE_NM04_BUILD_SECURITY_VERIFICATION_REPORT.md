# NET MERGE INTEGRATION — NM-04 BUILD SECURITY & VERIFICATION REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Software Architect, Windows Security Engineer, Senior Go Engineer, Rust Systems Engineer, QA Verification Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved. Rust bonding-engine code untouched. Database migrations preserved. NM-01, NM-02, NM-03 reports preserved.

---

## STATUS SUMMARY

| Workstream | Status | Evidence |
|---|---|---|
| Rust IPC client (`netmerge-client`) | **COMPILED** | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client unit tests | **SOURCE IMPLEMENTED** | Tests written; execution blocked by SAC error 4551 |
| Go IPC service + secured named pipe | **SOURCE IMPLEMENTED** | **NOT COMPILED** — Go not installed on this host |
| Go unit tests | **SOURCE IMPLEMENTED** | Tests written; cannot compile/run (Go not installed) |
| Named-pipe security (CreateNamedPipe + SDDL + client validation) | **SOURCE IMPLEMENTED** | Restrictive DACL, impersonation-based identity validation |
| Cross-process mode ownership (named mutex) | **SOURCE IMPLEMENTED** | `Global\multinet-netmerge-mode-owner-v1` |
| Engine lifecycle (startup, rollback, stop, route restore) | **SOURCE IMPLEMENTED** | Vendored upstream Wintun/tun code; not executed |
| Rust-to-Go named-pipe round trip | **NOT EXECUTED** | Neither side runnable on this host |
| Hardware verification | **NOT EXECUTED** | No three-interface test performed |
| Code signing / SAC runtime | **NOT EXECUTED** | No signing identity |

**NM-04 is not accepted.** Per the acceptance criteria, NM-04 requires the Go service to compile, Go unit tests to pass, Rust-to-Go IPC communication, named-pipe authorization testing, cross-process mode ownership enforcement, and Windows lifecycle testing. None of these can be confirmed on this host because Go is not installed and SAC blocks Rust test-binary execution. No results are fabricated.

---

## 1. Toolchain Installation and Environment

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
| Go version required by upstream go.mod | go 1.27 |
| Smart App Control | ENABLED (error 4551 observed blocking Rust test binaries) |

### Build-environment limitations (explicit)

1. **Go is not installed.** The `services/net-merge/` Go module cannot be compiled, and Go tests cannot be run. This is a hard blocker for NM-04 acceptance criteria 1–2.
2. **Smart App Control blocks test-binary execution.** `cargo test` binaries compile but cannot run (os error 4551). This affects `netmerge-client` tests.
3. **No code-signing identity available.** Unsigned Go binaries will be blocked by SAC at runtime.
4. **External Go dependencies not resolved.** gVisor, tun2socks, Wintun bindings must be resolved via `go mod download` in the build environment.

### Commands to run in the approved environment

```powershell
# Install Go 1.27 (matching upstream go.mod)
# Then:
cd D:\git\multinet-cybershield-ai\services\net-merge
go mod download
go fmt ./...
go vet ./...
go build -o net-merge-service.exe .
go test ./...
```

```powershell
# Rust (on this host, compiles but tests blocked by SAC)
cargo check -p netmerge-client      # EXIT 0
cargo check -p bonding-engine       # EXIT 0
cargo test -p netmerge-client       # COMPILES, execution blocked by SAC 4551
```

---

## 2. Exact Go and Rust Build Commands

### Go (not executed — Go not installed)

| Command | Purpose | Expected |
|---|---|---|
| `go mod download` | Resolve external dependencies (gVisor, tun2socks, wintun) | Exit 0 |
| `go fmt ./...` | Format check | Exit 0, no output |
| `go vet ./...` | Static analysis | Exit 0 |
| `go build -o net-merge-service.exe .` | Compile service | Exit 0, binary produced |
| `go test ./...` | Run unit tests | Exit 0, all tests pass |

### Rust (executed on this host)

| Command | Purpose | Result |
|---|---|---|
| `cargo check -p netmerge-client` | Compile IPC client | **EXIT 0** |
| `cargo check -p bonding-engine` | Compile bonding engine | **EXIT 0** |
| `cargo test -p netmerge-client` | Run IPC client tests | **COMPILES, blocked by SAC 4551** |
| `cargo test -p bonding-engine` | Run bonding engine tests | **COMPILES, blocked by SAC 4551** |
| `cargo fmt --all -- --check` | Format check | **BLOCKED by SAC 4551** |

---

## 3. Compilation and Unit-Test Results

### Rust compilation results

| Crate | Command | Exit Code | Warnings | Notes |
|---|---|---|---|---|
| `netmerge-client` | `cargo check -p netmerge-client` | 0 | None | Compiles cleanly |
| `bonding-engine` | `cargo check -p bonding-engine` | 0 | None | Pre-existing, unchanged |

### Rust unit-test results

| Test | Status | Evidence |
|---|---|---|
| `mode_conflict_prevented` | Compiled, not executed | SAC 4551 |
| `default_pipe_name_has_prefix` | Compiled, not executed | SAC 4551 |
| `pipe_version_is_one` | Compiled, not executed | SAC 4551 |
| `default_request_timeout_is_ten_seconds` | Compiled, not executed | SAC 4551 |
| `set_mode_b_updates_local_tracking` | Compiled, not executed | SAC 4551 |
| `set_mode_b_conflict_with_mode_a` | Compiled, not executed | SAC 4551 |
| `io_error_classified_as_pipe_disconnected` | Compiled, not executed | SAC 4551 |
| `io_error_with_pipe_in_message_classified_as_disconnected` | Compiled, not executed | SAC 4551 |
| `generic_io_error_remains_io` | Compiled, not executed | SAC 4551 |

### Go compilation results

| Package | Command | Exit Code | Notes |
|---|---|---|---|
| All `services/net-merge/...` | `go build` | NOT EXECUTED | Go not installed |

### Go unit-test results

| Test file | Status | Evidence |
|---|---|---|
| `internal/ipc/contracts_test.go` | Not compiled | Go not installed |
| `internal/ipc/pipesec_test.go` | Not compiled | Go not installed |
| `internal/modeowner/modeowner_test.go` | Not compiled | Go not installed |
| `internal/engineadapter/engine_adapter_test.go` | Not compiled | Go not installed |

---

## 4. Named-Pipe ACL and Authorization Results

### Security descriptor (SDDL)

```
D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)
```

| Component | Meaning |
|---|---|
| `D:P` | DACL is protected (not inherited from parent object) |
| `(A;;GA;;;BA)` | Allow Full Control (GA) to Built-in Administrators (BA) |
| `(A;;GRGW;;;S-1-5-32-545)` | Allow Read/Write (GR, GW) to Built-in Users (interactive logon) |
| **Everyone (S-1-1-0)** | **Explicitly excluded** — no network or anonymous access |
| **Anonymous logon (S-1-5-7)** | **Explicitly excluded** |

### Pipe creation (CreateNamedPipe, not net.Listen)

The pipe is created via `CreateNamedPipeW` with:
- `PIPE_ACCESS_DUPLEX \| FILE_FLAG_FIRST_PIPE_INSTANCE`
- `PIPE_TYPE_MESSAGE \| PIPE_READMODE_MESSAGE \| PIPE_WAIT`
- `PIPE_UNLIMITED_INSTANCES`
- 1 MiB in/out buffer sizes (enforces `MaxMessageSize`)
- Custom `SECURITY_ATTRIBUTES` with the restrictive SDDL above

### Client identity validation

After `ConnectNamedPipe` succeeds, the server:
1. Calls `ImpersonateNamedPipeClient` to impersonate the client
2. Opens the thread token (`OpenThreadToken` with `TOKEN_QUERY`)
3. Retrieves the user SID (`GetTokenUser`)
4. Checks if the SID matches `WinBuiltinAdministratorsSid` (authorized)
5. Checks if the SID matches `WinBuiltinUsersSid` (authorized — local interactive)
6. Rejects all other identities with `ClientError` / `ServiceError`
7. Reverts to service identity (`RevertToSelf`)

### Results

| Test | Status | Evidence |
|---|---|---|
| SDDL excludes Everyone | SOURCE IMPLEMENTED | `pipesec_test.go::TestPipeSDDLIsRestrictive` |
| SDDL grants Administrators full control | SOURCE IMPLEMENTED | `pipesec_test.go::TestPipeSDDLIsRestrictive` |
| SDDL has protected DACL | SOURCE IMPLEMENTED | `pipesec_test.go::TestPipeSDDLIsRestrictive` |
| Security descriptor creation from SDDL | SOURCE IMPLEMENTED | `pipesec_test.go::TestStringToSecurityDescriptor` |
| Invalid SDDL rejected | SOURCE IMPLEMENTED | `pipesec_test.go::TestStringToSecurityDescriptorInvalid` |
| Client identity validation | SOURCE IMPLEMENTED | `validateClientIdentity()` in `pipesec_windows.go` |
| Unauthorized client rejection | SOURCE IMPLEMENTED | Server rejects non-admin/non-users clients |
| Max message size (1 MiB) | SOURCE IMPLEMENTED | `MaxMessageSize` constant + pipe buffer sizes |
| Request timeout (10 s) | SOURCE IMPLEMENTED | `RequestTimeout` constant + read deadline |
| Concurrency limit (64) | SOURCE IMPLEMENTED | `connSem` semaphore |
| Request schema validation | SOURCE IMPLEMENTED | ID check, version check, method check in `HandleConn` |
| No arbitrary command execution | SOURCE IMPLEMENTED | Only 8 documented methods dispatchable |

**NOT execution-verified:** Named-pipe creation, `ConnectNamedPipe`, `ImpersonateNamedPipeClient`, `ConnectNamedPipe` authorization — all require Windows privileges and a compiled Go binary.

---

## 5. Cross-Process Mode Ownership Tests

### Mechanism

A Windows named mutex `Global\multinet-netmerge-mode-owner-v1` provides cross-process ownership:
- Created with `CreateMutexW` and `bInitialOwner = TRUE`
- If `GetLastError` returns `ERROR_ALREADY_EXISTS` (183), another process owns the mode
- The mutex is kernel-owned: if the owning process crashes, the mutex is automatically released
- On graceful shutdown, `ReleaseMutex` + `CloseHandle` explicitly releases it

### Mode A (Net Merge) acquisition flow

1. Service starts, creates `modeowner.Owner`
2. Calls `TryAcquire()` with retry loop (200ms intervals, up to `--acquire-timeout` seconds, default 5s)
3. If acquired: Mode A is available, Wintun/tun can start
4. If not acquired: another process (Mode B / Rust bonding engine) owns the tunnel; Mode A start is refused
5. On shutdown: `Release()` is called (deferred)
6. On crash: kernel auto-releases the mutex; next start can acquire

### IPC-level enforcement

- `handleStart()` rejects with `mode_conflict` if `engine.ModeBActive()` is true
- `handleSetModeB()` updates the mode tracker via `engine.SetModeB()`
- `ModeBActive()` is checked before every Mode A start

### Results

| Test | Status | Evidence |
|---|---|---|
| Mutex acquire + release | SOURCE IMPLEMENTED | `modeowner_test.go::TestTryAcquireAndRelease` |
| Release when not owned (no-op) | SOURCE IMPLEMENTED | `modeowner_test.go::TestReleaseWhenNotOwned` |
| Mutex name constant | SOURCE IMPLEMENTED | `modeowner_test.go::TestMutexName` |
| Re-acquire while owned rejected | SOURCE IMPLEMENTED | `modeowner_test.go::TestTryAcquireAndRelease` |
| Cross-process contention | NOT EXECUTED | Requires two processes |
| Stale ownership after crash | NOT EXECUTED | Kernel auto-release; not tested |
| Mode A start rejected when Mode B active | SOURCE IMPLEMENTED | `handleStart` checks `ModeBActive()` |
| Atomic transition | SOURCE IMPLEMENTED | Mutex + tracker are acquired together in `main()` |

**NOT execution-verified:** Cross-process contention, stale ownership recovery, atomic transitions under race conditions — require multiple processes and a compiled Go binary.

---

## 6. Engine Lifecycle Tests

### Lifecycle design (SOURCE IMPLEMENTED, not executed)

| Phase | Implementation | Status |
|---|---|---|
| Adapter discovery | `nics.DiscoverNicsDetailed` — `GetAdaptersAddresses` + `net.Interfaces` | SOURCE IMPLEMENTED |
| Wintun driver check | `platform.CheckDriver` — verifies `wintun.dll` next to exe | SOURCE IMPLEMENTED |
| Wintun initialization | `tun.Tun.On` — creates Wintun adapter, configures address/MTU | SOURCE IMPLEMENTED |
| Capture route installation | `tun_windows.go::configure` — `route add 0.0.0.0/1, 128.0.0.0/1` | SOURCE IMPLEMENTED |
| Health probes | `sched.RunProbes` — TCP to 1.1.1.1/8.8.8.8 per family | SOURCE IMPLEMENTED |
| Failed startup rollback | `engine.Start()` defers `t.Close()` on `tun.On()` failure | SOURCE IMPLEMENTED |
| Normal stop | `engine.Stop()` → `tun.Close()` → `delRoutes()` + `stopStrongHost()` | SOURCE IMPLEMENTED |
| Route cleanup | `delRoutes()` deletes `0.0.0.0/1, 128.0.0.0/1` routes | SOURCE IMPLEMENTED |
| Weak-host restoration | `stopStrongHost()` / `restoreStrongHost()` — puts back netsh settings | SOURCE IMPLEMENTED |
| Adapter loss/recovery | `sched.Tick` + `sched.Update` — link counters + rescan | SOURCE IMPLEMENTED (not wired into adapter rescan loop) |
| Service restart after crash | Mutex auto-release + `CleanupStale()` removes leftover routes | SOURCE IMPLEMENTED |

### Results

| Test | Status | Evidence |
|---|---|---|
| Service startup | SOURCE IMPLEMENTED | `engineadapter::Start()` |
| Upstream adapter discovery | SOURCE IMPLEMENTED | `nics::DiscoverNicsDetailed` |
| Wintun initialization | SOURCE IMPLEMENTED | `tun::On` + `tun_windows::configure` |
| Capture-route installation | SOURCE IMPLEMENTED | `route add 0.0.0.0/1, 128.0.0.0/1` |
| Failed startup rollback | SOURCE IMPLEMENTED | Deferred `t.Close()` on error |
| Normal stop + route cleanup | SOURCE IMPLEMENTED | `tun.Close()` → `delRoutes()` |
| Weak-host restoration | SOURCE IMPLEMENTED | `stopStrongHost()` / `restoreStrongHost()` |
| Adapter loss and recovery | SOURCE IMPLEMENTED | `sched.Tick` + probe tiers |
| Service restart after interrupted shutdown | SOURCE IMPLEMENTED | Mutex auto-release + `CleanupStale()` |

**NOT execution-verified:** All engine lifecycle tests require Windows admin privileges, Wintun driver, and a compiled Go binary.

**Not yet wired:** The adapter rescan/tick loop (`sched.Tick` + `sched.Update` on a timer) is not connected in the engine adapter — `Status()` reports the snapshot taken at start time. This is a known gap for NM-05+.

---

## 7. Go and Rust Integration Verification

### Rust client capabilities

| Feature | Status |
|---|---|
| IPC serialization/deserialization | SOURCE IMPLEMENTED — `serde_json` for `Request`/`Response` |
| Protocol-version compatibility | SOURCE IMPLEMENTED — `pipe_version` checked on every request/response |
| Timeouts | SOURCE IMPLEMENTED — `call_with_timeout` with channel-based worker thread |
| Disconnect handling | SOURCE IMPLEMENTED — `PipeDisconnected` error variant, `From<std::io::Error>` classification |
| Mode transition rejection | SOURCE IMPLEMENTED — `set_mode()` rejects A↔B conflicts |
| Service error propagation | SOURCE IMPLEMENTED — `ServiceError` mapped to `ClientError::Service` |

### Go server capabilities

| Feature | Status |
|---|---|
| Request parsing | SOURCE IMPLEMENTED — `json.Unmarshal` with schema validation |
| Version validation | SOURCE IMPLEMENTED — `pipe_version` must equal `PIPEVersion` |
| Unknown method rejection | SOURCE IMPLEMENTED — `unknown_method` error |
| Malformed JSON rejection | SOURCE IMPLEMENTED — `invalid_json` error |
| Message size limit | SOURCE IMPLEMENTED — 1 MiB buffer + explicit check |
| Concurrency limit | SOURCE IMPLEMENTED — 64-connection semaphore |

### Rust-to-Go named-pipe round trip

**NOT EXECUTED.** Requires:
1. Go service compiled and running (Go not installed)
2. Rust client able to connect (SAC blocks test binaries)
3. Named pipe accessible (requires Windows privileges)

This is a hard blocker for acceptance criterion 3 ("The Rust IPC client communicates with the Go service").

---

## 8. SAC Execution Results

| Item | Status | Evidence |
|---|---|---|
| Smart App Control | ENABLED | Not modified |
| Rust test binary execution | BLOCKED | `os error 4551` observed directly |
| Go binary execution | NOT TESTED | Go not installed; unsigned binaries expected to be blocked |
| Code signing | NOT EXECUTED | No signing identity available |
| Signed binary SAC test | NOT EXECUTED | No signed binary produced |

**Conclusion:** SAC runtime verification cannot be performed on this host. A code-signed binary must be tested in an approved environment where SAC permits execution.

---

## 9. Hardware Test Evidence

**NOT EXECUTED.** No three-interface hardware test has been performed.

Planned topology (per NM-01/NM-02):
1. Laptop primary Wi-Fi
2. Phone 1 USB tethering (Android)
3. Phone 2 hotspot through separate USB Wi-Fi adapter

Planned tests (none executed):
- Interface discovery and identification
- Independent path connectivity
- Per-interface traffic and throughput
- Connection-level load distribution
- Interface disconnection and recovery
- Route restoration

**Do not report single-flow packet aggregation for Net Merge.** Its intended operation is distributing separate connections across WAN links (connection-level load balancing, not packet-level bonding).

---

## 10. Remaining Blockers and Next Steps

| # | Blocker | Severity | Remediation |
|---|---|---|---|
| 1 | Go not installed | Critical | Install Go 1.27 in approved Windows build/test environment; run `go build`/`go test` |
| 2 | SAC blocks Rust test binaries (4551) | Critical | Move test execution to approved environment where SAC permits; or code-sign test binaries |
| 3 | Go service not compiled | Critical | Complete Go build once Go is available |
| 4 | No code-signing identity | High | Obtain code-signing cert; sign `net-merge-service.exe`; verify SAC execution |
| 5 | Pipe ACL via `CreateNamedPipe` not execution-tested | High | Test in approved environment with admin privileges |
| 6 | Cross-process mode ownership not tested | High | Test with two concurrent processes (Go service + Rust bonding engine) |
| 7 | Engine lifecycle not tested | High | Test Wintun init, route install/remove, weak-host restore on real Windows |
| 8 | Rust-to-Go IPC round trip not tested | High | Compile both sides, connect over named pipe, exchange requests |
| 9 | No three-interface hardware test | High | Execute hardware test plan on three-interface topology |
| 10 | External Go deps not resolved | Medium | `go mod download` for gVisor, tun2socks, wintun bindings |
| 11 | Adapter rescan/tick loop not wired | Medium | Connect `sched.Tick` + `sched.Update` to a timer in engine adapter |

---

## 11. Acceptance Criteria — Not Met

| Criterion | Status | Evidence |
|---|---|---|
| Go service compiles | NOT MET | Go not installed |
| Go unit tests execute and pass | NOT MET | Go not installed |
| Rust IPC client communicates with Go service | NOT MET | Neither side runnable here |
| Named-pipe authorization applied and tested | NOT MET | Requires compiled Go binary + Windows privileges |
| Mode ownership enforced across processes | NOT MET | Requires two processes |
| Startup, rollback, stop tested on Windows | NOT MET | Requires compiled Go binary + Wintun + admin |

**Three-interface hardware operation remains a separate acceptance gate.**

**Do not report a source-level implementation or successful Rust `cargo check` as proof that the complete Net Merge integration works.**

---

## 12. Repository Protection

- `D:\git\browser` was never touched.
- No `git reset`, `git clean`, or destructive cleanup performed.
- Nothing staged or committed.
- Existing database migrations preserved.
- NM-01, NM-02, NM-03 reports preserved.
- Rust bonding-engine code not overwritten.
- All pre-existing uncommitted modifications preserved.

---

## 13. Files Added or Changed

### Created

| File | Purpose |
|---|---|
| `services/net-merge/internal/ipc/pipesec_windows.go` | Secure named pipe creation (CreateNamedPipe + SDDL), client identity validation, HandleConn |
| `services/net-merge/internal/ipc/pipesec_test.go` | SDDL restrictiveness tests, security descriptor creation tests |
| `services/net-merge/internal/modeowner/modeowner_windows.go` | Cross-process mode ownership via Windows named mutex |
| `services/net-merge/internal/modeowner/modeowner_test.go` | Mutex acquire/release tests |
| `docs/daily-reports/NET_MERGE_NM04_BUILD_SECURITY_VERIFICATION_REPORT.md` | This report |

### Modified

| File | Change |
|---|---|
| `services/net-merge/main.go` | Uses `modeowner.Owner` for cross-process ownership; `engineadapter` instead of stub; acquire-timeout flag |
| `services/net-merge/internal/ipc/server.go` | Rewrote `Run()` to use `securePipe` + `ConnectNamedPipe` + `validateClientIdentity` + `HandleConn`; removed `net.Listener`; added `SetModeB` handler |
| `services/net-merge/internal/ipc/contracts.go` | Added `MethodSetModeB`, `SetModeBParams`, security constants (`MaxMessageSize`, `MaxConcurrentConns`, `RequestTimeout`) |
| `services/net-merge/internal/mode/exclusive.go` | Added `SetBonding(bool)` and `SetNetMerge(bool)` methods |
| `services/net-merge/internal/lifecycle/service.go` | Fixed `cfgStateDir` package-level hack; captures `Config` in struct |
| `crates/netmerge-client/src/lib.rs` | Added `call_with_timeout`, `DEFAULT_REQUEST_TIMEOUT`, `PipeDisconnected` handling, worker-thread I/O with channel timeout |
| `crates/netmerge-client/src/error.rs` | Added `PipeDisconnected`, `RequestTimeout` variants; `From<std::io::Error>` classification; error classification tests |
| `crates/netmerge-client/src/contracts.rs` | Added `Method::SetModeB`, `SetModeBParams`, `Request::set_mode_b()`, roundtrip test |

### Not touched

- `D:\git\browser` (never)
- Existing database migrations (preserved)
- Rust bonding-engine code (`crates/bonding-engine/*`) — not overwritten
- NM-01, NM-02, NM-03 reports — preserved
- All pre-existing uncommitted modifications — preserved

---

## Attestation

This report is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` and the current Multinet working tree. The Go source code is written but cannot be compiled or tested on this host because Go is not installed. Rust test binaries compile but cannot execute because Smart App Control (error 4551) remains enabled and was not modified. No results were fabricated; where compilation/execution could not be observed, that limitation is stated explicitly. NM-04 is **not accepted**.
