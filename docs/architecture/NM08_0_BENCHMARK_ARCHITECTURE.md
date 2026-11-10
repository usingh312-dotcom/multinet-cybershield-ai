# MultiNet CyberShield AI — Milestone NM-08.0
# Architecture Specification: MultiNet Benchmark Lab & Testing Framework

**Document ID:** `NM08_0_BENCHMARK_ARCHITECTURE`  
**Date:** 2026-11-10  
**Status:** APPROVED FOR IMPLEMENTATION  
**Author:** Principal Rust Networking Architect & Network Performance Engineer  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Milestone:** NM-08.0 (Competitive Benchmark Lab & End-to-End Bonding Qualification)

---

## 1. System Vision & Integrity Mandate

The **MultiNet Benchmark Lab** is an empirical measurement, verification, and competitive comparison platform for MultiNet CyberShield AI.

Its core tenet is **uncompromising measurement integrity**:
1. **Tier Segregation:** Synthetic simulations, local loopback tests, live single-interface tests, bonded end-to-end tests, and competitor imports exist in strictly partitioned evidence tiers. A metric from one tier must never be combined, compared without qualification, or substituted for another.
2. **Honesty over Claims:** If a capability is not implemented (e.g. bidirectional relay ingress, multipath packet reassembly, return path), the benchmark runner registers that test as `BLOCKED` with an explicit technical rationale. Unimplemented tests are never marked `PASS`.
3. **No Simulated WAN Results:** Synthetic scheduler tests measure algorithmic deficit round-robin and queue decisions; they must never be presented as physical WAN speed.
4. **Zero Host Network Disruption:** Tests do not mutate Windows routing tables, alter DNS settings, modify firewall policies, or bounce network adapters.
5. **Ethical Competitor Comparison:** Speedify data is imported via documented, manual or operator-provided CSV/JSON records under a strict fairness protocol. MultiNet does not scrape, inject into, or reverse-engineer competitor software.

---

## 2. High-Level Architecture

The Benchmark Lab is structured into a reusable Rust library (`crates/benchmark-core`), a standalone operator CLI (`crates/netbench-cli`), and a desktop dashboard view (`apps/desktop/src/pages/BenchmarkLab.tsx`).

```
+---------------------------------------------------------------------------------+
|                                User Interfaces                                  |
|   +------------------------------------+   +--------------------------------+   |
|   |         Tauri Desktop UI           |   |       netbench-cli (CLI)       |   |
|   |     (Diagnostics / Benchmark Lab)  |   | (run, baseline, compare, etc.) |   |
|   +-----------------+------------------+   +---------------+----------------+   |
+---------------------|--------------------------------------|--------------------+
                      |                                      |
                      +-------------------+------------------+
                                          |
+-----------------------------------------v---------------------------------------+
|                              crates/benchmark-core                              |
|                                                                                 |
|  +------------------------+  +--------------------------+  +-----------------+  |
|  |     Data Contract      |  |   Impairment Simulator   |  |   Competitor    |  |
|  | (RunManifest, Results, |  | (Delay, Loss, Jitter,    |  |  (Import, CSV,  |  |
|  |  Metrics, Provenance)  |  |  Reorder, Queue Depth)   |  | Comparability)  |  |
|  +-----------+------------+  +------------+-------------+  +--------+--------+  |
|              |                            |                         |           |
|  +-----------v----------------------------v-------------------------v--------+  |
|  |                             Measurement Providers                         |  |
|  |  +------------------+  +--------------------+  +----------------------+   |  |
|  |  |    Synthetic     |  |      Loopback      |  |    Live Interface    |   |  |
|  |  |    Scheduler     |  |    Socket & Wire   |  | (crates/speed-test)  |   |  |
|  |  +------------------+  +--------------------+  +----------------------+   |  |
|  |  +------------------+  +--------------------+                             |  |
|  |  |    Bonded E2E    |  |   Optional Iperf3  |                             |  |
|  |  |  (Reports BLOCKED|  |  (External Bounded |                             |  |
|  |  |   until relay)   |  |      Execution)    |                             |  |
|  |  +------------------+  +--------------------+                             |  |
|  +------------------------------------+--------------------------------------+  |
|                                       |                                         |
|  +------------------------------------v--------------------------------------+  |
|  |                           Report Generators                               |  |
|  |   manifest.json  |  results.json  |  summary.md  |  report.html (print)   |  |
|  +---------------------------------------------------------------------------+  |
+---------------------------------------------------------------------------------+
```

---

## 3. Evidence Tier Model

Every benchmark execution and recorded metric is tagged with an immutable `EvidenceTier`:

| Tier | Code | Execution Boundary | Purpose |
| :--- | :---: | :--- | :--- |
| **Synthetic** | `SYNTHETIC` | In-memory algorithms; mock or simulated interfaces; deterministic pseudorandom seed. | Validates scheduling proportions, deficit accounting, path-loss exclusion, and starvation prevention under controlled impairment. |
| **Loopback** | `LOOPBACK` | Local OS sockets (`127.0.0.1` / `::1`); actual packet serialization, framing, and mock receiver. | Validates framing overhead, HMAC verification, socket buffer throughput, and local pipe latency. |
| **Live Interface** | `LIVE_INTERFACE` | Single physical or virtual network interface (`WLAN`, `Ethernet`, `Cellular`) bound to specific local IP; live HTTPS transfer via `crates/speed-test`. | Measures baseline performance of an unbonded physical interface to establish the denominator for bonding efficiency. |
| **Bonded E2E** | `BONDED_E2E` | Multi-WAN egress across $\ge 2$ physical links to a remote relay server with return path. | **Currently BLOCKED.** MultiNet's remote relay and reassembly plane are pending. Registered as `BLOCKED` with detailed audit evidence. |
| **Competitor External** | `COMPETITOR_EXTERNAL` | Manually executed and imported measurements of third-party solutions (e.g. Speedify) run under documented protocol. | Enables apples-to-apples performance comparisons when strict environmental comparability conditions are met. |

---

## 4. Benchmark Data Contract & Schema (`schema_version: "1.0.0"`)

All outputs adhere to a strict versioned JSON schema. Ambiguous fields such as `speed` or `latency` are forbidden in favor of explicit units.

### 4.1 Schema Definition

```json
{
  "schema_version": "1.0.0",
  "run_id": "019315bc-8a42-7cf2-9e84-1849a99725f1",
  "timestamps": {
    "start_utc": "2026-11-10T10:00:00.000Z",
    "end_utc": "2026-11-10T10:00:05.120Z",
    "duration_ms": 5120
  },
  "metadata": {
    "app_version": "0.1.0-beta.1",
    "commit_sha": "a72566351a051b59e1807e9f8d671c7e0ce62fc9",
    "branch": "feature/nm-08.0-benchmark-lab",
    "os": "windows",
    "os_version": "Microsoft Windows 11 Enterprise (10.0.26100)",
    "architecture": "x86_64",
    "cpu_model": "Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz",
    "ram_total_bytes": 34267234304
  },
  "tier": "SYNTHETIC",
  "profile_name": "synthetic-3wan-asymmetric",
  "seed": 42,
  "tests": [
    {
      "test_id": "sched_deficit_3wan",
      "test_name": "Three-Path Asymmetric Scheduler Share",
      "tier": "SYNTHETIC",
      "status": "PASS",
      "reason": null,
      "metrics": {
        "duration_ms": 1000,
        "throughput_bps": 125000000,
        "throughput_mbps": 125.0,
        "latency_ms": 18.5,
        "jitter_ms": 2.1,
        "loss_pct": 0.0,
        "packets_sent": 10000,
        "packets_received": 10000,
        "packets_dropped": 0,
        "packets_reordered": 0,
        "bytes_sent": 15000000,
        "bytes_received": 15000000,
        "p50_latency_ms": 18.2,
        "p95_latency_ms": 22.4
      },
      "per_path_metrics": [
        {
          "path_id": "wan1_fiber",
          "interface_name": "eth0_sim",
          "packets_sent": 6000,
          "bytes_sent": 9000000,
          "packet_share_pct": 60.0,
          "byte_share_pct": 60.0
        },
        {
          "path_id": "wan2_starlink",
          "interface_name": "eth1_sim",
          "packets_sent": 3000,
          "bytes_sent": 4500000,
          "packet_share_pct": 30.0,
          "byte_share_pct": 30.0
        },
        {
          "path_id": "wan3_5g",
          "interface_name": "eth2_sim",
          "packets_sent": 1000,
          "bytes_sent": 1500000,
          "packet_share_pct": 10.0,
          "byte_share_pct": 10.0
        }
      ]
    }
  ]
}
```

---

## 5. Deterministic Impairment Simulator

Network impairment must be simulated deterministically without root privileges or OS-level packet filters (such as Linux `tc` or Windows kernel drivers).

The `ImpairmentSimulator` operates as a virtual channel:
1. **Bandwidth Throttling:** Token bucket rate limiter calculating virtual byte transmission allowances.
2. **Fixed Latency & Jitter:** Configurable delay queue based on seeded pseudo-random distribution:
   $$\text{delay} = \text{base\_latency} + \text{jitter\_sample}$$
3. **Packet Loss:** Uniform loss probability derived from PCG32/Xorshift random generator initialized with the test seed.
4. **Packet Duplication:** Configurable duplication percentage generating clone packets.
5. **Packet Reordering:** Out-of-order queue insertion that holds a fraction of packets for a randomized duration before delivery.
6. **Queue Capacity:** Maximum queue depth; packets exceeding depth are dropped and recorded as buffer overflow drops.

---

## 6. Measurement Providers

### 6.1 Synthetic Scheduler Provider
Reuses `crates/bonding-engine/src/scheduler.rs` (`WeightedPacketScheduler`) and `crates/bonding-engine/src/path_manager.rs` (`PathManager`).
- Simulates realistic multi-path networks:
  - Scenario 1: Single path baseline.
  - Scenario 2: Two equal paths (e.g. dual 100 Mbps, 15ms RTT).
  - Scenario 3: Three asymmetric paths (Fiber 500 Mbps/10ms, Starlink 120 Mbps/45ms, 5G 60 Mbps/30ms).
  - Scenario 4: Degraded path dynamic shedding.
  - Scenario 5: Path failure & recovery.

### 6.2 Single-Interface Live Provider
Reuses `crates/speed-test/src/lib.rs`.
- Validates that the requested `--interface-local-ip` actually exists on the Windows host and belongs to an active adapter.
- Binds `reqwest::Client` to the unicast interface IP.
- Executes download, upload, and latency phases against an HTTPS server.
- Enforces hard timeouts and cancellation tokens.
- Live network execution requires explicit `--enable-live-network-tests` and interactive confirmation.

### 6.3 Loopback Provider
- Binds actual OS UDP sockets on `127.0.0.1:0`.
- Executes `crates/tunnel-protocol` framing, header serialization, and HMAC tag verification.
- Injects simulated impairment between sender and receiver.

### 6.4 Bonded E2E Provider (Blocked)
- Formally registers the test suite for multi-WAN bonding.
- Because `relay/bonding-server/src/main.rs` is a foundation placeholder and `BondedSocket` has no receive/return loop, this provider immediately returns `TestStatus::Blocked` with explicit architectural reasoning.

### 6.5 Optional Iperf3 Adapter
- Wraps an operator-installed `iperf3` executable.
- Enforces strict execution bounds:
  - Max duration: 30 seconds.
  - Explicit rate limits for UDP (default 10 Mbps).
  - Rejects execution if binary is not found or fails `--version` check.
  - Parses `--json` output into standard `BenchmarkMetrics`.

---

## 7. Speedify Competitor Comparison Protocol

To ensure fairness, comparisons must adhere to the **Apples-to-Apples Protocol**:

1. **Environmental Parity:**
   - Same physical machine, OS version, and CPU architecture.
   - Same active network adapters and physical WAN connections.
   - Same remote test server and geographical region.
   - Measured within the same 60-minute time window to prevent ISP drift.
2. **Sample Size:**
   - Minimum of 5 valid runs per configuration (recommended 10 for qualification).
   - Median (p50), 95th percentile (p95), minimum, and maximum are computed.
3. **Comparability Gate:**
   If any condition is violated (e.g., test servers differ, WAN configurations mismatch, or sample count < 5), the comparison generator tags the output as `NON_COMPARABLE` and forbids declaring a performance winner.

---

## 8. Reporting Pipeline

The benchmark core exports 5 standard artifacts to `artifacts/benchmark/runs/<run_id>/`:
1. `manifest.json`: System info, commit hash, configuration, environment metadata.
2. `results.json`: Machine-readable results and metrics for all executed suites.
3. `summary.md`: Human-readable engineering summary table.
4. `report.html`: Self-contained, responsive HTML report with embedded CSS, print stylesheets (`@media print`), and evidence-tier badges.
5. `comparison.csv`: Tabular comparison rows formatted for data analysis.

---

## 9. CI / CD Integration

GitHub Actions executes only safe, deterministic, non-root tests:
- `cargo test -p benchmark-core`
- `netbench-cli run --profile synthetic-3wan --seed 42`
- Verify that `manifest.json`, `results.json`, and `report.html` are generated and valid.
- Zero live network or route modification commands are executed in CI.
- Strict aggregate gate: any `FAIL` or unexpected missing test causes CI job termination.
