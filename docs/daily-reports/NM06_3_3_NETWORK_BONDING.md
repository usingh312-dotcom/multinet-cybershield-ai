# NET MERGE INTEGRATION — NM-06.3.3 NETWORK BONDING REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Windows Systems Architect, Senior Go Engineer, Rust Systems Engineer, Windows Networking Engineer
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No staging, commit, reset, or clean. All existing modifications preserved.

---

## 1. Executive Summary

MultiNet CyberShield AI implements two distinct networking modes:

| Aspect | Mode A (Net Merge) | Mode B (Rust bonding engine) |
|---|---|---|
| **Mechanism** | Connection-level load balancing | Packet-level UDP bonding |
| **Scope** | All system traffic via capture routes | Dedicated authenticated sessions |
| **Single flow** | Uses exactly one link | Split across all paths |
| **Best for** | Downloads, browsers, updates, torrents | Cloud bonding, single-flow aggregation |
| **Owner** | Elevated Go service | Unelevated Rust process |
| **Wintun/routes** | Yes, owned exclusively | No |
| **Authentication** | None (relies on underlying TLS) | HMAC-SHA256 per-packet |
| **Mutually exclusive?** | **Yes** — kernel mutex enforces | **Yes** — kernel mutex enforces |

**Critical finding:** Mode A and Mode B cannot be active simultaneously. The kernel named mutex `Global\multinet-netmerge-mode-owner-v1` ensures exclusive ownership. No source-level defect was found in the mode exclusion logic. However, zero runtime verification has been performed.

---

## 2. Mode A: Connection-Level Multi-WAN Load Balancing

### Architecture

Net Merge runs as an elevated Go service that:

1. **Discovers physical adapters** via `nics.DiscoverNicsDetailed()` — uses `GetAdaptersAddresses` + `net.Interfaces`, filters out virtual/tunnel/VPN adapters, keeps adapters with default gateways
2. **Creates Wintun virtual adapter** named `net-merge` with fixed GUID `{3f0d5a7e-9b1c-4e52-8a47-1c6b2d9e5f30}`
3. **Installs capture routes** `0.0.0.0/1` and `128.0.0.0/1` — these take precedence over the default route without replacing it
4. **Runs gVisor userland TCP/IP stack** + tun2socks — all captured flows are re-dialed through pinned NICs
5. **Pins each flow** via `IP_UNICAST_IF` (socket option 31) + `LocalAddr` bind — prevents routing loops where pinned sockets flow back into the TUN
6. **Balances new flows** via weighted least-busy-flows scheduler — each new TCP/UDP connection assigned to one adapter
7. **Probes internet** per adapter/family via TCP to 1.1.1.1/8.8.8.8 — dead adapters sidelined with cooldown
8. **Auto-weights adapters** by measured peak throughput — faster links get more connections
9. **Disables weak-host-send + forwarding** on participating adapters while capture is on — prevents routing loops
10. **Restores settings** on stop/crash — `stopStrongHost()`/`restoreStrongHost()`

### Scheduler algorithm

```
weight = peak_throughput / max_peak_throughput  (floored at 0.05)
score = (active_flows + 1) / weight
select path with lowest score
```

- **Auto mode:** Weights derived from smoothed peak throughput measurements
- **Static mode:** Weights from static `-weight` configuration
- **Tiers:** Probed up (tier 0) > not probed yet (tier 1) > probe failed/cooldown (tier 2)
- Only the best non-empty tier is used; if all NICs look dead, flows still go somewhere
- Random start for tie-breaking prevents resonance with arrival patterns

### Per-path accounting

| Metric | Source | Notes |
|---|---|---|
| Active flows | `slots.active` | Atomic counter |
| Total flows | `slots.total` | Atomic counter |
| Failed flows | `slots.failed` | Atomic counter |
| Bytes in/out | `slots.bytesIn/Out` | Atomic counters |
| Rate in/out | `slots.rateIn/Out` | Bytes/s over last tick |
| Link rate in/out | `slots.linkRateIn/Out` | From OS `GetIfEntry2Ex` counters |
| Peak | `slots.peak` | Smoothed max recent rate |
| Weight | `slots.weight` | Dynamic (auto) or static |
| Limit Mbps | `slots.limitMbps` | Token bucket cap |

---

## 3. Mode B: Packet-Level UDP Bonding

### Architecture

The Rust bonding engine (`crates/bonding-engine`) implements:

1. **Weighted Deficit Round-Robin (WDRR) scheduler** — each round adds `weight` to each eligible path's `deficit`; selects path with `deficit >= packet_size` and largest deficit; reduces deficit by `packet_size`
2. **Dynamic weights** — `weight = goodput × loss_factor × latency_factor × jitter_factor × queue_factor`
   - `goodput` from `GoodputEstimator` over cumulative `acked_bytes` (never PHY rate)
   - `loss_factor = 1/(1+loss_pct)`
   - `latency_factor = 1/(1+rtt_ms/100)`
   - `jitter_factor = 1/(1+jitter_ms)`
   - `queue_factor = 1/(1+queue_depth/queue_capacity)`
3. **Path skips** — `!health.usable()`, `queue_full()`, `mtu < packet_size`
4. **Per-path accounting** — `record_transmission()`, `record_acknowledgement()`, `SchedulerStats`

### Authentication framing (`tunnel-protocol/src/framing.rs`)

- **HMAC-SHA256** keyed by `FramingLayer::key` — covers `seq_no_le_bytes + payload`
- **Constant-time verification** via `Mac::verify_slice()` — no timing side-channel
- **32-byte auth tag** — full SHA-256 output
- **Replay protection** — `last_accepted_seq` tracking; `seq_no <= last_accepted_seq` rejected
- **Sequence wraparound** — `wrapping_add`; conservative replay check on wraparound

### Test coverage (framing.rs)

| Test | Purpose |
|---|---|
| `encapsulate_then_decapsulate_roundtrips` | Correct authentication roundtrip |
| `tampered_tag_is_rejected` | Flipped tag byte → TagMismatch |
| `tampered_payload_is_rejected` | Flipped payload byte → TagMismatch |
| `sequence_numbers_increment` | seq 0, 1, 2, ... |
| `oversized_payload_is_rejected` | MAX_PAYLOAD + 1 → FrameTooLarge |
| `replay_attack_is_rejected` | Same frame twice → ReplayDetected |
| `out_of_order_higher_seq_is_accepted` | Lower seq after higher → ReplayDetected |
| `sequence_wraparound_handled` | u64::MAX → 0 wraparound |

---

## 4. Mode A / Mode B Mutual Exclusion

### Enforcement points

| Layer | Mechanism | Status |
|---|---|---|
| Kernel mutex | `Global\multinet-netmerge-mode-owner-v1` | Source implemented |
| Go service `Start()` | Checks `mode.IsBonding()` before acquiring Wintun | Source implemented |
| Rust client `set_mode()` | Rejects A↔B conflicts | Source implemented |
| `BondedSocket::remove_path()` | Removes from sockets map + scheduler | Source implemented |

### Transition sequence

```
Mode A active → Mode B requested:
1. Rust client sends IPC set_mode_b(active=true)
2. Go service: engine.SetModeB(true, clientPid)
3. Coordinator: RequestModeB(rustPid) — acquires kernel mutex
4. If Mode A running: Stop() — removes capture routes, tears down TUN
5. Set mode = Bonding, store rustPid and rustHandle
6. MonitorRustProcess(): wait on process handle
7. Return {active:true, mode_b:true}

Mode B active → Mode A requested:
1. Tauri sends IPC start({...})
2. Go service: engine.Start(cfg)
3. Coordinator: RequestModeA() — checks mode.IsBonding() → reject
4. Return error: "cannot activate Net Merge while Mode B is active"
```

### Race condition handling

| Race scenario | Handling |
|---|---|
| Two concurrent Mode A requests | Mutex serializes; second gets "already active" |
| Two concurrent Mode B requests | Mutex serializes; second gets "already active" |
| Mode A start while Mode B active | Rejected: "cannot activate Mode A while Mode B is active" |
| Mode B start while Mode A active | Rejected: "cannot activate Mode B while Mode A is active" |
| Crash during Mode A | Kernel releases mutex; new instance can acquire |
| Crash during Mode B | Kernel releases mutex; `restoreStrongHost()` cleans up |

---

## 8. Network Interface Handling

### Interface discovery (`nics.DiscoverNicsDetailed`)

1. Enumerate all interfaces via `net.Interfaces()`
2. Filter: `net.FlagUp` set, `net.FlagLoopback` not set
3. Get adapter info via `GetAdaptersAddresses`:
   - Skip if `OperStatus != IfOperStatusUp`
   - Skip if `!physicalIfType(IfType)` (tunnels, PPP, loopback excluded)
   - Skip if description matches virtual keywords
4. Get addresses per interface:
   - IPv4: first non-link-local unicast
   - IPv6: first global unicast (not private, not link-local)
5. Apply gateway filter: if `!ai.v4`, set `ip4 = nil`; if `!ai.v6`, set `ip6 = nil`
6. Skip if no usable IP address

### Physical adapter classification

| IfType | Value | Classification |
|---|---|---|
| Ethernet | 6 | Physical |
| Ethernet (fast/gigabit) | 62, 69, 117 | Physical |
| IEEE 802.11 Wi-Fi | 71 | Physical |
| Mobile broadband (WWAN) | 243, 244 | Physical |
| Tunnel | 131 | Virtual (skipped) |
| PPP | 23, 108 | Virtual (skipped) |
| Loopback | 24 | Virtual (skipped) |
| Proprietary virtual (VPN) | 53 | Virtual (skipped) |

### Virtual adapter detection

| Keyword pattern | Examples |
|---|---|
| Name: `loopback|vethernet|wsl|tailscale|zerotier|wintun|...` | `vEthernet`, `wsl`, `tailscale` |
| Desc: `virtual|vpn|tap|tunnel|wintun|wireguard|...` | `VMware`, `OpenVPN`, `WireGuard` |

---

## 9. Attestation

This report is based on direct source inspection. Network bonding code is implemented but not execution-tested (Go not installed, SAC blocks Rust test binaries). Mode A/B mutual exclusion logic is sound on paper but not execution-verified. No results are fabricated.
