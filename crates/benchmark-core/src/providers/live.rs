//! Single-interface live baseline provider.
//!
//! Reuses `crates/speed-test` to perform genuine HTTPS download, upload,
//! and ping measurements pinned to a selected network interface.
//!
//! Live tests are strictly opt-in and require affirmative confirmation.

use crate::models::{
    calculate_percentiles, BenchmarkMetrics, EvidenceTier, TestCaseResult, TestStatus,
};
use speed_test::{run_speed_test, SpeedTestConfig, SpeedTestError, SpeedTestReport};
use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Configuration for repeated live baseline sampling.
#[derive(Debug, Clone)]
pub struct BaselineSeriesConfig {
    pub server_url: String,
    pub interface_local_ip: Option<IpAddr>,
    pub allow_loopback_cert: bool,
    pub sample_count: usize,
}

/// Runs a single-trial live baseline speed test against the specified endpoint.
///
/// Produces a single sample. Under NM-08.0.2 rules, a single measurement does not qualify
/// for competitor comparison, which requires at least five independent trials.
pub async fn run_live_baseline(
    server_url: String,
    interface_local_ip: Option<IpAddr>,
    allow_loopback_cert: bool,
    cancel: Arc<AtomicBool>,
) -> TestCaseResult {
    run_live_baseline_series(
        BaselineSeriesConfig {
            server_url,
            interface_local_ip,
            allow_loopback_cert,
            sample_count: 1,
        },
        cancel,
    )
    .await
}

/// Runs a repeated live baseline series across `sample_count` independent trials.
pub async fn run_live_baseline_series(
    config: BaselineSeriesConfig,
    cancel: Arc<AtomicBool>,
) -> TestCaseResult {
    let server_url = config.server_url.clone();
    let ip = config.interface_local_ip;
    let allow_cert = config.allow_loopback_cert;

    run_live_baseline_series_with_runner(config, cancel, |_trial| {
        let s_url = server_url.clone();
        async move {
            let sp_config = SpeedTestConfig {
                server_url: s_url,
                interface_local_ip: ip,
                download_bytes: 8 * 1024 * 1024,
                upload_bytes: 4 * 1024 * 1024,
                ping_samples: 10,
                timeout_secs: 30,
                allow_self_signed_loopback: allow_cert,
            };
            let dummy_cancel = Arc::new(AtomicBool::new(false));
            run_speed_test(&sp_config, &dummy_cancel, |_| {}).await
        }
    })
    .await
}

/// Generalized live baseline series runner supporting dependency-injected test runners.
pub async fn run_live_baseline_series_with_runner<F, Fut>(
    config: BaselineSeriesConfig,
    cancel: Arc<AtomicBool>,
    mut runner: F,
) -> TestCaseResult
where
    F: FnMut(usize) -> Fut,
    Fut: std::future::Future<Output = Result<SpeedTestReport, SpeedTestError>>,
{
    let target_name = config
        .interface_local_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "default-egress".into());

    // Fail-closed if specified interface IP cannot be bound locally
    if let Some(ip) = config.interface_local_ip {
        if std::net::UdpSocket::bind((ip, 0)).is_err() {
            return TestCaseResult {
                test_id: "live_interface_baseline".to_string(),
                test_name: format!("Single Interface Baseline ({})", target_name),
                tier: EvidenceTier::LiveInterface,
                status: TestStatus::Fail,
                reason: Some(format!(
                    "Interface IP {} is not assigned to any local adapter or cannot be bound",
                    ip
                )),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            };
        }
    }

    if config.sample_count == 0 {
        return TestCaseResult {
            test_id: "live_interface_baseline".to_string(),
            test_name: format!("Single Interface Baseline ({})", target_name),
            tier: EvidenceTier::LiveInterface,
            status: TestStatus::Fail,
            reason: Some("Sample count must be at least 1".to_string()),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    if config.sample_count > 50 {
        return TestCaseResult {
            test_id: "live_interface_baseline".to_string(),
            test_name: format!("Single Interface Baseline ({})", target_name),
            tier: EvidenceTier::LiveInterface,
            status: TestStatus::Fail,
            reason: Some("Sample count exceeds maximum limit of 50".to_string()),
            metrics: None,
            per_path_metrics: vec![],
            raw_samples: vec![],
        };
    }

    let mut samples = Vec::with_capacity(config.sample_count);
    let mut total_duration_ms = 0u64;
    let mut all_rtt_samples = Vec::new();
    let mut total_bytes_sent = 0u64;
    let mut total_bytes_received = 0u64;
    let mut total_ping_samples = 0u64;
    let mut total_ping_received = 0u64;
    let mut ping_latencies = Vec::new();
    let mut jitter_samples = Vec::new();

    for trial in 1..=config.sample_count {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return TestCaseResult {
                test_id: "live_interface_baseline".to_string(),
                test_name: format!("Single Interface Baseline ({})", target_name),
                tier: EvidenceTier::LiveInterface,
                status: TestStatus::Skipped,
                reason: Some("Test was cancelled by operator".to_string()),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: samples,
            };
        }

        match runner(trial).await {
            Ok(report) => {
                if report.download_mbps <= 0.0 || !report.download_mbps.is_finite() {
                    return TestCaseResult {
                        test_id: "live_interface_baseline".to_string(),
                        test_name: format!("Single Interface Baseline ({})", target_name),
                        tier: EvidenceTier::LiveInterface,
                        status: TestStatus::Fail,
                        reason: Some(format!(
                            "Trial {}/{} recorded invalid or non-positive throughput: {:.2} Mbps. Incomplete series cannot qualify as baseline.",
                            trial, config.sample_count, report.download_mbps
                        )),
                        metrics: None,
                        per_path_metrics: vec![],
                        raw_samples: samples,
                    };
                }

                samples.push(report.download_mbps);
                total_duration_ms += report.duration_ms;
                all_rtt_samples.extend_from_slice(&report.rtt_samples);
                total_bytes_sent += report.upload_bytes;
                total_bytes_received += report.download_bytes;
                total_ping_samples += report.ping_samples as u64;
                total_ping_received += report.ping_received as u64;
                if report.ping_ms > 0.0 {
                    ping_latencies.push(report.ping_ms);
                }
                jitter_samples.push(report.jitter_ms);
            }
            Err(e) => {
                return TestCaseResult {
                    test_id: "live_interface_baseline".to_string(),
                    test_name: format!("Single Interface Baseline ({})", target_name),
                    tier: EvidenceTier::LiveInterface,
                    status: TestStatus::Fail,
                    reason: Some(format!(
                        "Trial {}/{} failed with error: {:?}. Incomplete series cannot qualify as baseline.",
                        trial, config.sample_count, e
                    )),
                    metrics: None,
                    per_path_metrics: vec![],
                    raw_samples: samples,
                };
            }
        }
    }

    // All requested trials completed successfully!
    let (p50_tp, p95_tp) = calculate_percentiles(&samples)
        .map(|(p50, p95)| (Some(p50), Some(p95)))
        .unwrap_or((None, None));
    let (p50_lat, p95_lat) = calculate_percentiles(&all_rtt_samples)
        .map(|(p50, p95)| (Some(p50), Some(p95)))
        .unwrap_or((None, None));

    let mean_throughput = samples.iter().sum::<f64>() / (samples.len() as f64);
    let avg_latency = if !ping_latencies.is_empty() {
        ping_latencies.iter().sum::<f64>() / (ping_latencies.len() as f64)
    } else {
        0.0
    };
    let avg_jitter = if !jitter_samples.is_empty() {
        jitter_samples.iter().sum::<f64>() / (jitter_samples.len() as f64)
    } else {
        0.0
    };
    let loss_pct = if total_ping_samples > 0 {
        ((total_ping_samples.saturating_sub(total_ping_received)) as f64
            / total_ping_samples as f64)
            * 100.0
    } else {
        0.0
    };

    TestCaseResult {
        test_id: "live_interface_baseline".to_string(),
        test_name: format!(
            "Single Interface Baseline ({}, {} trials)",
            target_name,
            samples.len()
        ),
        tier: EvidenceTier::LiveInterface,
        status: TestStatus::Pass,
        reason: None,
        metrics: Some(BenchmarkMetrics {
            duration_ms: total_duration_ms,
            throughput_bps: (mean_throughput * 1_000_000.0) as u64,
            throughput_mbps: mean_throughput,
            latency_ms: avg_latency,
            jitter_ms: avg_jitter,
            loss_pct,
            packets_sent: total_ping_samples,
            packets_received: total_ping_received,
            packets_dropped: total_ping_samples.saturating_sub(total_ping_received),
            packets_reordered: 0,
            bytes_sent: total_bytes_sent,
            bytes_received: total_bytes_received,
            p50_latency_ms: p50_lat,
            p95_latency_ms: p95_lat,
            p50_throughput_mbps: p50_tp,
            p95_throughput_mbps: p95_tp,
        }),
        per_path_metrics: vec![],
        raw_samples: samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mock_report(download_mbps: f64) -> SpeedTestReport {
        SpeedTestReport {
            server_url: "https://127.0.0.1:9000".into(),
            server_name: "Mock Server".into(),
            server_observed_client_ip: "127.0.0.1".into(),
            interface_local_ip: None,
            duration_ms: 1000,
            download_mbps,
            upload_mbps: 10.0,
            ping_ms: 15.0,
            jitter_ms: 2.0,
            loss_pct: 0.0,
            download_bytes: 1_000_000,
            upload_bytes: 500_000,
            ping_samples: 5,
            ping_received: 5,
            rtt_samples: vec![14.0, 15.0, 15.5, 16.0, 14.5],
            started_at: "2026-11-10T00:00:00Z".into(),
            completed_at: "2026-11-10T00:00:01Z".into(),
        }
    }

    #[tokio::test]
    async fn test_live_baseline_series_five_samples_produce_five_distinct_samples() {
        let cancel = Arc::new(AtomicBool::new(false));
        let config = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 5,
        };

        let values = [100.0, 102.5, 99.0, 101.0, 103.0];
        let result = run_live_baseline_series_with_runner(config, cancel, |trial| async move {
            Ok(make_mock_report(values[trial - 1]))
        })
        .await;

        assert_eq!(result.status, TestStatus::Pass);
        assert_eq!(result.raw_samples.len(), 5);
        assert_eq!(result.raw_samples, values.to_vec());
        let metrics = result.metrics.unwrap();
        assert!(metrics.p50_throughput_mbps.is_some());
        assert!(metrics.p95_throughput_mbps.is_some());
        assert_eq!(metrics.throughput_mbps, 101.1);
    }

    #[tokio::test]
    async fn test_live_baseline_series_single_sample_remains_one_sample() {
        let cancel = Arc::new(AtomicBool::new(false));
        let config = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 1,
        };

        let result = run_live_baseline_series_with_runner(config, cancel, |_| async move {
            Ok(make_mock_report(95.5))
        })
        .await;

        assert_eq!(result.status, TestStatus::Pass);
        assert_eq!(result.raw_samples.len(), 1);
        assert_eq!(result.raw_samples[0], 95.5);
    }

    #[tokio::test]
    async fn test_live_baseline_series_fails_closed_when_trial_fails() {
        let cancel = Arc::new(AtomicBool::new(false));
        let config = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 5,
        };

        let result = run_live_baseline_series_with_runner(config, cancel, |trial| async move {
            if trial == 3 {
                Err(SpeedTestError::Transport("connection reset".to_string()))
            } else {
                Ok(make_mock_report(100.0))
            }
        })
        .await;

        assert_eq!(result.status, TestStatus::Fail);
        assert!(result.reason.unwrap().contains("Trial 3/5 failed"));
        assert!(result.metrics.is_none());
    }

    #[tokio::test]
    async fn test_live_baseline_series_fails_closed_when_trial_records_zero_throughput() {
        let cancel = Arc::new(AtomicBool::new(false));
        let config = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 5,
        };

        let result = run_live_baseline_series_with_runner(config, cancel, |trial| async move {
            if trial == 2 {
                Ok(make_mock_report(0.0))
            } else {
                Ok(make_mock_report(100.0))
            }
        })
        .await;

        assert_eq!(result.status, TestStatus::Fail);
        assert!(result
            .reason
            .unwrap()
            .contains("invalid or non-positive throughput"));
    }

    #[tokio::test]
    async fn test_live_baseline_series_sample_count_bounds() {
        let cancel = Arc::new(AtomicBool::new(false));

        let config_zero = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 0,
        };
        let res_zero =
            run_live_baseline_series_with_runner(config_zero, cancel.clone(), |_| async {
                Ok(make_mock_report(100.0))
            })
            .await;
        assert_eq!(res_zero.status, TestStatus::Fail);

        let config_excess = BaselineSeriesConfig {
            server_url: "https://127.0.0.1:9000".to_string(),
            interface_local_ip: None,
            allow_loopback_cert: true,
            sample_count: 51,
        };
        let res_excess = run_live_baseline_series_with_runner(config_excess, cancel, |_| async {
            Ok(make_mock_report(100.0))
        })
        .await;
        assert_eq!(res_excess.status, TestStatus::Fail);
    }
}
