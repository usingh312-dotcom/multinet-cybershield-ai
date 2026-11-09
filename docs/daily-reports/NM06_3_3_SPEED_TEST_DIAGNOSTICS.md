# NET MERGE INTEGRATION — NM-06.3.3 SPEED TEST & DIAGNOSTICS REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Systems Engineer, QA Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No staging, commit, reset, or clean. All existing modifications preserved.

---

## 1. Executive Summary

MultiNet CyberShield AI includes two distinct measurement systems:

| System | Purpose | Location | Status |
|---|---|---|---|
| Speed Test | Download/upload/latency measurement against deployed speed test service | `crates/speed-test` | Implemented, tests blocked by SAC |
| Traffic Monitor | Real per-interface byte counters and throughput rates | `crates/network-monitor` + `apps/desktop/src-tauri/src/diagnostics.rs` | Implemented, tests blocked by SAC |
| Interface Discovery | Windows network adapter enumeration via IP Helper API | `crates/interface-discovery` | Implemented, tests blocked by SAC |

**Critical finding:** All diagnostics code is implemented but not execution-tested. Go is not installed (cannot compile/test Go service), and Smart App Control blocks Rust test binary execution (error 4551).

---

## 2. Speed Test Client (`crates/speed-test`)

### Architecture

The speed test client measures download/upload/latency against a deployed speed test service (HTTPS endpoint).

### Configuration (`SpeedTestConfig`)

| Field | Type | Default | Constraint |
|---|---|---|---|
| `server_url` | String | `https://127.0.0.1:9443` | Must use HTTPS |
| `interface_local_ip` | Option<IpAddr> | None | Socket bound to this address |
| `download_bytes` | u64 | 16 MB | 1..=64 MB |
| `upload_bytes` | u64 | 8 MB | 1..=64 MB |
| `ping_samples` | u32 | 12 | 1..=50 |
| `timeout_secs` | u64 | 30 | 1..=300 |
| `allow_self_signed_loopback` | bool | true | Only for loopback servers |

### Test stages

| Stage | Description |
|---|---|
| `Preparing` | Opening socket on selected interface |
| `Latency` | Sending latency probes (ping samples) |
| `Download` | Downloading measurement payload |
| `Upload` | Uploading measurement payload |
| `Finalizing` | Writing verified result |
| `Complete` | Test finished successfully |
| `Cancelled` | Test cancelled by user |
| `Failed` | Test failed with error |

### Measurement integrity

| Metric | Source | Integrity |
|---|---|---|
| Download MBps | `received_bytes / download_secs` | Actual bytes received / actual time |
| Upload MBps | `config.upload_bytes / upload_secs` | Actual bytes sent / actual time |
| Ping MS | `sum(rtts) / count` | Actual TCP connect times |
| Jitter MS | `sum(abs(rtt[i] - rtt[i-1])) / (count - 1)` | Successive RTT differences |
| Loss PCT | `failures / ping_samples` | Failed probes / total probes |
| Server observed IP | Server response | From server JSON |
| Interface local IP | Config | Socket bound to this address |

### Validation rules

| Check | Constraint |
|---|---|
| Server URL | Must start with `https://` |
| Server URL length | ≤ 512 characters, no whitespace, no `@` |
| Self-signed certs | Only allowed for loopback servers |
| Download bytes | 1..=64 MB |
| Upload bytes | 1..=64 MB |
| Ping samples | 1..=50 |
| Timeout secs | 1..=300 |

### Speed test result persistence

Results are stored in SQLite database table `speed_test_results` with columns:

| Column | Type |
|---|---|
| `id` | TEXT |
| `server_url` | TEXT |
| `server_name` | TEXT |
| `server_observed_client_ip` | TEXT |
| `interface_id` | TEXT |
| `interface_local_ip` | TEXT |
| `download_mbps` | REAL |
| `upload_mbps` | REAL |
| `ping_ms` | REAL |
| `jitter_ms` | REAL |
| `loss_pct` | REAL |
| `download_bytes` | INTEGER |
| `upload_bytes` | INTEGER |
| `ping_samples` | INTEGER |
| `ping_received` | INTEGER |
| `duration_ms` | INTEGER |
| `started_at` | TEXT |
| `completed_at` | TEXT |
| `created_at` | TEXT |

### Retention policy

| Metric | Retention |
|---|---|
| Speed test results | Last 200 results |
| Traffic samples | Last 24 hours |

---

## 3. Traffic Monitor (`crates/network-monitor` + `diagnostics.rs`)

### Architecture

```
Windows IP Helper (GetIfTable2)
       │
       ▼
RateTracker (per-interface byte counters → bps rates)
       │
       ▼
DiagnosticsState (1s sampler loop)
       │
       ▼
SQLite database (traffic_samples table)
       │
       ▼
Tauri IPC commands (get_traffic_snapshot, get_traffic_history)
       │
       ▼
React frontend dashboard
```

### Interface sample fields

| Field | Type | Description |
|---|---|---|
| `interface_index` | u32 | Windows interface index |
| `name` | String | Adapter name |
| `description` | String | Adapter description |
| `if_type` | u32 | Windows IfType value |
| `is_up` | bool | Operational status |
| `in_octets` | u64 | Cumulative RX bytes |
| `out_octets` | u64 | Cumulative TX bytes |
| `rx_bps` | Option<f64) | Receive rate (None for first sample) |
| `tx_bps` | Option<f64) | Transmit rate (None for first sample) |
| `rx_delta_bytes` | u64 | Bytes since last sample |
| `tx_delta_bytes` | u64 | Bytes since last sample |
| `counter_reset` | bool | Counter moved backwards |

### Rate tracker behavior

| Scenario | Behavior |
|---|---|
| First sample | `rx_bps = None`, `tx_bps = None`, `counter_reset = false` |
| Normal counter increment | `rx_bps = delta_bytes * 8 / seconds` |
| Counter reset (decrease) | `rx_bps = Some(0.0)`, `tx_bps = Some(0.0)`, `counter_reset = true` |
| Zero elapsed time | `rx_bps = None`, `tx_bps = None` |
| Adapter disappears | Forgotten from tracker; re-added cleanly on reappear |

### Traffic snapshot aggregate

| Field | Type | Description |
|---|---|---|
| `sampled_at` | String | RFC 3339 timestamp |
| `source` | String | "GetIfTable2" |
| `monitor_active` | bool | Sampler running state |
| `interfaces` | Vec<TrafficInterfaceDto> | Per-interface samples |
| `physical_rx_bps` | Option<f64) | Aggregate physical RX rate |
| `physical_tx_bps` | Option<f64) | Aggregate physical TX rate |
| `session_rx_bytes` | u64 | Session RX bytes (physical only) |
| `session_tx_bytes` | u64 | Session TX bytes (physical only) |
| `total_rx_bytes` | u64 | Cumulative RX bytes (physical only) |
| `total_tx_bytes` | u64 | Cumulative TX bytes (physical only) |
| `peak_combined_bps` | f64 | Peak combined rate from DB |

---

## 4. Interface Discovery (`crates/interface-discovery`)

### Discovery method

Windows IP Helper API (`GetAdaptersAddresses`) with flags:
- `GAA_FLAG_INCLUDE_GATEWAYS`
- `GAA_FLAG_SKIP_ANYCAST`
- `GAA_FLAG_SKIP_MULTICAST`
- `GAA_FLAG_SKIP_DNS_SERVER`

### Interface classification (`classify_if_type`)

| IfType | Value | Classification |
|---|---|---|
| Ethernet | 6 | Ethernet |
| IEEE 802.11 Wi-Fi | 71 | Wifi |
| Software loopback | 24 | Loopback |
| Tunnel | 131 | Tunnel |
| PPP | 23, 108 | Ppp |
| Other | _ | Other |

### Physical adapter detection

| Criterion | Requirement |
|---|---|
| IfType | Ethernet (6) or Wi-Fi (71) |
| Not virtual | `looks_virtual()` returns false |
| MAC address | Length ≥ 6 bytes |

### Virtual adapter keywords

`virtual`, `hyper-v`, `vethernet`, `vmware`, `virtualbox`, `vbox`, `npcap`, `winpcap`, `loopback`, `tap-`, `tap adapter`, `openvpn`, `wireguard`, `teredo`, `isatap`, `6to4`, `pseudo`, `wan miniport`, `wi-fi direct`, `wireless direct`, `kernel debug`, `zerotier`, `tailscale`, `hamachi`, `docker`, `wsl`, `container`, `vpn`

---

## 5. Tauri Diagnostics IPC Commands

| Command | Parameters | Returns | Privilege |
|---|---|---|---|
| `get_traffic_monitor_status` | none | `TrafficMonitorStatusDto` | None |
| `set_traffic_monitor_running` | `running: bool` | `TrafficMonitorStatusDto` | None |
| `get_traffic_snapshot` | none | `TrafficSnapshotDto` | None (reads DB) |
| `get_traffic_history` | `interface_id?: String`, `seconds: u32` | `Vec<TrafficSampleDto>` | None (reads DB) |
| `list_network_interfaces` | none | `Vec<InterfaceDto>` | None |
| `start_speed_test` | `StartSpeedTestRequest` | `String` (test ID) | None |
| `get_speed_test_status` | `id: String` | `SpeedTestStatusDto` | None |
| `cancel_speed_test` | `id: String` | `()` | None |
| `get_speed_test_history` | none | `Vec<SpeedTestHistoryDto>` | None (reads DB) |

### Traffic monitor status

| Field | Type | Description |
|---|---|---|
| `running` | bool | Sampler running state |
| `last_sample_age_ms` | Option<u64> | Age of last sample |
| `source` | &'static str | "GetIfTable2" |

---

## 6. Attestation

This report is based on direct source inspection. Speed test and diagnostics code is implemented but not execution-tested (Go not installed, SAC blocks Rust test binaries). All measurement integrity claims are based on source code review, not executed measurements. No results are fabricated.
