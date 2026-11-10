//! Core data contracts and schemas for MultiNet Benchmark Lab.
//!
//! Schema version: 1.0.0
//! Enforces unambiguous field names with explicit units (e.g. `duration_ms`,
//! `throughput_bps`, `latency_ms`).

use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Explicit evidence tiers separating synthetic simulations from real measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceTier {
    /// In-memory scheduler and impairment models; deterministic pseudorandom seed.
    Synthetic,
    /// Real local UDP/TCP sockets on 127.0.0.1; protocol framing and wire overhead.
    Loopback,
    /// Single unbonded physical or virtual interface HTTPS throughput and latency.
    LiveInterface,
    /// Multi-WAN egress with remote relay ingress, reassembly, and return path.
    /// (Reported as BLOCKED when relay/reassembly infrastructure is not deployed).
    BondedE2e,
    /// Manually captured and imported third-party measurements (e.g., Speedify).
    CompetitorExternal,
}

impl std::fmt::Display for EvidenceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Synthetic => write!(f, "SYNTHETIC"),
            Self::Loopback => write!(f, "LOOPBACK"),
            Self::LiveInterface => write!(f, "LIVE_INTERFACE"),
            Self::BondedE2e => write!(f, "BONDED_E2E"),
            Self::CompetitorExternal => write!(f, "COMPETITOR_EXTERNAL"),
        }
    }
}

/// Execution status of an individual benchmark test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TestStatus {
    Pass,
    Fail,
    Blocked,
    Skipped,
    NotTested,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Blocked => write!(f, "BLOCKED"),
            Self::Skipped => write!(f, "SKIPPED"),
            Self::NotTested => write!(f, "NOT_TESTED"),
        }
    }
}

/// Detailed benchmark metrics with explicit physical units.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub duration_ms: u64,
    pub throughput_bps: u64,
    pub throughput_mbps: f64,
    pub latency_ms: f64,
    pub jitter_ms: f64,
    pub loss_pct: f64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub packets_reordered: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50_throughput_mbps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_throughput_mbps: Option<f64>,
}

/// Per-path metrics for multi-WAN packet distribution accounting.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerPathMetrics {
    pub path_id: String,
    pub interface_name: String,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub packet_share_pct: f64,
    pub byte_share_pct: f64,
    pub loss_pct: f64,
    pub rtt_ms: f64,
}

/// Result of a single benchmark test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub test_id: String,
    pub test_name: String,
    pub tier: EvidenceTier,
    pub status: TestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<BenchmarkMetrics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub per_path_metrics: Vec<PerPathMetrics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_samples: Vec<f64>,
}

/// Monotonic and wall-clock timestamps for benchmark execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTimestamps {
    pub start_utc: String,
    pub end_utc: String,
    pub duration_ms: u64,
}

/// Host execution environment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetadata {
    pub app_version: String,
    pub commit_sha: String,
    pub branch: String,
    pub os: String,
    pub os_version: String,
    pub architecture: String,
    pub cpu_model: String,
    pub ram_total_bytes: u64,
}

impl Default for SystemMetadata {
    fn default() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            commit_sha: "unknown".to_string(),
            branch: "unknown".to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: "unknown".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_model: "unknown".to_string(),
            ram_total_bytes: 0,
        }
    }
}

/// Summary counts across all test cases in a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub blocked: usize,
    pub skipped: usize,
    pub not_tested: usize,
}

impl RunSummary {
    pub fn from_tests(tests: &[TestCaseResult]) -> Self {
        let mut summary = Self {
            total_tests: tests.len(),
            passed: 0,
            failed: 0,
            blocked: 0,
            skipped: 0,
            not_tested: 0,
        };
        for t in tests {
            match t.status {
                TestStatus::Pass => summary.passed += 1,
                TestStatus::Fail => summary.failed += 1,
                TestStatus::Blocked => summary.blocked += 1,
                TestStatus::Skipped => summary.skipped += 1,
                TestStatus::NotTested => summary.not_tested += 1,
            }
        }
        summary
    }

    pub fn is_all_passed(&self) -> bool {
        self.total_tests > 0 && self.passed == self.total_tests
    }
}

/// Top-level benchmark run artifact conforming to schema 1.0.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub schema_version: String,
    pub run_id: String,
    pub timestamps: RunTimestamps,
    pub metadata: SystemMetadata,
    pub tier: EvidenceTier,
    pub profile_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_direction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_wans: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wan_baselines: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonding_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_bonded: Option<bool>,
    pub tests: Vec<TestCaseResult>,
    pub summary: RunSummary,
}

impl BenchmarkRun {
    pub fn new(
        run_id: String,
        tier: EvidenceTier,
        profile_name: String,
        seed: Option<u64>,
        timestamps: RunTimestamps,
        metadata: SystemMetadata,
        tests: Vec<TestCaseResult>,
    ) -> Self {
        let summary = RunSummary::from_tests(&tests);
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            run_id,
            timestamps,
            metadata,
            tier,
            profile_name,
            seed,
            server_url: None,
            test_direction: None,
            protocol: None,
            hardware_profile: None,
            active_wans: None,
            wan_baselines: None,
            bonding_mode: None,
            duration_secs: None,
            is_bonded: None,
            tests,
            summary,
        }
    }
}

/// Percentile calculation helper.
///
/// Implements nearest-rank percentile estimation.
/// Returns `None` if fewer than 2 samples are provided to prevent misleading single-sample percentiles.
pub fn calculate_percentiles(samples: &[f64]) -> Option<(f64, f64)> {
    if samples.len() < 2 {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let p50_idx = ((n as f64) * 0.50).ceil() as usize - 1;
    let p95_idx = ((n as f64) * 0.95).ceil() as usize - 1;

    let p50 = sorted[p50_idx.min(n - 1)];
    let p95 = sorted[p95_idx.min(n - 1)];

    Some((p50, p95))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentiles_calculation() {
        assert_eq!(calculate_percentiles(&[]), None);
        assert_eq!(calculate_percentiles(&[10.0]), None);

        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let (p50, p95) = calculate_percentiles(&samples).unwrap();
        assert!((p50 - 50.0).abs() < 1e-6);
        assert!((p95 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_schema_serialization_roundtrip() {
        let test = TestCaseResult {
            test_id: "test_1".to_string(),
            test_name: "Synthetic test".to_string(),
            tier: EvidenceTier::Synthetic,
            status: TestStatus::Pass,
            reason: None,
            metrics: Some(BenchmarkMetrics {
                duration_ms: 1000,
                throughput_bps: 100_000_000,
                throughput_mbps: 100.0,
                latency_ms: 15.0,
                jitter_ms: 2.0,
                loss_pct: 0.0,
                packets_sent: 1000,
                packets_received: 1000,
                packets_dropped: 0,
                packets_reordered: 0,
                bytes_sent: 1_250_000,
                bytes_received: 1_250_000,
                p50_latency_ms: Some(14.8),
                p95_latency_ms: Some(15.5),
                p50_throughput_mbps: Some(100.0),
                p95_throughput_mbps: Some(100.0),
            }),
            per_path_metrics: vec![],
            raw_samples: vec![14.8, 15.2, 15.5],
        };

        let run = BenchmarkRun::new(
            "run-123".to_string(),
            EvidenceTier::Synthetic,
            "test-profile".to_string(),
            Some(42),
            RunTimestamps {
                start_utc: "2026-11-10T00:00:00Z".to_string(),
                end_utc: "2026-11-10T00:00:01Z".to_string(),
                duration_ms: 1000,
            },
            SystemMetadata::default(),
            vec![test],
        );

        let json = serde_json::to_string_pretty(&run).unwrap();
        assert!(json.contains("\"schema_version\": \"1.0.0\""));
        assert!(json.contains("\"tier\": \"SYNTHETIC\""));
        assert!(json.contains("\"passed\": 1"));

        let deserialized: BenchmarkRun = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.run_id, "run-123");
        assert_eq!(deserialized.summary.passed, 1);
    }
}
