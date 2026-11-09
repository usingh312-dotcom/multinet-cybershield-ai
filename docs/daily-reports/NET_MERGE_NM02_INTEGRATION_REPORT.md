# NET MERGE INTEGRATION — NM-02 SERVICE INTEGRATION REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Network Systems Architect, Senior Go Engineer, Rust Architect, Windows Networking Engineer
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved. Rust bonding-engine code untouched. Database migrations preserved.

---

## STATUS SUMMARY

| Workstream | Status | Evidence |
|---|---|---|
| Rust IPC client crate (`netmerge-client`) | **COMPILED** | `cargo check -p netmerge-client` EXIT 0 |
| Rust IPC client unit tests | **BLOCKED** | Test binary compiles; execution blocked by Smart App Control error 4551 |
| Go IPC service (`services/net-merge/`) | **UNVERIFIED** | Go not installed on this host; cannot compile |
| Named-pipe IPC contract (v1) | **DEFINED** | `crates/netmerge-client/src/contracts.rs` + `services/net-merge/internal/ipc/contracts.go` |
| Mode A / Mode B mutual exclusion | **DEFINED (source)** | `services/net-merge/internal/mode/exclusive.go` + Rust client `set_mode` |
| Safe shutdown / route restoration | **DESIGNED** | `lifecycle/service.go` Stop() + engine teardown |
| Hardware verification | **NOT EXECUTED** | No three-interface test performed |

**NM-02 is not accepted as operational.** Source scaffolding is in place; the Rust client compiles but its tests cannot execute (SAC 4551); the Go service cannot be compiled (Go not installed); no hardware verification has been performed. Do not claim working integration based on source alone.

---

## 1. Upstream Commit and Source Inventory

**Commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` — "Release 0.3.7 (#78)", cloned 2026-11-09 to a temp working directory (not the Multinet repo).

### Top-level layout (upstream)

| Path | Purpose |
|---|---|
| `cmd/net-merge/` | CLI: `tray`, `run`, `on\|off\|quit`, `status`, `limit`, `socks`, `list`, `cleanup`, `autostart`, `install`, `version` |
| `internal/engine/` | Scheduler (weighted least-busy-flows), balancer (SOCKS5 + TCP/UDP relay), health probes, rate limits |
| `internal/nics/` | Adapter discovery (`GetAdaptersAddresses`), `IP_UNICAST_IF` egress forcing, per-interface byte counters |
| `internal/tun/` | Wintun virtual adapter, route add/delete (`0.0.0.0/1`, `128.0.0.0/1`), weak-host/forwarding control, stale cleanup |
| `internal/platform/` | Admin elevation (`runas`), single-instance mutex, tray, autostart (scheduled task), install |
| `internal/tray/` | Systray menu, autopause, update check |
| `internal/state/` | File-based status/limit markers (CLI ↔ running instance IPC) |
| `scripts/` | Build scripts, Wintun fetcher + signature verification, smoke tests |

### Go dependency manifest (`go.mod`)

Module `net-merge`, Go 1.27. Direct deps:
- `fyne.io/systray v1.12.2` — tray
- `github.com/vishvananda/netlink v1.3.1` — Linux links/routes
- `github.com/xjasonlyu/tun2socks/v2 v2.7.0` — userspace TUN→proxy
- `golang.org/x/net v0.56.0`, `golang.org/x/sys v0.46.0`
- `golang.zx2c4.com/wireguard v0.0.0-20260522210424-ecfc5a8d5446` — Wintun bindings
- `gvisor.dev/gvisor v0.0.0-20260701204157-69c2d17aea96` — userspace TCP/IP stack

### License
Upstream is **MIT** (Copyright 2026 Rohit Mahto). Wintun DLL is a separate signed binary from WireGuard LLC (fetched at install via `scripts/fetch-wintun.ps1`, SHA-256 pinned + Authenticode verified).

---

## 2. Actual Networking Architecture

Multinet now supports two distinct, non-confusing modes:

### MODE A — Multi-WAN Load Balancing (Net Merge Go service)

An independent, elevated Go process that:
- Discovers physical adapters (Wi-Fi, Ethernet, USB tether, phone hotspot); skips VPN/tunnel/virtual
- Creates a Wintun virtual adapter (`net-merge`) with fixed GUID
- Adds capture routes `0.0.0.0/1` and `128.0.0.0/1` (beat default route without replacing it)
- Runs a gVisor userspace TCP/IP stack + tun2socks; every captured flow is re-dialed through a pinned NIC
- Pins each flow's source IP + `IP_UNICAST_IF` (31) so it cannot be captured by the TUN's own routes (no routing loop)
- Balances new TCP/UDP flows across adapters via weighted least-busy-flows (per address family)
- Auto-weights adapters by measured throughput; static `-weight` overrides
- Probes internet per adapter/family (TCP to 1.1.1.1/8.8.8.8); dead adapters sidelined with cooldown, recover quickly
- Disables weak-host-send + forwarding on participating adapters while capture is on; restores on stop/crash
- Per-adapter speed caps via token buckets

**This is connection-level distribution:** one flow = one adapter. Multiple concurrent flows spread across links. A single long-lived TCP flow never spans adapters.

### MODE B — True Multinet UDP Bonding (existing Rust engine)

Retains `crates/bonding-engine`:
- Authenticated multi-path sessions (HMAC-SHA256 framing, constant-time tag verify, replay protection)
- Packet-level WDRR scheduling across paths
- UDP packet splitting; cloud packet reassembly (peer-side relay)
- Bidirectional forwarding; single-flow aggregation
- Tunnel failover

**This is packet-level bonding:** one flow split across paths. Complementary to Mode A, not interchangeable.

**Boundary rule:** Mode A and Mode B must never be active simultaneously (enforced by IPC + Rust client `set_mode`).

---

## 3. Reusable Modules (from upstream Net Merge)

| Upstream module | Reuse in NM-02 | Multinet equivalent (if any) |
|---|---|---|
| `internal/nics` (discovery, counters, ForceEgress) | **Full** — adapter discovery, per-interface bps, IP_UNICAST_IF pinning | `interface-discovery` + `network-monitor` (Rust, similar but separate) |
| `internal/engine/sched.go` | **Full** — weighted least-busy-flows, auto-weight, tiers, failover | `scheduler.rs` (WDRR, different algorithm) |
| `internal/engine/probe.go` | **Full** — per-family internet probes, up/down/unknown states | PathManager `infer_health` (loss/latency thresholds) |
| `internal/engine/balancer.go` | **Full** — DialTCP with failover, Relay byte accounting | `socket.rs` (BondedSocket) |
| `internal/engine/limit.go` | **Full** — per-adapter token-bucket speed caps | None |
| `internal/tun/tun_windows.go` | **Full** — Wintun adapter, routes, strong-host control, stale cleanup | `NetworkPlatform` trait (stubs only) |
| `internal/tun/tun.go` | **Full** — gVisor stack, tun2socks proxy, On/Off/teardown | None |
| `internal/platform/*` | **Full** — elevation, single-instance, autostart task | None |
| `internal/state/` | **Partial** — file-based status/limit markers (replaced by named pipe) | None |

**Not reused (Multinet retains its own):** HMAC-SHA256 framing (`tunnel-protocol`), Rust WDRR scheduler (`bonding-engine`), Tauri IPC/dashboard.

---

## 4. Go Service Integration Architecture

### Process model

```
MultiNet (Tauri, unelevated)  <-- named pipe -->  Net Merge service (elevated, Go)
     |                                               |
 IPC client (crates/netmerge-client)              named-pipe server (services/net-merge/)
     |                                               |
 Mode A/B exclusion                            owns Wintun + routes (Mode A only)
```

### Named-pipe endpoint
`\\.\pipe\multinet-netmerge-v1`

### ACL (PipeACL)
`D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-32-545)(A;;GRGW;;;S-1-1-0)`
- Administrators: full
- Interactive user (S-1-5-32-545): full
- Authenticated Users: read/write only

### Framing
One JSON object per line, newline-delimited. Client sends `Request`, server replies `Response`. Protocol version `pipe_version` must equal `1` or the request is rejected.

### IPC contracts (version 1)

| Method | Request params | Response result |
|---|---|---|
| `health` | none | `{service, version, pipe_version, mode_a_active, mode_b_active, on}` |
| `status` | none | `{on, mode_a, mode_b, adapters:[...], v6_state}` |
| `adapters` | none | `{adapters:[{name,index,ipv4,ipv6,weight,...}], skipped:[{name,reason}]}` |
| `start` | `{adapters, weights?, auto_weight, ipv6, mtu}` | `{status:"started"}` |
| `stop` | none | `{status:"stopped"}` |
| `set_limit` | `{adapter, mbit_per_sec}` | `{status:"applied"}` |
| `mode_b_status` | none | `{active: bool}` |

### Files created (Go service)

| File | Purpose |
|---|---|
| `services/net-merge/go.mod` | Go module `multinet.ai/netmerge-service` (Go 1.27) |
| `services/net-merge/LICENSE` | Upstream MIT license (verbatim) |
| `services/net-merge/THIRD_PARTY_NOTICES.md` | SBOM-style attribution |
| `services/net-merge/main.go` | Service entry point; named-pipe server + engine adapter |
| `services/net-merge/internal/ipc/contracts.go` | Request/Response types, router, marshal/unmarshal |
| `services/net-merge/internal/ipc/server.go` | Named-pipe listener, handlers, ACL |
| `services/net-merge/internal/lifecycle/service.go` | Start/Stop process lifecycle, readiness wait |
| `services/net-merge/internal/mode/exclusive.go` | Mode A / Mode B mutual exclusion tracker |

**Note:** `newEngine` in `main.go` is a stub that returns nil; the real implementation wraps upstream `engine.Scheduler`/`Balancer`/`tun.Tun`. Must be completed in an approved Go environment.

---

## 5. Rust Engine Compatibility

### New crate: `crates/netmerge-client`

Added to workspace `Cargo.toml` members. Compiles cleanly (`cargo check -p netmerge-client` EXIT 0).

| File | Purpose |
|---|---|
| `crates/netmerge-client/Cargo.toml` | serde, serde_json, thiserror, uuid (workspace deps) |
| `crates/netmerge-client/src/lib.rs` | `NetMergeClient`: connect, connected(), mode(), call(), set_mode() |
| `crates/netmerge-client/src/contracts.rs` | `Request`, `Response`, `Method`, `StartParams`, `AdapterStatus`, `HealthResult` |
| `crates/netmerge-client/src/error.rs` | `ClientError` enum |
| `crates/netmerge-client/src/pipe.rs` | `open()`, `open_with_retry()`, `PIPE_SDDL` |

### Mode A / Mode B mutual exclusion (Rust client)

`NetMergeClient::set_mode()` rejects a transition into Mode A while Mode B is active (and vice versa), returning `ClientError::ModeConflict`. This mirrors the Go service's `ModeBActive()` check in `handleStart`.

### Compatibility notes
- The Rust client opens `\\.\pipe\multinet-netmerge-v1` via `std::fs::File` (Windows named pipes are filesystem paths) — no extra Win32 crate needed.
- Each `call()` opens/closes a pipe connection (stateless, auditable). A long-lived connection can be added later.
- The client is Windows-only; non-Windows targets return `UnsupportedPlatform`.
- Tauri (unelevated) calls into `netmerge-client`; it never holds admin privileges.

---

## 6. Dual-Mode Product Architecture

| Aspect | Mode A (Net Merge) | Mode B (Rust bonding) |
|---|---|---|
| Mechanism | Connection-level load balancing | Packet-level UDP bonding |
| Scope | All system traffic via capture routes | Dedicated authenticated sessions |
| Single flow | Uses one link | Split across all paths |
| Best for | Downloads, browsers, updates, torrents | Cloud bonding, single-flow aggregation |
| Owner | Net Merge Go service (elevated) | `bonding-engine` Rust crate |
| IPC contract | `netmerge-client` | direct crate call |
| Wintun/routes | Yes, owned by Net Merge | No |
| Active simultaneously? | **No** — mutual exclusion enforced | **No** |

Mutual exclusion is enforced at two points:
1. Rust client `set_mode()` rejects conflicting transitions (`ClientError::ModeConflict`).
2. Go service `handleStart()` rejects start if `engine.ModeBActive()` is true.

---

## 7. Three-Path Hardware Compatibility

### Target hardware (Mode A)
1. **Main Wi-Fi** (built-in)
2. **USB tethering** (Android phone USB tether)
3. **External USB Wi-Fi adapter** (second phone hotspot)

### Required hardware tests (NOT yet executed — no three-interface test performed)

| Test | Mode | Success criterion |
|---|---|---|
| Wi-Fi 1 alone | A | Internet via Net Merge; adapter v4 up |
| USB tether alone | A | Internet via Net Merge; adapter v4 up |
| Wi-Fi 2 alone | A | Internet via Net Merge; adapter v4 up |
| All three active (parallel downloads) | A | Combined throughput > fastest single link; each adapter carries flows |
| Individual path disconnection | A | Disconnected adapter stops receiving flows within probe interval; traffic continues |
| Connection recovery | A | Reconnected adapter rejoins within seconds |
| DNS over each adapter | A | Name resolution works with each as sole active path |
| IPv4 | A | All traffic over IPv4 capture routes |
| IPv6 limitations | A | When not all adapters have IPv6, IPv6 refused (V6Auto); apps use IPv4 |
| Route restoration | A | After stop/crash: `128.0.0.0` route removed; normal default route restored; weak-host settings put back |
| Application traffic distribution | A | Multiple concurrent downloads spread across adapters per scheduler weights |

**Do not treat Net Merge's single-NIC Windows smoke test (`scripts/windows-smoke.ps1`) as proof of three-link aggregation.** It runs on one NIC and checks TUN/routes/probe/relay/off only.

---

## 8. Windows Security Review

### Smart App Control
SAC remains ENABLED. A freshly built, unsigned `net-merge-service.exe` will be blocked (error 4551), as observed with Rust test binaries. Before runtime acceptance:
- Code-sign the Net Merge binary with an enterprise certificate trusted by machine policy, OR
- Add the binary's path/publisher to the allowed list via policy (machine-owner decision; not modified during this assessment).

### Wintun DLL
Must be obtained per `scripts/fetch-wintun.ps1`: v0.14.1, SHA-256 `07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51`, Authenticode signature verified as "WireGuard LLC". Do not substitute an unverified DLL.

### Privilege isolation
- Net Merge needs elevation for Wintun, `route add/delete`, `netsh`, `IP_UNICAST_IF`, weak-host toggles.
- It re-launches itself elevated via `ShellExecute(runas)` when not admin.
- Tauri remains unelevated; all admin actions flow through the authenticated named pipe.
- Tauri must not receive unrestricted administrator privileges.

### IPC security
- Named pipe ACL restricts connect to administrators + interactive user.
- Every IPC request validated; unknown methods and unsupported versions rejected.
- No shell/command execution exposed.
- Version check on every request.

### Network safety
- Capture routes (`0.0.0.0/1`, `128.0.0.0/1`) take precedence without replacing default route; `Stop()` deletes them.
- `startStrongHost` disables weak-host-send/forwarding while capture is on; `stopStrongHost`/`restoreStrongHost` restore settings, including after crash (temp-file marker).
- IPv6 best-effort; when not all adapters have IPv6, IPv6 flows refused so apps fall back to IPv4.

---

## 9. Licensing Report

| Component | License | Attribution required | Notes |
|---|---|---|---|
| Net Merge (Go source) | MIT (Rohit Mahto, 2026) | Yes | Preserved verbatim in `services/net-merge/LICENSE` + source headers |
| Wintun DLL (WireGuard LLC) | Verify (binary dist) | Yes | Not in repo; fetched at install; signature verified |
| gVisor | Apache 2.0 | Yes | Upstream dependency; include in SBOM |
| tun2socks | MIT | Yes | Upstream dependency |
| golang.org/x/sys, x/net | BSD-style | Yes | Indirect deps |
| Multinet Rust crates | Proprietary | — | Separate work; controls service over IPC |

### SBOM action
Create `docs/licensing/NET_MERGE_SBOM.md` listing net-merge (MIT), gVisor (Apache 2.0), tun2socks (MIT), wireguard/wintun (verify), fyne/systray, vishvananda/netlink, golang.org/x/* with version pins from `go.mod`/`go.sum`.

---

## 10. Testing Strategy

### Unit tests
- **Rust `netmerge-client`:** 5 tests (`mode_conflict_prevented`, `default_pipe_name_has_prefix`, `pipe_version_is_one`, `health_request_roundtrips`, `start_params_roundtrip`, `response_ok_shape`). **Compiled but not executed** (SAC 4551 blocks test binary).
- **Rust `bonding-engine` / `tunnel-protocol`:** pre-existing; execution blocked by SAC (see M1.1 report).
- **Go:** upstream `internal/engine/*_test.go`, `internal/nics/nics_test.go`, `internal/tun/tun_test.go`. **Not run** (Go not installed).

### IPC contract tests
- Rust client → Go server: discover/start/stop/status/limit/health/mode_b_status roundtrips; malformed JSON; version mismatch; pipe disconnect/reconnect. **Not executable** (Go service not built, SAC blocks Rust test binaries).

### Integration tests
- Three-link hardware test plan (section 7).
- Crash recovery: kill Net Merge; confirm routes cleaned on restart; weak-host restored.
- SAC verification: signed Net Merge binary executes with SAC enabled.

### Not tested in NM-02
- No claim that Net Merge or Multinet bonding is operational without real three-link Windows hardware verification.

---

## 11. Performance Benchmarks Available

- Upstream scheduler: weighted least-busy-flows per address family; auto-weights from smoothed peak throughput; per-adapter token-bucket caps.
- Multinet scheduler: WDRR with dynamic weights from goodput/loss/RTT/jitter/queue (source written; execution verification blocked by SAC).
- No shared benchmark harness exists yet. NM-02 does not create one; performance numbers come from the hardware verification plan (section 7).
- Upstream README cites ~0.18 CPU cores per 100 Mbit/s and 232–303 Mbit/s on Wi-Fi + USB tether + iPhone hotspot (fastest single link ~121 Mbit/s) — vendor claim, verify on actual hardware.

---

## 12. Recommended Integration Decision

**Adopt Option A:** Run Net Merge as a separate, elevated Go service controlled by MultiNet over an authenticated versioned local named-pipe IPC. Multinet's Rust bonding engine remains for Mode B. The two modes operate on separate traffic classes with mutual exclusion.

**Do not:**
- Port Net Merge into Rust (Option C) — no demonstrated benefit; gVisor/Wintun/replace would have to be rebuilt.
- Let Tauri run elevated or call `netsh`/`route`/Wintun directly.
- Run unsigned Net Merge binaries with SAC enabled.
- Treat a single-NIC smoke test as three-link aggregation proof.

---

## 13. Implementation Roadmap

| Phase | Work | Gate |
|---|---|---|
| NM-02 (this) | Integration architecture; Go IPC scaffolding; Rust IPC client; IPC contracts v1 | This report reviewed |
| NM-03 | Build signed Net Merge binary; verify SAC execution on approved Windows VM; fetch+verify Wintun DLL | Binary runs with SAC on |
| NM-04 | Complete Go `engine` adapter wrapping upstream Scheduler/Balancer/tun.Tun | Go service compiles |
| NM-05 | Wire Tauri commands → NetMergeClient (discover/start/stop/status/limit/events) | IPC contract tests pass |
| NM-06 | Three-link hardware test (Mode A): all paths active, disconnection, recovery, DNS, IPv4/IPv6, route restoration | All section-7 tests pass |
| NM-07 | Performance benchmarks Mode A (two vs three connections) vs. fastest link | Numbers recorded, labeled |
| NM-08 | Licensing SBOM; attribution preserved | SBOM published |
| NM-09 | Mode B hardware test (Multinet Rust bonding) — only after M1.1 accepted | Not started |

**Stop condition:** Do not begin integration coding beyond this scaffolding until this assessment is reviewed. Do not claim Net Merge or Multinet true bonding is operational without real Windows three-link hardware verification.

---

## Files Changed / Created

### Created
- `crates/netmerge-client/Cargo.toml`
- `crates/netmerge-client/src/lib.rs`
- `crates/netmerge-client/src/contracts.rs`
- `crates/netmerge-client/src/error.rs`
- `crates/netmerge-client/src/pipe.rs`
- `services/net-merge/go.mod`
- `services/net-merge/LICENSE`
- `services/net-merge/THIRD_PARTY_NOTICES.md`
- `services/net-merge/main.go`
- `services/net-merge/internal/ipc/contracts.go`
- `services/net-merge/internal/ipc/server.go`
- `services/net-merge/internal/lifecycle/service.go`
- `services/net-merge/internal/mode/exclusive.go`
- `docs/daily-reports/NET_MERGE_NM02_INTEGRATION_REPORT.md` (this file)

### Modified
- `Cargo.toml` (workspace) — added `"crates/netmerge-client"` member

### Not touched
- `D:\git\browser` (never)
- Existing database migrations (preserved)
- Rust bonding-engine code (not overwritten)
- Any existing uncommitted modifications (preserved)

---

## Outstanding Blockers

1. **Go not installed** on this host — `services/net-merge/` cannot be compiled. Move to an approved Windows environment with Go.
2. **Smart App Control (error 4551)** blocks Rust test-binary execution — `netmerge-client` unit tests cannot run. Same blocker as M1.1.
3. **`newEngine` is a stub** — the real Go engine adapter wrapping upstream `Scheduler`/`Balancer`/`tun.Tun` is not implemented.
4. **No three-interface hardware test** — Mode A cannot be claimed operational.
5. **No code signing** — unsigned Net Merge binary will be blocked by SAC at runtime.

---

## Attestation

This report is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` and the current Multinet working tree. No upstream code was copied into the Multinet repo (the Go service is a separate module referencing upstream); attribution and MIT notices are preserved. No Multinet modifications were discarded, staged, or committed. `D:\git\browser` was never touched. Smart App Control remained enabled throughout; no Windows security policy was weakened. No results were fabricated; where compilation/execution could not be observed (Go not installed, SAC 4551), that limitation is stated explicitly. NM-02 is **not accepted as operational**.
