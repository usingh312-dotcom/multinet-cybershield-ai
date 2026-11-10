//! Single-interface live baseline provider.
//!
//! Reuses `crates/speed-test` to perform genuine HTTPS download, upload,
//! and ping measurements pinned to a selected network interface.
//!
//! Live tests are strictly opt-in and require affirmative confirmation.

use crate::models::{
    calculate_percentiles, BenchmarkMetrics, EvidenceTier, TestCaseResult, TestStatus,
};
use speed_test::{run_speed_test, SpeedTestConfig, SpeedTestError};
use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Runs a single-interface baseline speed test against the specified endpoint.
pub async fn run_live_baseline(
    server_url: String,
    interface_local_ip: Option<IpAddr>,
    allow_loopback_cert: bool,
    cancel: Arc<AtomicBool>,
) -> TestCaseResult {
    // Fail-closed if specified interface IP is not assigned to any local adapter.
    if let Some(ip) = interface_local_ip {
        if std::net::UdpSocket::bind((ip, 0)).is_err() {
            return TestCaseResult {
                test_id: "live_interface_baseline".to_string(),
                test_name: format!("Single Interface Baseline ({})", ip),
                tier: EvidenceTier::LiveInterface,
                status: TestStatus::Fail,
                reason: Some(format!(
                    "Interface IP {} is not assigned to any local adapter",
                    ip
                )),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            };
        }
    }

    let config = SpeedTestConfig {
        server_url: server_url.clone(),
        interface_local_ip,
        download_bytes: 8 * 1024 * 1024, // 8 MB
        upload_bytes: 4 * 1024 * 1024,   // 4 MB
        ping_samples: 10,
        timeout_secs: 30,
        allow_self_signed_loopback: allow_loopback_cert,
    };

    let report_result = run_speed_test(&config, &cancel, |_progress| {}).await;

    match report_result {
        Ok(report) => {
            let pass = report.download_mbps > 0.0;
            let (p50_lat, p95_lat) = calculate_percentiles(&report.rtt_samples)
                .map(|(p50, p95)| (Some(p50), Some(p95)))
                .unwrap_or((None, None));

            TestCaseResult {
                test_id: "live_interface_baseline".to_string(),
                test_name: format!(
                    "Single Interface Baseline ({})",
                    interface_local_ip
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| "default-egress".into())
                ),
                tier: EvidenceTier::LiveInterface,
                status: if pass {
                    TestStatus::Pass
                } else {
                    TestStatus::Fail
                },
                reason: if pass {
                    None
                } else {
                    Some("Zero throughput recorded".into())
                },
                metrics: Some(BenchmarkMetrics {
                    duration_ms: report.duration_ms,
                    throughput_bps: (report.download_mbps * 1_000_000.0) as u64,
                    throughput_mbps: report.download_mbps,
                    latency_ms: report.ping_ms,
                    jitter_ms: report.jitter_ms,
                    loss_pct: report.loss_pct,
                    packets_sent: report.ping_samples as u64,
                    packets_received: report.ping_received as u64,
                    packets_dropped: (report.ping_samples.saturating_sub(report.ping_received))
                        as u64,
                    packets_reordered: 0,
                    bytes_sent: report.upload_bytes,
                    bytes_received: report.download_bytes,
                    p50_latency_ms: p50_lat,
                    p95_latency_ms: p95_lat,
                    p50_throughput_mbps: None,
                    p95_throughput_mbps: None,
                }),
                per_path_metrics: vec![],
                raw_samples: vec![report.download_mbps],
            }
        }
        Err(e) => {
            let (status, reason) = match e {
                SpeedTestError::Cancelled => (
                    TestStatus::Skipped,
                    "Test was cancelled by operator".to_string(),
                ),
                SpeedTestError::Server(msg) => (TestStatus::Fail, format!("Server error: {}", msg)),
                SpeedTestError::Transport(msg) => (
                    TestStatus::Blocked,
                    format!("Transport unavailable: {}", msg),
                ),
                SpeedTestError::InvalidConfig(msg) => {
                    (TestStatus::Fail, format!("Invalid configuration: {}", msg))
                }
            };

            TestCaseResult {
                test_id: "live_interface_baseline".to_string(),
                test_name: "Single Interface Baseline".to_string(),
                tier: EvidenceTier::LiveInterface,
                status,
                reason: Some(reason),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }
        }
    }
}
