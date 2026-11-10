# MultiNet CyberShield AI — Milestone NM-08.0.1
# Baseline Audit & Defect Catalogue: Benchmark Integrity & Fair Comparison Gates

**Document ID:** `NM08_0_1_BASELINE_AUDIT`  
**Date:** 2026-11-10  
**Status:** AUDITED & VERIFIED  
**Author:** Principal Rust Networking Architect, Benchmark/Statistics Engineer & QA Automation Lead  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Base Commit:** `56be6f21acd25eb98892e1e30c2472e079f76a5a`  
**Branch:** `feature/nm-08.0-benchmark-lab`  
**Pull Request:** #2  
**Milestone:** NM-08.0.1 (Benchmark Integrity, Live Measurement & Comparison Gate Remediation)

---

## 1. Executive Summary

Following the initial implementation of Milestone NM-08.0 (run `42757633677`), an independent architectural review identified ten integrity and statistical defects in the benchmark subsystem. While NM-08.0 correctly established evidence tier partitioning and kept the missing remote relay ingress formally marked as `BLOCKED`, the benchmark measurement harnesses contained mathematical shortcuts, tautological comparison inputs, unescaped HTML reporting, and rate limiter flaws.

Milestone **NM-08.0.1** is the focused remediation milestone dedicated to eliminating every synthetic shortcut, enforcing rigorous statistical estimation, and ensuring fair, unassailable competitor comparison gates.

---

## 2. Identified Defect Catalogue

### Defect 1: Live Remote HTTPS Baseline Validation Flaws
- **Location:** `crates/speed-test/src/lib.rs`, `crates/benchmark-core/src/providers/live.rs`, `crates/netbench-cli/src/main.rs`
- **Severity:** P0 (Security & Integrity)
- **Defect:** 
  1. `allow_self_signed_loopback` was set to `true` by default in `SpeedTestConfig::default()` and unconditionally passed as `true` in `netbench-cli baseline`.
  2. The operator interface IP (`--interface-local-ip`) was parsed but not validated against local host network interfaces before execution; invalid IPs caused confusing transport errors rather than immediate fail-closed validation.
  3. `netbench-cli baseline` returned exit code 0 even on test failure or block, and only generated 4 artifacts (omitted `results.csv`).

### Defect 2: Competitor Comparison Protocol Tautological Inputs
- **Location:** `crates/netbench-cli/src/main.rs`, `crates/benchmark-core/src/competitor.rs`
- **Severity:** P0 (Methodological Rigor)
- **Defect:** `netbench-cli compare` extracted competitor fields (`comp_record.test_server_url` and `comp_record.test_direction`) and passed them as the MultiNet run's test conditions into `evaluate_comparability`. This rendered server and direction checks tautological. Furthermore, comparability checks omitted hardware profile, active WAN identity/count, per-WAN baselines (15% tolerance), protocol, bonding mode, and topology matching (single-interface vs bonded).

### Defect 3: Measurement Statistics & Manufactured Percentiles
- **Location:** `crates/speed-test/src/lib.rs`, `crates/benchmark-core/src/providers/live.rs`, `crates/benchmark-core/src/providers/loopback.rs`
- **Severity:** P0 (Truth in Measurement)
- **Defect:**
  1. `SpeedTestReport` computed average ping and successive-difference jitter, but discarded the underlying individual probe RTTs (`rtts: Vec<f64>`).
  2. `live.rs` manufactured latency percentiles as `p50 = report.ping_ms` and `p95 = report.ping_ms + report.jitter_ms` instead of computing nearest-rank percentiles from actual probe samples.
  3. For throughput, single aggregate transfers duplicated the throughput value twice (`vec![report.download_mbps, report.download_mbps]`) and reported identical p50/p95 percentiles for a 1-sample transfer.
  4. `loopback.rs` hardcoded latency metrics (`0.1 ms`, `0.05 ms jitter`, `0.1 p50`, `0.2 p95`) instead of timing individual packet transits.

### Defect 4: Impairment Simulator Bandwidth Shaping Token Bucket Flaw
- **Location:** `crates/benchmark-core/src/impairment.rs`
- **Severity:** P1 (Simulation Correctness)
- **Defect:** When multiple packets were offered simultaneously at virtual timestamp $t=0$, each packet observed `elapsed_ms = 0` and computed the same wait delay. Consequently, all burst packets received the identical delivery timestamp, flooding the receiver simultaneously rather than serializing across the simulated physical link. Additionally, duplicate packets did not consume link transmission time.

### Defect 5: Desktop Dashboard Static Mock & Empty State Absence
- **Location:** `apps/desktop/src/pages/BenchmarkLab.tsx`
- **Severity:** P1 (UI Truthfulness)
- **Defect:** The desktop page rendered a hardcoded array of `defaultScenarios` displaying `status: PASS` and fixed throughput/latency figures, presenting synthetic models as completed runs even when no tests had been executed.

### Defect 6: Synthetic Scenario Assertions & Throughput Labeling
- **Location:** `crates/benchmark-core/src/providers/synthetic.rs`
- **Severity:** P1 (Assertion Strength)
- **Defect:** `run_tri_asymmetric_paths_scenario` asserted `c_fiber > c_starlink` and `c_5g > 0`, but did not assert that all 10,000 scheduled packets were accounted for (`c_fiber + c_starlink + c_5g == 10_000`) or that `c_starlink > c_5g`. Furthermore, the 680 Mbps sum of path capacities was reported as measured throughput rather than configured capacity. Missing adversarial cases (e.g. zero-capacity starvation).

### Defect 7: Wire Codec Divergence Between BondedSocket and Loopback
- **Location:** `crates/bonding-engine/src/socket.rs`, `crates/benchmark-core/src/providers/loopback.rs`, `crates/tunnel-protocol/src/framing.rs`
- **Severity:** P0 (Architectural Consistency)
- **Defect:** `bonding-engine/src/socket.rs` defined a private `serialize_frame` function encoding binary frames (`[length: u16 le][seq_no: u64 le][flags: u8][auth_tag: [u8; 32]][payload]`), while `loopback.rs` serialized frames using `serde_json::to_vec`. The loopback socket test was not exercising the production wire binary format.

### Defect 8: HTML Benchmark Report Cross-Site Scripting (XSS) Vulnerability
- **Location:** `crates/benchmark-core/src/reports/html.rs`
- **Severity:** P1 (Application Security)
- **Defect:** `generate_html_report` directly interpolated unescaped dynamic strings (`run.run_id`, `run.profile_name`, `t.test_name`, `t.test_id`, `t.reason`, `p.path_id`) into raw HTML templates, allowing script injection if untrusted inputs were recorded in failure reasons or profile names.

### Defect 9: iperf3 Adapter Unbounded Execution & False Pass
- **Location:** `crates/benchmark-core/src/providers/iperf3.rs`
- **Severity:** P1 (Harness Hardening)
- **Defect:** `run_iperf3_test` used blocking `Command::output()` without a process timeout, risking indefinite hangs. It unconditionally classified all runs as `EvidenceTier::Loopback` even for remote IP addresses. If `bits_per_second` parsed as 0.0, the test still returned `TestStatus::Pass` with duplicated raw samples.

### Defect 10: Qualification Script Lacked Granular Test ID Assertions
- **Location:** `packaging/test-benchmark-qualification.ps1`
- **Severity:** P1 (Release Gate Integrity)
- **Defect:** The test script verified aggregate counts (`failed -eq 0`, `blocked -ge 5`), but did not assert individual test IDs by name, did not verify that BLOCKED reasons were non-empty and contained architectural rationales, and lacked regression checks for NM-08.0.1 fixes.

---

## 3. Remediation Target Matrix

| Item | Focus Area | Target File(s) | Success Metric |
| :--- | :--- | :--- | :--- |
| **P0.1** | Canonical Wire Codec | `crates/tunnel-protocol/src/framing.rs` | Shared `encode_frame`/`decode_frame` in `tunnel-protocol` used by both `BondedSocket` and `loopback.rs`. Zero JSON on socket wire. |
| **P0.2** | Probe Statistics | `crates/speed-test`, `crates/benchmark-core/src/providers/live.rs` | `rtt_samples: Vec<f64>` preserved. Latency p50/p95 computed from samples. Jitter = mean absolute successive difference. No duplicate throughput samples. |
| **P0.3** | Live Interface Validation | `crates/benchmark-core/src/providers/live.rs`, `netbench-cli` | `allow_self_signed_loopback` default false; rejected for non-loopback. Fail-closed on unassigned interface IP. 5 artifacts & non-zero exit on fail. |
| **P0.4** | Fair Competitor Protocol | `crates/benchmark-core/src/competitor.rs`, `netbench-cli` | `MultiNetComparisonInput::from_benchmark_run` eliminates tautologies. 12-rule evaluation (OS, HW, WANs, 15% baseline tolerance, mode, protocol, topology). |
| **P1.1** | Bandwidth Shaping | `crates/benchmark-core/src/impairment.rs` | Token bucket with serialized `next_available_tx_virtual_ms`. Simultaneous packet bursts spaced by wire transmission time. |
| **P1.2** | Synthetic Assertions | `crates/benchmark-core/src/providers/synthetic.rs` | Assert all 10,000 packets scheduled and ordered (Fiber > Starlink > 5G > 0). Add zero-capacity starvation protection scenario. |
| **P1.3** | HTML Escaping | `crates/benchmark-core/src/reports/html.rs` | Comprehensive `escape_html(&str) -> String` applied to all dynamic values. XSS regression test verified. |
| **P1.4** | iperf3 Hardening | `crates/benchmark-core/src/providers/iperf3.rs` | Process timeout and child kill. Tier classified (`Loopback` vs `LiveInterface` opt-in). Zero/negative throughput fails closed. |
| **P1.5** | Desktop UI Clean State | `apps/desktop/src/pages/BenchmarkLab.tsx` | Removed hardcoded PASS cards. Added empty state for runs. Illustrative reference models clearly labeled. |
| **P1.6** | Qualification Suite | `packaging/test-benchmark-qualification.ps1` | Individual assertions for all 15 test IDs (10 PASS, 5 BLOCKED). Strict non-empty rationale verification. |
