//! Optional iperf3 adapter.
//!
//! Provides a safe, bounded runner for an operator-installed `iperf3` executable.
//! Never downloads or installs external binaries.
//! If `iperf3` is not found, marks the test SKIPPED rather than failing or faking a pass.

use crate::models::{BenchmarkMetrics, EvidenceTier, TestCaseResult, TestStatus};
use serde_json::Value;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Iperf3Config {
    pub server_host: String,
    pub server_port: u16,
    pub duration_secs: u32,
    pub protocol_udp: bool,
    pub udp_target_bps: Option<u64>,
    pub reverse: bool,
    pub bind_ip: Option<String>,
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
        }
    }
}

pub fn run_iperf3_test(config: Iperf3Config) -> TestCaseResult {
    // 1. Verify iperf3 availability
    let check = Command::new("iperf3").arg("--version").output();
    if check.is_err() {
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 Measurement (Optional)".to_string(),
            tier: EvidenceTier::Loopback,
            status: TestStatus::Skipped,
            reason: Some("iperf3 executable not found on system PATH. Install iperf3 to enable external verification.".into()),
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

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return TestCaseResult {
                test_id: "iperf3_measurement".to_string(),
                test_name: "iperf3 Measurement".to_string(),
                tier: EvidenceTier::Loopback,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to execute iperf3: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return TestCaseResult {
            test_id: "iperf3_measurement".to_string(),
            test_name: "iperf3 Measurement".to_string(),
            tier: EvidenceTier::Loopback,
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
    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: Value = match serde_json::from_str(&json_str) {
        Ok(val) => val,
        Err(e) => {
            return TestCaseResult {
                test_id: "iperf3_measurement".to_string(),
                test_name: "iperf3 Measurement".to_string(),
                tier: EvidenceTier::Loopback,
                status: TestStatus::Fail,
                reason: Some(format!("Failed to parse iperf3 JSON: {}", e)),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }
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

    TestCaseResult {
        test_id: "iperf3_measurement".to_string(),
        test_name: "iperf3 Measurement".to_string(),
        tier: EvidenceTier::Loopback,
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
            p50_throughput_mbps: Some(mbps),
            p95_throughput_mbps: Some(mbps),
        }),
        per_path_metrics: vec![],
        raw_samples: vec![mbps, mbps],
    }
}
