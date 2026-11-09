# NET MERGE INTEGRATION — NM-03 BUILD AND ENGINE INTEGRATION REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Software Architect, Senior Go Engineer, Rust Systems Engineer, Windows Networking Engineer, Security Verification Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved. Rust bonding-engine code untouched. Database migrations preserved. NM-01 and NM-02 reports preserved.

---

## STATUS SUMMARY

| Workstream | Status | Evidence |
|---|---|---|
| Rust IPC client (`netmerge-client`) | **COMPILED** | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client unit tests | **SOURCE IMPLEMENTED** | Tests written; execution blocked by SAC error 4551 (cannot run) |
| Go IPC service + engine adapter | **SOURCE IMPLEMENTED** | **NOT COMPILED** — Go not installed on this host |
| Go unit tests | **SOURCE IMPLEMENTED** | Tests written; cannot compile/run (Go not installed) |
| Named-pipe IPC contract v1 (hardened) | **SOURCE IMPLEMENTED** | ACL, max message size, concurrency limit, request timeout, schema validation |
| Engine adapter (real upstream integration) | **SOURCE IMPLEMENTED** | Wraps upstream Scheduler/Balancer/Tun/Nics; not compiled |
| Mode A / Mode B mutual exclusion | **SOURCE IMPLEMENTED** | Shared `mode.Tracker`; enforced in Rust client + Go service |
| Transactional startup / rollback | **SOURCE IMPLEMENTED** | Rollback on tun.On() failure; safe Stop() |
| Hardware verification | **NOT EXECUTED** | No three-interface test performed |
| Code signing / SAC runtime | **NOT EXECUTED** | No signing identity; no signed binary tested |

**NM-03 is not accepted.** Per the Definition of Done, acceptance requires the Go service to compile, the real upstream engine integrated, the Rust client to communicate with it, lifecycle/mutual-exclusion tests to pass, and Windows runtime verification. None of these can be confirmed on this host because Go is not installed and SAC blocks Rust test-binary execution. No results are fabricated.

---

## 1. Toolchain and Environment Details

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
| Smart App Control | ENABLED (error 4551 observed blocking Rust/Go test binaries) |
| MSVC / link.exe | Not verified (not needed for pure-Rust crates) |

### Build-environment limitations (explicit)

1. **Go is not installed.** The `services/net-merge/` Go module cannot be compiled, and Go tests cannot be run. This is a hard blocker for NM-03 acceptance. Move to an approved Windows build/test environment with Go 1.27 installed.
2. **Smart App Control blocks test-binary execution.** `cargo test` binaries compile but cannot run (os error 4551). This affects both `netmerge-client` and `bonding-engine` tests. Observed directly during this session.
3. **No code-signing identity available.** Unsigned Go binaries will be blocked by SAC at runtime. A release-signing plan is required (section 8).
4. **External Go dependencies not vendored.** gVisor, tun2socks, Wintun bindings (`golang.zx2c4.com/wireguard/tun`) must be resolved via `go mod download` in the build environment. They are not present locally.

---

## 2. Upstream Commit and Licensing Record

| Field | Value |
|---|---|
| Repository | https://github.com/net-merge/net-merge |
| Commit | `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7 #78) |
| License | MIT (Copyright 2026 Rohit Mahto) |
| Attribution preserved | Yes — upstream LICENSE copied verbatim; THIRD_PARTY_NOTICES.md maintained |
| Vendored packages | `internal/engine`, `internal/nics`, `internal/tun`, `internal/platform` |
| Vendor location | `services/net-merge/internal/{engine,nics,tun,platform,logx}` |
| Import path re-base | `net-merge/internal/X` → `multinet.ai/netmerge-service/internal/X` |
| Functional changes to upstream | None |

### Third-party dependencies (SBOM-relevant)

| Component | License | Role |
|---|---|---|
| net-merge | MIT | Connection-level multi-WAN engine (vendored) |
| gVisor | Apache 2.0 | Userland TCP/IP stack (external dep) |
| tun2socks | MIT | TUN→proxy tunnel (external dep) |
| wireguard / wintun | Verify | Wintun DLL bindings (external dep; signed by WireGuard LLC) |
| golang.org/x/sys, x/net | BSD-style | System calls (external dep) |
| fyne.io/systray | MIT | Tray (external dep) |
| vishvananda/netlink | MIT | Linux links/routes (external dep; not used on Windows) |

Wintun DLL (v0.14.1, SHA-256 `07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51`) must be fetched per `scripts/fetch-wintun.ps1` with Authenticode signature verified against "WireGuard LLC".

---

## 3. Files Added or Changed

### Created

| File | Purpose |
|---|---|
| `services/net-merge/main.go` | Rewritten: uses real `engineadapter` instead of stub |
| `services/net-merge/internal/engineadapter/engine_adapter.go` | **NEW**: real engine adapter wrapping upstream Scheduler/Balancer/Tun |
| `services/net-merge/internal/engineadapter/engine_adapter_test.go` | **NEW**: mode exclusivity + lifecycle tests |
| `services/net-merge/internal/engine/sched.go` | **NEW**: vendored upstream Scheduler (weighted least-busy-flows) |
| `services/net-merge/internal/engine/balancer.go` | **NEW**: vendored upstream Balancer (DialTCP + Relay + failover) |
| `services/net-merge/internal/engine/limit.go` | **NEW**: vendored upstream token-bucket speed limits |
| `services/net-merge/internal/engine/probe.go` | **NEW**: vendored upstream health probes (1.1.1.1/8.8.8.8) |
| `services/net-merge/internal/nics/nics.go` | **NEW**: vendored upstream adapter discovery |
| `services/net-merge/internal/nics/info_windows.go` | **NEW**: vendored upstream GetAdaptersAddresses |
| `services/net-merge/internal/nics/counters_windows.go` | **NEW**: vendored upstream GetIfEntry2Ex counters |
| `services/net-merge/internal/nics/egress_windows.go` | **NEW**: vendored upstream IP_UNICAST_IF (31) forcing |
| `services/net-merge/internal/tun/tun.go` | **NEW**: vendored upstream Wintun TUN + gVisor stack |
| `services/net-merge/internal/tun/tun_windows.go` | **NEW**: vendored upstream Wintun configure/routes/weak-host |
| `services/net-merge/internal/tun/forward.go` | **NEW**: vendored upstream TCP/UDP forwarders |
| `services/net-merge/internal/platform/platform_windows.go` | **NEW**: vendored upstream elevation/single-instance |
| `services/net-merge/internal/platform/desktop_windows.go` | **NEW**: vendored upstream CheckDriver (Wintun DLL check) |
| `services/net-merge/internal/platform/autostart_windows.go` | **NEW**: vendored upstream scheduled-task autostart |
| `services/net-merge/internal/logx/logx.go` | **NEW**: vendored upstream leveled logging |
| `services/net-merge/LICENSE` | **NEW**: upstream MIT license (verbatim) |
| `services/net-merge/THIRD_PARTY_NOTICES.md` | **NEW**: SBOM-style attribution |
| `services/net-merge/internal/ipc/contracts_test.go` | **NEW**: IPC contract parsing, validation, security-limit tests |
| `docs/daily-reports/NET_MERGE_NM03_BUILD_AND_ENGINE_INTEGRATION_REPORT.md` | **NEW**: this report |

### Modified

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Already had `netmerge-client` member (NM-02); no new change |
| `services/net-merge/internal/ipc/contracts.go` | Added `MethodSetModeB`, `SetModeBParams`, security constants (`MaxMessageSize`, `MaxConcurrentConns`, `RequestTimeout`) |
| `services/net-merge/internal/ipc/server.go` | Hardened: concurrency semaphore, message-size limit, request timeout, schema validation, `SetModeB` handler |
| `services/net-merge/internal/mode/exclusive.go` | Added `SetBonding(bool)` and `SetNetMerge(bool)` for IPC-driven mode reporting |
| `crates/netmerge-client/src/contracts.rs` | Added `Method::SetModeB`, `SetModeBParams`, `Request::set_mode_b()` builder |
| `crates/netmerge-client/src/lib.rs` | Added `NetMergeClient::set_mode_b()` + unit tests |
| `crates/netmerge-client/src/contracts.rs` | Added `set_mode_b` roundtrip test |

### Not touched

- `D:\git\browser` (never)
- Existing database migrations (preserved)
- Rust bonding-engine code (`crates/bonding-engine/*`) — not overwritten
- NM-01 and NM-02 reports — preserved
- All existing uncommitted modifications — preserved

---

## 4. Real Engine Integration Status

The `newEngine` stub (NM-02) has been replaced with a real engine adapter in `services/net-merge/internal/engineadapter/engine_adapter.go`. It wraps the pinned upstream Net Merge components:

| Upstream component | Role in adapter | Status |
|---|---|---|
| `nics.DiscoverNicsDetailed` | Adapter discovery for `Discover()` | Integrated |
| `platform.CheckDriver` | Wintun DLL presence check before any system action | Integrated |
| `engine.NewScheduler` | Weighted least-busy-flows scheduler with auto-weight | Integrated |
| `engine.NewBalancer` | DialTCP with per-NIC pinning + failover + Relay | Integrated |
| `tun.New` / `tun.Tun.On` | Wintun adapter creation + capture routes + gVisor stack | Integrated |
| `sched.RunProbes` | Per-family health probes (1.1.1.1/8.8.8.8) | Integrated |
| `sched.SetLimit` | Per-adapter token-bucket speed caps | Integrated |
| `tun.Tun.Close` | Route removal + weak-host restoration on stop | Integrated |

### Transactional startup / rollback

`engine.Start()` is transactional:
1. Checks `platform.CheckDriver()` (Wintun present) — fails fast if missing.
2. Discovers adapters — fails if none usable.
3. Builds scheduler + balancer (in-memory, no system change).
4. Calls `tun.On()` — creates Wintun adapter, configures address/MTU, installs capture routes (`0.0.0.0/1`, `128.0.0.0/1`), starts weak-host control.
5. If `tun.On()` fails, `t.Close()` is invoked via deferred rollback to tear down anything started.
6. Only on success are `sched`, `balancer`, `tun` committed to the engine struct.

`engine.Stop()` calls `tun.Close()` which deletes capture routes and restores weak-host settings. Safe to call when already stopped.

### Mode A / B mutual exclusion

The `mode.Tracker` (shared between `main.go` and `engineadapter`) is the authoritative owner:
- `engine.Start()` rejects if `mode.IsBonding()` is true.
- `engineadapter.SetModeB(active)` sets Mode B state; if activated while Mode A is running, Mode A is stopped (`t.Close()`) to preserve exclusivity.
- The Rust client's `set_mode_b()` sends IPC `set_mode_b` to the service AND updates local tracking.
- `handleStart()` rejects with `mode_conflict` if `engine.ModeBActive()` is true.

No concurrent transition races: the tracker is mutex-guarded; the engine struct is mutex-guarded.

### What is NOT yet wired

- `engineadapter` does not yet call `sched.RunProbes` in a way that feeds probe results back into `Status()` adapter states beyond the initial snapshot. `Status()` reports `e.lastSnapshot` (taken at start). A live rescan/tick loop is not implemented in the adapter — this is a known gap for NM-04+.
- The `tun.Tun` `Warm()` / `KeepDevice` macOS behavior is not used on Windows.
- No `netsh`/`route` calls are made by Multinet directly — all routed through the Go service.

---

## 5. IPC Security Implementation

### Named-pipe endpoint
`\\.\pipe\multinet-netmerge-v1`

### Security controls (implemented in `internal/ipc/server.go`)

| Control | Implementation |
|---|---|
| Pipe ACL | `PipeACL()` returns SDDL `D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-32-545)(A;;GRGW;;;S-1-1-0)` — administrators + interactive user full; authenticated users read/write. **Note:** Go's `net.Listen("pipe", …)` does not accept a custom SDDL; production must create the pipe via Windows `CreateNamedPipe` with this security descriptor. Documented as a gap. |
| Concurrency limit | `connSem` channel buffered at `MaxConcurrentConns` (64); excess connections rejected. |
| Message size limit | `MaxMessageSize` (1 MiB); scanner buffer capped; oversized frames rejected with `message_too_large`. |
| Request timeout | `RequestTimeout` (10 s) read deadline per connection; stalled clients cannot hold connections. |
| Schema validation | Every request validated: non-empty `id`, matching `pipe_version`, known `method`, valid JSON. Malformed/unknown/unsupported-version requests rejected with structured `ServiceError`. |
| No arbitrary command execution | No shell, no `exec.Command` from IPC, no pass-through to OS commands. Only the 8 documented methods are dispatchable. |
| Clean disconnect | `scanner.Err()` checked; `io.EOF` treated as normal disconnect. |

### Tauri privilege isolation
- Tauri remains unelevated; it never holds admin privileges.
- All privileged operations (Wintun, routes, `netsh`, `IP_UNICAST_IF`, weak-host toggles) remain inside the elevated Go service.
- MultiNet talks to the service only over the authenticated named pipe.

---

## 6. Test Programme

### Go tests (written, NOT compiled/run — Go not installed)

| Test file | Coverage |
|---|---|
| `internal/ipc/contracts_test.go` | Request marshal roundtrip, invalid JSON rejection, unknown method, version mismatch, `NewOKResponse`/`NewErrorResponse` shapes, `MaxMessageSize` constant, `SetModeBParams` roundtrip |
| `internal/engineadapter/engine_adapter_test.go` | Mode exclusivity (ActivateNetMerge over Bonding fails), `SetBonding`/`SetNetMerge` state transitions |

### Rust tests (compile, NOT executed — SAC 4551)

| Test | Coverage |
|---|---|
| `netmerge-client::mode_conflict_prevented` | Mode A→B and B→A conflicts rejected |
| `netmerge-client::set_mode_b_updates_local_tracking` | Local mode tracking updates on set_mode_b |
| `netmerge-client::set_mode_b_conflict_with_mode_a` | Mode B activation while Mode A active |
| `netmerge-client::health_request_roundtrips` | Health request serializes with correct pipe version |
| `netmerge-client::start_params_roundtrip` | Start params roundtrip |
| `netmerge-client::response_ok_shape` | Response shape validation |
| `netmerge-client::set_mode_b_request_roundtrips` | SetModeB request serializes correctly |

### Tests that cannot run here (blockers)
- All Rust `cargo test` binaries — blocked by SAC error 4551
- All Go `go test` — Go not installed
- IPC roundtrip (Rust client ↔ Go service) — neither side executable here

### Windows integration tests (planned, not executed)
- Named-pipe ACL and authorization (connect as non-admin should be restricted)
- Service startup and shutdown (Wintun creation, route install, route removal)
- Route installation and restoration (`128.0.0.0` present during Mode A, absent after Stop)
- No simultaneous Mode A / Mode B ownership
- Adapter loss and recovery (disable an adapter, confirm flows stop; re-enable, confirm flows resume)

---

## 7. Three-Interface Hardware Verification

**NOT EXECUTED.** No three-interface hardware test has been performed. This host does not have the required topology attached, and no test environment with the three target interfaces is available in this session.

### Planned test topology (per NM-01/NM-02)

1. **Laptop primary Wi-Fi** (built-in)
2. **Phone 1 USB tethering** (Android)
3. **Phone 2 hotspot through a separate USB Wi-Fi adapter**

### Planned hardware tests (none executed)

| Test | Mode | Success criterion |
|---|---|---|
| Wi-Fi 1 alone | A | Internet via Net Merge; adapter v4 up |
| USB tether alone | A | Internet via Net Merge; adapter v4 up |
| Wi-Fi 2 alone | A | Internet via Net Merge; adapter v4 up |
| All three active (parallel downloads) | A | Combined throughput > fastest single link; each adapter carries flows |
| Individual path disconnection | A | Disconnected adapter stops receiving flows within probe interval; traffic continues |
| Connection recovery | A | Reconnected adapter rejoins and receives flows within seconds |
| DNS over each adapter | A | Name resolution works with each as sole active path |
| IPv4 | A | All traffic over IPv4 capture routes |
| IPv6 limitations | A | When not all adapters have IPv6, IPv6 refused (V6Auto); apps use IPv4 |
| Route restoration | A | After stop/crash: `128.0.0.0` route removed; normal default route restored; weak-host settings put back |
| Application traffic distribution | A | Multiple concurrent downloads spread across adapters per scheduler weights |

**Do not treat Net Merge's single-NIC smoke test as proof of three-link aggregation.** Net Merge is connection-level load balancing: one TCP connection = one adapter. Multiple concurrent connections spread across adapters.

---

## 8. Code Signing and Smart App Control

### Release-signing plan (not yet executed)

1. Obtain a code-signing certificate from a trusted CA (or use an internal enterprise CA trusted by the target machine policy).
2. Sign `net-merge-service.exe` and `wintun.dll` (or verify Wintun's existing WireGuard LLC signature).
3. Verify signature chain: `Get-AuthenticodeSignature` must return `Valid` with a trusted signer.
4. Test the signed binary on a Windows system with Smart App Control **enabled**.
5. Record the actual execution result — signing does not guarantee SAC acceptance.

### SAC verification (not executed)

- Unsigned Go binaries will be blocked by SAC (error 4551), as observed with Rust test binaries on this host.
- The Wintun DLL must be verified per `scripts/fetch-wintun.ps1` (SHA-256 + Authenticode "WireGuard LLC").
- Never disable SAC, bypass Code Integrity, or substitute unrelated trusted binaries.

### Status
| Item | Status |
|---|---|
| Signing identity | NOT OBTAINED |
| Signed binary | NOT PRODUCED |
| SAC runtime test | NOT EXECUTED |

---

## 9. Build and Test Commands (recorded; exit codes where observable)

### Rust (executed on this host)

```
cargo check -p netmerge-client
→ EXIT 0 (compiles cleanly)

cargo test -p netmerge-client
→ COMPILES, EXECUTION BLOCKED: os error 4551 (Smart App Control)

cargo check -p bonding-engine
→ EXIT 0 (pre-existing, unchanged)

cargo test -p bonding-engine
→ COMPILES, EXECUTION BLOCKED: os error 4551
```

### Go (NOT executable — Go not installed)

```
go version
→ command not found

go build -o net-merge-service.exe ./services/net-merge
→ NOT EXECUTED (Go not installed)

go test ./services/net-merge/...
→ NOT EXECUTED (Go not installed)

cargo fmt --all -- --check
→ BLOCKED: cargo-fmt blocked by SAC error 4551
```

### Commands to run in the approved environment

```
# Install Go 1.27 (matching upstream go.mod)
# Fetch dependencies
cd services/net-merge
go mod download

# Build
go build -o net-merge-service.exe .

# Vet
go vet ./...

# Run tests
go test ./...

# Fetch + verify Wintun
powershell -ExecutionPolicy Bypass -File scripts/fetch-wintun.ps1 -OutDir dist
```

---

## 10. Remaining Blockers and Remediation

| # | Blocker | Severity | Remediation |
|---|---|---|---|
| 1 | Go not installed | Critical | Install Go 1.27 in approved Windows build/test environment; run `go build`/`go test` |
| 2 | SAC blocks Rust test binaries (4551) | Critical | Move test execution to approved environment where SAC permits; or code-sign test binaries |
| 3 | `net-merge-service.exe` not compiled | Critical | Complete Go build once Go is available |
| 4 | No code-signing identity | High | Obtain code-signing cert; sign `net-merge-service.exe`; verify SAC execution |
| 5 | Pipe ACL not applied via `CreateNamedPipe` | Medium | Replace `net.Listen("pipe", …)` with Windows `CreateNamedPipe` using `PipeACL()` SDDL |
| 6 | No live rescan/tick in adapter | Medium | Wire `sched.Tick` + `sched.Update` loop into adapter for live adapter/weight updates |
| 7 | No three-interface hardware test | High | Execute hardware test plan on the three-interface topology |
| 8 | External Go deps not resolved | Medium | `go mod download` for gVisor, tun2socks, wintun bindings |

---

## 11. Classification of Results

| Result | Classification | Evidence |
|---|---|---|
| Rust IPC client source written | SOURCE IMPLEMENTED | `crates/netmerge-client/` compiles |
| Rust IPC client compiles | COMPILED | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client tests executed | NOT EXECUTED | SAC 4551 blocks test binary |
| Go engine adapter source written | SOURCE IMPLEMENTED | `services/net-merge/internal/engineadapter/` |
| Go service compiles | NOT COMPILED | Go not installed |
| Go unit tests executed | NOT EXECUTED | Go not installed |
| IPC security (ACL, limits, validation) | SOURCE IMPLEMENTED | `internal/ipc/server.go` + `contracts.go` |
| Mode exclusivity | SOURCE IMPLEMENTED | `mode.Tracker` + client + service enforcement |
| Transactional startup/rollback | SOURCE IMPLEMENTED | `engine.Start()` rollback on `tun.On()` failure |
| Wintun lifecycle | SOURCE IMPLEMENTED | Vendored `tun_windows.go` + `CheckDriver` |
| Route restoration | SOURCE IMPLEMENTED | `tun.Close()` → `delRoutes()` + `stopStrongHost()` |
| Hardware verification | NOT EXECUTED | No three-interface test performed |
| Code signing | NOT EXECUTED | No signing identity |
| SAC runtime verified | NOT EXECUTED | No signed binary tested |

---

## 12. Repository Protection

- `D:\git\browser` was never touched.
- No `git reset`, `git clean`, or destructive cleanup performed.
- Nothing staged or committed.
- Existing database migrations preserved.
- NM-01 and NM-02 reports preserved.
- Rust bonding-engine code not overwritten.
- All pre-existing uncommitted modifications preserved.

---

## 13. Definition of Done — Not Met

NM-03 is **not accepted** because:

- [ ] Go service compiles — **BLOCKED** (Go not installed)
- [x] Real upstream engine integrated — SOURCE IMPLEMENTED (not compiled)
- [ ] Rust client communicates with Go service — **NOT EXECUTED** (neither side runnable here)
- [ ] Lifecycle and mutual-exclusion tests pass — **NOT EXECUTED** (SAC + no Go)
- [ ] Windows runtime verification documented — **NOT EXECUTED**
- [ ] Three-interface hardware verification — **NOT EXECUTED** (separate, per NM-01/NM-02)
- [ ] SAC-enabled release execution — **NOT EXECUTED** (no signing identity)

**A successful compile is not proof of working multi-WAN networking.** The source is implemented and the Rust client compiles, but the Go service cannot be built or tested on this host.

---

## Attestation

This report is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` and the current Multinet working tree. Upstream packages were vendored into `services/net-merge/internal/` with MIT attribution preserved verbatim; no functional changes were made to upstream logic. No Multinet modifications were discarded, staged, or committed. `D:\git\browser` was never touched. Smart App Control remained enabled throughout; no Windows security policy was weakened. No results were fabricated; where compilation/execution could not be observed (Go not installed, SAC 4551), that limitation is stated explicitly.
