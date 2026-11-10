//! Optional iperf3 adapter.
//!
//! Provides a safe, bounded runner for an operator-installed `iperf3` executable.
//! Never downloads or installs external binaries.
//! If `iperf3` is not found, marks the test SKIPPED rather than failing or faking a pass.

use crate::models::{
    calculate_percentiles, BenchmarkMetrics, EvidenceTier, TestCaseResult, TestStatus,
};
use serde_json::Value;
use std::io::Read;
use std::net::IpAddr;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Iperf3Config {
    pub server_host: String,
    pub server_port: u16,
    pub duration_secs: u32,
    pub protocol_udp: bool,
    pub udp_target_bps: Option<u64>,
    pub reverse: bool,
    pub bind_ip: Option<String>,
    pub enable_live_tests: bool,
}

impl Default for Iperf3Config {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".into(),
            server_port: 5201,
            duration_secs: 5,
            protocol_udp: false,
            udp_target_bps: Some(10_000_000), // 10 Mbps safe default
            reverse: false,
            bind_ip: None,
            enable_live_tests: false,
        }
    }
}

fn is_loopback(host: &str) -> bool {
    let clean = host.trim().trim_start_matches('[').trim_end_matches(']');
    clean.eq_ignore_ascii_case("localhost")
        || clean
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

pub fn run_iperf3_test(config: Iperf3Config) -> TestCaseResult {
    let is_loop = is_loopback(&config.server_host);
    let tier = if is_loop {
        EvidenceTier::Loopback
    } else {
        EvidenceTier::LiveInterface
    };

    if !is_loop && !config.enable_live_tests {
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 External Measurement".to_string(),
            tier,
            status: TestStatus::Blocked,
            reason: Some(
                "External iperf3 server testing requires affirmative opt-in via enable_live_tests"
                    .into(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    // 1. Verify iperf3 availability
    let check = Command::new("iperf3").arg("--version").output();
    if check.is_err() {
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 Measurement (Optional)".to_string(),
            tier,
            status: TestStatus::Skipped,
            reason: Some(
                "iperf3 executable not found on system PATH. Install iperf3 to enable external verification."
                    .into(),
            ),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    // 2. Bound parameters
    let safe_duration = config.duration_secs.clamp(1, 30);
    let mut cmd = Command::new("iperf3");
    cmd.arg("-c").arg(&config.server_host);
    cmd.arg("-p").arg(config.server_port.to_string());
    cmd.arg("-t").arg(safe_duration.to_string());
    cmd.arg("-J"); // JSON output
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if config.protocol_udp {
        cmd.arg("-u");
        let bitrate = config.udp_target_bps.unwrap_or(10_000_000);
        cmd.arg("-b").arg(bitrate.to_string());
    }

    if config.reverse {
        cmd.arg("-R");
    }

    if let Some(ref bind) = config.bind_ip {
        cmd.arg("-B").arg(bind);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return TestCaseResult {
                test_id: "iperf3_measurement".to_string(),
                test_name: "iperf3 Measurement".to_string(),
                tier,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to spawn iperf3: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            };
        }
    };

    // Enforce process timeout: safe_duration + 5 seconds
    let timeout = Duration::from_secs((safe_duration as u64) + 5);
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("iperf3 process timed out".to_string());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("Error waiting for iperf3 child: {}", e));
            }
        }
    };

    let exit_status = match status {
        Ok(s) => s,
        Err(msg) => {
            return TestCaseResult {
                test_id: "iperf3_measurement".to_string(),
                test_name: "iperf3 Measurement".to_string(),
                tier,
                status: TestStatus::Fail,
                reason: Some(format!(
                    "iperf3 process timed out after {} seconds: {}",
                    safe_duration + 5,
                    msg
                )),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            };
        }
    };

    let mut stdout_buf = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_buf);
    }
    let mut stderr_buf = Vec::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_buf);
    }

    if !exit_status.success() {
        let stderr = String::from_utf8_lossy(&stderr_buf);
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 Measurement".to_string(),
            tier,
            status: TestStatus::Fail,
            reason: Some(format!(
                "iperf3 server communication failed: {}",
                stderr.trim()
            )),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    // Parse JSON
    let json_str = String::from_utf8_lossy(&stdout_buf);
    let v: Value = match serde_json::from_str(&json_str) {
        Ok(val) => val,
        Err(e) => {
            return TestCaseResult {
                test_id: "iperf3_measurement".to_string(),
                test_name: "iperf3 Measurement".to_string(),
                tier,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to parse iperf3 JSON: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            };
        }
    };

    let bps = v["end"]["sum_received"]["bits_per_second"]
        .as_f64()
        .or_else(|| v["end"]["sum_sent"]["bits_per_second"].as_f64())
        .unwrap_or(0.0);
    let bytes = v["end"]["sum_received"]["bytes"]
        .as_u64()
        .or_else(|| v["end"]["sum_sent"]["bytes"].as_u64())
        .unwrap_or(0);
    let mbps = bps / 1_000_000.0;

    // Fail closed on zero or non-finite throughput
    if mbps <= 0.0 || !mbps.is_finite() {
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 Measurement".to_string(),
            tier,
            status: TestStatus::Fail,
            reason: Some(format!(
                "Zero or non-finite throughput recorded: {:.2} Mbps",
                mbps
            )),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    // Extract per-interval samples if present
    let mut interval_samples = Vec::new();
    if let Some(intervals) = v["intervals"].as_array() {
        for intv in intervals {
            if let Some(intv_bps) = intv["sum"]["bits_per_second"].as_f64() {
                let intv_mbps = intv_bps / 1_000_000.0;
                if intv_mbps > 0.0 && intv_mbps.is_finite() {
                    interval_samples.push(intv_mbps);
                }
            }
        }
    }

    let (p50_tp, p95_tp) = calculate_percentiles(&interval_samples)
        .map(|(p50, p95)| (Some(p50), Some(p95)))
        .unwrap_or((None, None));

    let raw_samples = if !interval_samples.is_empty() {
        interval_samples
    } else {
        vec![mbps]
    };

    TestCaseResult {
        test_id: "iperf3_measurement".to_string(),
        test_name: "iperf3 Measurement".to_string(),
        tier,
        status: TestStatus::Pass,
        reason: None,
        metrics: Some(BenchmarkMetrics {
            duration_ms: (safe_duration as u64) * 1000,
            throughput_bps: bps as u64,
            throughput_mbps: mbps,
            latency_ms: 0.0,
            jitter_ms: 0.0,
            loss_pct: 0.0,
            packets_sent: 0,
            packets_received: 0,
            packets_dropped: 0,
            packets_reordered: 0,
            bytes_sent: bytes,
            bytes_received: bytes,
            p50_latency_ms: None,
            p95_latency_ms: None,
            p50_throughput_mbps: p50_tp,
            p95_throughput_mbps: p95_tp,
        }),
        per_path_metrics: vec![],
        raw_samples,
    }
}
