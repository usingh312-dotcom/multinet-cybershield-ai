# NET MERGE INTEGRATION ASSESSMENT — NM-01

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Upstream commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7, cloned 2026-11-09)
**Date:** 2026-11-09
**Role:** Principal Network Systems Architect, Senior Go Engineer, Rust Architect, Windows Networking Engineer
**Smart App Control:** ENABLED on original host — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No `git reset`, `git clean`, stage, or commit. All existing Multinet modifications preserved.

---

## 1. Upstream Commit and Source Inventory

### Commit
`2b333a3ad820996ba337588adfa8f7259a45baec` — "Release 0.3.7 (#78)" (HEAD of `main`, cloned with `--depth 1`).

### Top-level layout

| Path | Purpose |
|---|---|
| `cmd/net-merge/` | CLI entry: `tray`, `run`, `on|off|quit`, `status`, `limit`, `socks`, `list`, `cleanup`, `autostart`, `install|uninstall`, `version` |
| `internal/engine/` | Scheduler, balancer (SOCKS5 + TCP/UDP relay), health probes, rate limits |
| `internal/nics/` | Adapter discovery, IP_UNICAST_IF egress forcing, per-interface byte counters |
| `internal/tun/` | Wintun virtual adapter, route add/delete, weak-host/forwarding control, stale cleanup |
| `internal/platform/` | Admin elevation, single-instance mutex, tray, autostart (scheduled task), install |
| `internal/tray/` | Systray menu, autopause, update check |
| `internal/state/` | Status file, limit requests/markers (file-based IPC between CLI and running instance) |
| `internal/logx/`, `internal/icon/` | Logging, tray icon |
| `scripts/` | Windows/macOS/Linux build scripts, Wintun fetcher + signature verification, smoke tests |
| `installer/`, `packaging/` | Platform installers |
| `nix/`, `.github/` | Nix flake, CI |

### Go dependency manifest (`go.mod`)

Module: `net-merge`, Go 1.27.

Direct dependencies:
- `fyne.io/systray v1.12.2` — tray icon
- `github.com/vishvananda/netlink v1.3.1` — Linux link/route/address management
- `github.com/xjasonlyu/tun2socks/v2 v2.7.0` — userspace TUN→proxy tunnel
- `golang.org/x/net v0.56.0`, `golang.org/x/sys v0.46.0` — stdlib extensions
- `golang.zx2c4.com/wireguard v0.0.0-20260522210424-ecfc5a8d5446` — Wintun bindings (`golang.zx2c4.com/wintun` indirect)
- `gvisor.dev/gvisor v0.0.0-20260701204157-69c2d17aea96` — userspace TCP/IP stack

Indirect: `godbus/dbus`, `google/btree`, `google/uuid`, `vishvananda/netns`, `uber/atomic`, `uber/multierr`, `uber/zap`, `golang.org/x/exp`, `golang.org/x/time`.

### License
**MIT** (Copyright 2026 Rohit Mahto). Standard MIT text; no copyleft. Attribution must be preserved in all copies/substantial portions.

Wintun (`golang.zx2c4.com/wintun`) is a separate prebuilt, signed DLL from WireGuard LLC — fetched at build/install time by `scripts/fetch-wintun.ps1` (v0.14.1, SHA-256 pinned, Authenticode signature verified against "WireGuard LLC"). Its own license applies to the DLL binary, not the Go source.

---

## 2. Current Multinet Inspection

### Crates inspected (actual working-tree state)

| Crate | Key files | State |
|---|---|---|
| `network-core` | `src/lib.rs` | `InterfaceDiscovery` trait + `NetworkPlatform` async trait (mostly `NotImplemented` stubs), `NetworkInterface`, `InterfaceMetrics`, `RouteInfo`, `MockNetworkPlatform`. No real Windows binding yet. |
| `interface-discovery` | `src/lib.rs`, `src/windows.rs` (untracked) | Real `GetAdaptersAddresses` enumeration on Windows; `classify_if_type`, `looks_virtual`, `is_physical_link`. |
| `network-monitor` | `src/lib.rs` | Real `GetIfTable2` counters via `windows-sys`; `RateTracker` computes honest per-interface bps rates (first sample = None). |
| `bonding-engine` | `src/lib.rs`, `path_manager.rs`, `scheduler.rs`, `socket.rs` (all untracked) | Packet-level WDRR scheduler, PathManager, BondedSocket, HMAC-SHA256 framing (tunnel-protocol). 19 tests written; execution blocked by Smart App Control on this host (4 failed pre-fix due to scheduler starvation; fix applied, post-fix execution still blocked). |
| `tunnel-protocol` | `src/lib.rs`, `src/framing.rs` (untracked) | HMAC-SHA256 authenticated framing + replay protection. 8 tests written; not execution-verified. |
| `relay/` | `bonding-server/`, `session-manager/`, `security-api/`, `speedtest-server/` | Reference relay implementations (not integrated with desktop yet). |
| `apps/desktop` | `src-tauri/src/main.rs`, `src-tauri/src/diagnostics.rs` (untracked), `src/*` (React/Tauri frontend) | Tauri 2 desktop app. IPC commands: traffic monitor (GetIfTable2 source), speed test, interface listing, incidents, security events, database status. `DiagnosticsState` owns a 1s traffic sampler writing to SQLite. |

### Overlapping functionality (Multinet vs Net Merge)

| Concern | Multinet (Rust) | Net Merge (Go) | Overlap |
|---|---|---|---|
| Adapter discovery | `interface-discovery` (IP Helper) + `network-core::InterfaceDiscovery` | `internal/nics` (`GetAdaptersAddresses` + `net.Interfaces`) | Both enumerate Windows adapters, filter virtual/tunnel, classify by IfType |
| Per-interface counters | `network-monitor` (`GetIfTable2`, `RateTracker`) | `internal/nics` `Octets()` (`GetIfEntry2Ex`) | Both read OS byte counters for per-adapter rates |
| Interface health | PathManager `infer_health` (loss/latency thresholds) | Engine `probe()` (TCP to 1.1.1.1/8.8.8.8 per family) | Both determine usable/dead paths |
| Weighting | `compute_weight()` (goodput × loss × RTT × jitter × queue) | Auto-weight by measured peak throughput / max peak | Both dynamically weight paths by measured capacity |
| Path failover | `schedule()` skips unhealthy/unavailable; recovery via `mark_recovered` | Scheduler tiers (up/unknown/down) + `Fail()` cooldown + probe rejoin | Both sideline dead adapters and recover them |
| Route/virtual IF | `NetworkPlatform` trait (stubs: `create_virtual_interface`, `configure_routes`, `restore_network`) | Wintun adapter + `route add 0.0.0.0/1,128.0.0.0/1` + strong-host control | Net Merge is fully implemented; Multinet is stubbed |
| Per-path pinning | `BondedSocket.send()` → scheduler-selected `UdpSocket.send_to()` | `ForceEgress` (IP_UNICAST_IF=31) + `LocalAddr` bind | Both pin flows to specific adapters |
| Framing/auth | HMAC-SHA256 per-packet (tunnel-protocol) | None (relies on underlying TLS) | Multinet has auth; Net Merge does not at packet level |
| IPC/dashboard | Tauri commands + SQLite | File-based state (`state/` package) + CLI | Both expose status to a UI/CLI |

### Fundamental difference (must not be confused)

- **Net Merge: connection-level load balancing.** Each new TCP/UDP *flow* is assigned to one adapter; a single flow never spans adapters. Good for multi-download aggregation; a single long-lived TCP flow uses exactly one link.
- **Multinet bonding-engine: packet-level UDP bonding.** Packets of one logical flow are split across paths via WDRR and reassembled at the cloud peer. Designed for single-flow aggregation.

These are complementary, not interchangeable.

---

## 3. Architecture Comparison

### Option A — Separate Net Merge Go service controlled by MultiNet

Multinet launches/net-merge as a child (or auto-started) Go process. MultiNet talks to it over a narrow, authenticated local IPC (Windows named pipe, versioned JSON). Net Merge owns the Wintun adapter, routes, and all per-interface sockets. Multinet's Rust bonding engine remains for cloud bonding sessions (Mode B).

| Factor | Assessment |
|---|---|
| Reuse | High — Net Merge's discovery, counters, Wintun, routes, probes, scheduler all reused as-is |
| Engineering effort | Low — IPC boundary only; no porting |
| Maintenance | Low — upstream Go code stays upstream; Multinet owns only the IPC shim |
| Security | Net Merge binary runs elevated (Wintun/routes need admin); IPC must be authenticated and restricted to localhost/named pipe; Tauri must NOT be elevated |
| Windows integration | Native Go Windows code; Wintun DLL signed by WireGuard LLC; smart-app-control risk on unsigned Go binaries (must verify) |
| Performance | Proven (~0.18 CPU cores per 100 Mbit/s per upstream README) |
| Process isolation | High — a crash in Net Merge cannot corrupt Multinet's Rust heap |
| Testing | Upstream has unit + smoke tests; Multinet adds IPC contract tests |
| Licensing | MIT — compatible; attribution preserved |

### Option B — Forked Net Merge networking daemon with dedicated local IPC

Same as A but Multinet forks the Go repo into its own tree, builds a custom `net-merge` binary with Multinet branding and a dedicated IPC socket, and ships it as a component.

| Factor | Assessment |
|---|---|
| Reuse | High (forked) |
| Engineering effort | Medium — fork maintenance, build pipeline, signing |
| Maintenance | Medium — must rebase on upstream for fixes |
| Security | Same as A, plus Multinet controls signing of its own build |
| Windows integration | Same as A |
| Performance | Same as A |
| Process isolation | High |
| Testing | Same as A plus fork-regression tests |
| Licensing | MIT — fork must preserve copyright + license + third-party notices |

### Option C — Port Net Merge functionality into Rust

Rewrite discovery, Wintun, routes, gVisor/tun2socks, scheduler, probes in Rust inside Multinet crates.

| Factor | Assessment |
|---|---|
| Reuse | None — full rewrite |
| Engineering effort | Very high — gVisor equivalent does not exist in Rust; Wintun bindings must be written; route/netsh integration re-implemented |
| Maintenance | Very high — Multinet owns all Windows networking code |
| Security | Multinet controls all code, but must implement correctness-critical Windows networking from scratch |
| Windows integration | Must be built; no existing Wintun Rust bindings in the workspace |
| Performance | Unknown until built |
| Process isolation | Same process — a panic can affect the whole Rust bonding engine |
| Testing | Must write everything |
| Licensing | No upstream obligation (clean room), but nothing reused |

### Recommended direction

**Option A** — a separate Net Merge Go service controlled by MultiNet over a narrow authenticated local IPC (Windows named pipe, versioned JSON contracts). This is the stated preferred direction. Do not force a Go-to-Rust rewrite without a demonstrated technical benefit; there is none for Mode A.

---

## 4. Two Distinct Networking Modes

### MODE A: Multi-WAN Load Balancing (Net Merge)

Uses Net Merge functionality for:
- Three physical adapters: Main Wi-Fi, USB tethering, external USB Wi-Fi (second phone)
- Wintun virtual adapter + capture routes (`0.0.0.0/1`, `128.0.0.0/1`)
- gVisor user-space TCP/IP stack + tun2socks
- Connection-level distribution (weighted least-busy-flows)
- Dynamic per-interface weighting (auto by measured throughput, or static `-weight`)
- Health monitoring (TCP probes to 1.1.1.1/8.8.8.8 per family; down cooldown)
- Automatic recovery (rejoin within seconds)
- Route cleanup on Off/crash (`delRoutes`, `CleanupStale`, strong-host restore)

This is **connection-level distribution**, not packet-level bonding. A single TCP flow uses one link; multiple concurrent flows spread across links.

### MODE B: True Multinet UDP Bonding (Rust engine)

Retains the existing Rust bonding engine for:
- Authenticated multi-path sessions (HMAC-SHA256 framing, replay protection)
- Packet-level scheduling (WDRR)
- UDP packet splitting across paths
- Cloud packet reassembly (peer-side, in relay/bonding-server)
- Bidirectional forwarding
- Single-flow aggregation (one flow spread across all paths)
- Tunnel failover between paths

### Boundary rule

Do not confuse Mode A (connection-level, Net Merge) with Mode B (packet-level, Multinet Rust). They solve different problems and can operate simultaneously on different traffic classes: e.g., browser/download traffic via Net Merge (Mode A), and a dedicated cloud-bonded session via the Rust engine (Mode B).

---

## 5. Integration Boundary

### Management API design (MultiNet → Net Merge)

A stable, versioned local IPC over a **Windows named pipe** (`\\.\pipe\multinet-netmerge-v1`):

| Message | Direction | Payload |
|---|---|---|
| `discover` | M→NM | `{}` |
| `discover.ok` | NM→M | `{ adapters: [{name, index, ip4, ip6, weight, skipped, reason}] }` |
| `start` | M→NM | `{ adapters: [...], weights: {...}, autoWeight: bool, ipv6: "auto|on|off", mtu: int }` |
| `started` | NM→M | `{ pid, mode, v6State, since }` |
| `stop` | M→NM | `{}` |
| `stopped` | NM→M | `{ ok }` |
| `status` | M→NM | `{}` |
| `status.ok` | NM→M | `{ on, adapters: [{name, v4, v6, state, weight, active, total, failed, bytesIn, bytesOut, rateIn, rateOut, linkIn, linkOut, limitMbps}] }` |
| `limit` | M→NM | `{ adapter, mbitPerSec }` |
| `limit.applied` | NM→M | `{ adapter, mbitPerSec }` |
| `event` | NM→M | `{ type: "adapter-up|adapter-down|recover|route-error|crash-recovery", adapter, ts }` |

### Key design constraints

- **Single owner for virtual adapter + route lifecycle:** Only the Net Merge process creates/deletes the Wintun adapter and capture routes. Multinet never calls `netsh`/`route`/`CreateIpForwardEntry` directly. Prevents conflicting Wintun instances and route fights.
- **Privileged operations in a dedicated restricted service:** Net Merge requires admin for Wintun/routes/IP_UNICAST_IF. It runs elevated (UAC) as a separate process. Multinet's Tauri frontend runs unprivileged and talks only over the authenticated named pipe.
- **Tauri must not receive unrestricted administrator privileges:** Tauri remains a normal user process. All admin actions flow through the Net Merge service via IPC. The named pipe ACL restricts connect to the invoking user.
- **No multiple conflicting Wintun instances:** Single-instance mutex (`Global\net-merge` in upstream; Multinet uses a distinct name e.g. `Global\multinet-netmerge-v1`) plus a health check before start.

### Multinet-side consumer

A new Rust module (e.g. `crates/netmerge-client` or inside `bonding-engine`/`network-core`) owns the named-pipe client, marshals JSON, and exposes `NetMergeEngine: Start/Stop/Status/SetLimit/Discover` to Tauri IPC. Tauri commands are thin wrappers.

---

## 6. Windows Security Review

### Smart App Control (SAC)

- SAC **remains ENABLED** on the original host. No weakening, no bypass, no unsigned binary execution.
- Net Merge is **Go**. A freshly built, unsigned `net-merge.exe` will be blocked by SAC (error 4551) just as Rust test binaries were. This must be verified in the approved dev/test environment before runtime acceptance.
- Acceptable mitigations (without disabling SAC): code-sign the Net Merge binary with an internal/enterprise certificate trusted by the machine policy, or add the binary's path/publisher to the allowed list via policy (admin/machine-owner decision — not modified during this assessment).
- The Wintun DLL must be obtained per `scripts/fetch-wintun.ps1`: SHA-256 pinned (`07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51` for v0.14.1) and Authenticode signature verified as signed by "WireGuard LLC". Do not substitute an unverified DLL.

### Privilege isolation

- Net Merge needs elevation for: Wintun adapter creation, `route add`/`delete`, `netsh` interface config, `IP_UNICAST_IF`, weak-host/forwarding toggles.
- It re-launches itself elevated via `ShellExecute(runas)` when `run` is invoked without admin.
- Multinet's Tauri app must remain unelevated; it never holds admin privileges and never calls privileged Windows APIs directly.

### Network safety

- Capture routes (`0.0.0.0/1`, `128.0.0.0/1`) take precedence over the default route without replacing it; `Off()` deletes them, restoring normal routing.
- `startStrongHost` disables weak-host-send and forwarding on participating adapters while capture is on (prevents routing loops where pinned sockets flow back into the TUN), and `stopStrongHost`/`restoreStrongHost` put settings back — including after a crash via a temp-file marker.
- IPv6 is best-effort; when not all adapters have IPv6, IPv6 flows are refused so apps fall back to IPv4 and spread across all adapters.

---

## 7. Licensing Report

| Component | License | Attribution required? | Notes |
|---|---|---|---|
| Net Merge (Go source) | MIT (Rohit Mahto, 2026) | Yes — preserve copyright + license text in copies/substantial portions | Fork/service reuse must include LICENSE and copyright header |
| Wintun DLL (WireGuard LLC) | Wintun/GPLv2 + MIT dual (binary distribution) | Yes — ship `wintun-LICENSE.txt` alongside DLL | Not part of this repo; fetched at install time; verify signature |
| gVisor | Apache 2.0 | Yes | Dependency of Net Merge; include in SBOM |
| tun2socks | MIT | Yes | Dependency of Net Merge |
| wireguard-go / wintun bindings | MIT / BSD-style | Yes | Indirect dependency |
| Multinet existing code | Proprietary | — | Keep separate; do not mix into Net Merge source tree |

### SBOM action
Create `docs/licensing/NET_MERGE_SBOM.md` listing: net-merge (MIT), gVisor (Apache 2.0), tun2socks (MIT), wireguard/wintun (verify), fyne/systray, vishvananda/netlink, golang.org/x/* — with version pins from `go.mod`/`go.sum`. Preserve the upstream LICENSE file verbatim wherever Net Merge source is distributed.

---

## 8. Hardware Verification Plan

### Target hardware (MODE A — three physical adapters)

1. **Main Wi-Fi** (built-in laptop Wi-Fi)
2. **USB tethering** (Android phone USB tether)
3. **External USB Wi-Fi adapter** connected to a second phone hotspot

### Required tests (none may be skipped; single-NIC smoke test is NOT proof of three-link aggregation)

| Test | Mode | Success criterion |
|---|---|---|
| Wi-Fi 1 alone | A | Internet via Net Merge; adapter shows v4 up |
| USB tether alone | A | Internet via Net Merge; adapter shows v4 up |
| Wi-Fi 2 alone | A | Internet via Net Merge; adapter shows v4 up |
| All three active (parallel downloads) | A | Combined throughput > fastest single link; each adapter carries flows |
| Individual path disconnection | A | Disconnected adapter stops receiving flows within probe interval; traffic continues on others |
| Connection recovery | A | Reconnected adapter rejoins and receives flows within seconds |
| DNS over each adapter | A | Name resolution works while each is the only active path |
| IPv4 | A | All traffic flows over IPv4 capture routes |
| IPv6 limitations | A | When not all adapters have IPv6, IPv6 is refused (V6Auto); apps use IPv4 |
| Route restoration | A | After `off`/crash, `128.0.0.0` route removed; normal default route restored; weak-host settings put back |
| Application traffic distribution | A | Multiple concurrent downloads spread across adapters per scheduler weights |

### Mode B hardware test (post-M1.1, separate)

Three-path UDP bonding to the cloud peer; single-flow aggregation; not part of NM-01 acceptance.

**Do not treat Net Merge's single-NIC Windows smoke test (`scripts/windows-smoke.ps1`) as proof of three-link aggregation.** It runs on one NIC and checks TUN/routes/probe/relay/off only.

---

## 9. Performance Verification

Tests to run (all on real three-link hardware; label each by mode):

| Scenario | Mode | Metric |
|---|---|---|
| Fastest individual link (single download) | A | Throughput, latency, loss |
| Net Merge, two connections (parallel downloads) | A | Combined throughput, per-adapter split |
| Net Merge, three connections (parallel downloads) | A | Combined throughput vs. sum of links; CPU, memory |
| Multinet true bonding (one long-lived TCP flow) | B | Single-flow throughput vs. single link; only after M1.1 accepted |

Record: throughput (Mbit/s), latency (ms), loss (%), CPU (%), memory (MB). Label each result explicitly as Mode A or Mode B. Upstream README cites ~0.18 CPU cores per 100 Mbit/s and 232–303 Mbit/s on Wi-Fi + USB tether + iPhone hotspot (fastest single link ~121 Mbit/s) — treat as vendor claim, verify on actual hardware.

---

## 10. Testing Strategy

### Unit tests
- Multinet Rust side: existing `bonding-engine` (19) and `tunnel-protocol` (8) tests — execution blocked on this host by SAC; move to approved environment to confirm.
- Net Merge: upstream `internal/engine/*_test.go`, `internal/nics/nics_test.go`, `internal/tun/tun_test.go`, `internal/tray/*_test.go` — run with `go test ./...` in the approved environment.

### IPC contract tests
- Multinet Rust named-pipe client → Net Merge Go server: discover/start/stop/status/limit/event roundtrips; malformed JSON handling; version mismatch; pipe disconnect/reconnect.

### Integration tests
- Three-link hardware test plan (section 8).
- Crash recovery: kill Net Merge process; confirm routes cleaned by `CleanupStale`/next start; confirm weak-host settings restored.
- SAC verification: confirm signed Net Merge binary executes with SAC enabled.

### What is NOT tested in NM-01
- No claim that Net Merge or Multinet bonding is operational without real three-link Windows hardware verification.
- No Mode B hardware test until M1.1 is accepted.

---

## 11. Performance Benchmarks Available

- Upstream scheduler: weighted least-busy-flows per address family; auto-weights from smoothed peak throughput; per-adapter speed limits via token buckets.
- Multinet scheduler: WDRR with dynamic weights from goodput/loss/RTT/jitter/queue (source written; execution verification blocked by SAC).
- No shared benchmark harness exists yet. NM-01 does not create one; performance numbers come from the hardware verification plan (section 9).

---

## 12. Recommended Integration Decision

**Adopt Option A:** Run Net Merge as a separate, elevated Go service (`net-merge run` / tray), controlled by Multinet over an authenticated versioned local named-pipe IPC. Multinet's Rust bonding engine remains for Mode B (true packet-level UDP bonding). The two modes operate on separate traffic classes.

**Do not:**
- Port Net Merge into Rust (Option C) — no demonstrated benefit; very high effort; gVisor/Wintun/replace would have to be rebuilt.
- Let Tauri run elevated or call `netsh`/`route`/`Wintun` directly.
- Run unsigned Net Merge binaries with SAC enabled — verify signing/policy first.
- Treat a single-NIC smoke test as three-link aggregation proof.

---

## 13. Implementation Roadmap

| Phase | Work | Owner | Gate |
|---|---|---|---|
| NM-01 (this) | Architecture assessment; upstream inspection; Multinet overlap analysis | Architect | This report reviewed |
| NM-02 | Build signed Net Merge binary; verify SAC execution on approved Windows VM; fetch+verify Wintun DLL | Build engineer | Binary runs with SAC on |
| NM-03 | Implement Multinet named-pipe IPC client (Rust) + Net Merge shim (Go) with v1 JSON contracts | Rust/Go engineers | IPC contract tests pass |
| NM-04 | Wire Tauri commands → NetMergeEngine (discover/start/stop/status/limit/events) | Tauri/Rust engineer | UI shows per-adapter status |
| NM-05 | Three-link hardware test (Mode A): all paths active, disconnection, recovery, DNS, IPv4/IPv6, route restoration | QA + hardware | All section-8 tests pass |
| NM-06 | Performance benchmarks Mode A (two vs three connections) vs. fastest link | QA | Numbers recorded, labeled |
| NM-07 | Licensing SBOM (`docs/licensing/NET_MERGE_SBOM.md`); attribution preserved | Release engineer | SBOM published |
| NM-08 | Mode B hardware test (Multinet Rust bonding) — only after M1.1 accepted | QA | Not started |

**Stop condition:** Do not begin integration coding until this assessment is reviewed. Do not claim Net Merge or Multinet true bonding is operational without real Windows three-link hardware verification.

---

## Attestation

This assessment is based on direct inspection of the Net Merge repository at commit `2b333a3ad820996ba337588adfa8f7259a45baec` (cloned to the temp opencode directory) and the current Multinet working tree at `D:\git\multinet-cybershield-ai`. No code was copied, ported, or rewritten. No Multinet modifications were discarded, staged, or committed. `D:\git\browser` was never touched. Smart App Control remained enabled throughout; no Windows security policy was weakened. No results were fabricated; where execution could not be observed (SAC-blocked Rust test binaries, unsigned Go binaries), that limitation is stated explicitly.
