//! Competitor measurement import, parsing, and fair comparability protocol.
//!
//! Enforces the "Apples-to-Apples" comparison rules:
//! 1. Same physical hardware and operating system platform.
//! 2. Same active WAN links and standalone baselines (within 15% tolerance).
//! 3. Same destination test server and transfer direction.
//! 4. Same protocol and bonding mode.
//! 5. Minimum of 5 valid, non-negative, finite measured samples.
//! 6. Disallows comparing synthetic or loopback runs against physical competitor measurements.
//! 7. Disallows comparing single-interface baseline against multi-WAN bonded configurations.

use crate::models::{calculate_percentiles, EvidenceTier};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CompetitorError {
    #[error("CSV parse error: {0}")]
    CsvParse(String),
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    #[error("insufficient samples: required at least {0}, got {1}")]
    InsufficientSamples(usize, usize),
    #[error("validation failure: {0}")]
    Validation(String),
}

/// A competitor measurement record imported from CSV or JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompetitorRecord {
    pub record_id: String,
    pub product: String,
    pub version: String,
    pub date_utc: String,
    pub operator: String,
    pub os: String,
    pub hardware_profile: String,
    pub active_wans: Vec<String>,
    pub baseline_per_wan_mbps: Vec<f64>,
    pub bonding_mode: String,
    pub test_server_url: String,
    pub test_direction: String,
    pub protocol: String,
    pub samples_mbps: Vec<f64>,
    pub latency_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub loss_pct: Option<f64>,
    pub notes: Option<String>,
}

impl CompetitorRecord {
    pub fn validate(&self) -> Result<(), CompetitorError> {
        if self.product.trim().is_empty() {
            return Err(CompetitorError::Validation(
                "product name cannot be empty".into(),
            ));
        }
        if self.samples_mbps.len() < 5 {
            return Err(CompetitorError::InsufficientSamples(
                5,
                self.samples_mbps.len(),
            ));
        }
        for s in &self.samples_mbps {
            if *s < 0.0 || !s.is_finite() {
                return Err(CompetitorError::Validation(format!(
                    "invalid throughput sample: {}",
                    s
                )));
            }
        }
        if self.active_wans.is_empty() {
            return Err(CompetitorError::Validation(
                "active_wans cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

/// Status of comparison between MultiNet and a Competitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComparabilityStatus {
    Comparable {
        multinet_p50_mbps: f64,
        multinet_p95_mbps: f64,
        competitor_p50_mbps: f64,
        competitor_p95_mbps: f64,
        ratio_p50: f64,
        winner: String,
    },
    NonComparable {
        reasons: Vec<String>,
    },
}

/// Normalized input for MultiNet test run comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiNetComparisonInput {
    pub run_id: String,
    pub tier: EvidenceTier,
    pub os: String,
    pub os_version: Option<String>,
    pub hardware_profile: String,
    pub active_wans: Vec<String>,
    pub baseline_per_wan_mbps: Vec<f64>,
    pub bonding_mode: String,
    pub test_server_url: String,
    pub test_direction: String,
    pub protocol: String,
    pub samples_mbps: Vec<f64>,
    pub duration_secs: Option<u64>,
    pub timestamp_utc: Option<String>,
    pub is_bonded: bool,
}

impl MultiNetComparisonInput {
    pub fn from_benchmark_run(run: &crate::models::BenchmarkRun) -> Self {
        let mut samples = Vec::new();
        for t in &run.tests {
            if !t.raw_samples.is_empty() {
                samples.extend_from_slice(&t.raw_samples);
            } else if let Some(ref m) = t.metrics {
                samples.push(m.throughput_mbps);
            }
        }

        Self {
            run_id: run.run_id.clone(),
            tier: run.tier,
            os: run.metadata.os.clone(),
            os_version: Some(run.metadata.os_version.clone()),
            hardware_profile: run
                .hardware_profile
                .clone()
                .unwrap_or_else(|| run.metadata.architecture.clone()),
            active_wans: run.active_wans.clone().unwrap_or_default(),
            baseline_per_wan_mbps: run.wan_baselines.clone().unwrap_or_default(),
            bonding_mode: run.bonding_mode.clone().unwrap_or_else(|| {
                if run.tier == EvidenceTier::BondedE2e {
                    "bonded".into()
                } else {
                    "single".into()
                }
            }),
            test_server_url: run.server_url.clone().unwrap_or_default(),
            test_direction: run
                .test_direction
                .clone()
                .unwrap_or_else(|| "download".into()),
            protocol: run.protocol.clone().unwrap_or_else(|| "https".into()),
            samples_mbps: samples,
            duration_secs: run
                .duration_secs
                .or(Some(run.timestamps.duration_ms / 1000)),
            timestamp_utc: Some(run.timestamps.start_utc.clone()),
            is_bonded: run.is_bonded.unwrap_or(run.tier == EvidenceTier::BondedE2e),
        }
    }
}

/// Result of evaluating comparability between MultiNet and a Competitor record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub report_id: String,
    pub multinet_run_id: String,
    pub multinet_tier: EvidenceTier,
    pub competitor_record_id: String,
    pub competitor_product: String,
    pub status: ComparabilityStatus,
}

/// Evaluates whether a MultiNet run and a Competitor record satisfy strict comparability rules.
pub fn evaluate_comparability(
    multinet: &MultiNetComparisonInput,
    competitor: &CompetitorRecord,
) -> ComparisonReport {
    let mut reasons = Vec::new();

    // RULE 1: Never compare synthetic simulations or loopback to physical competitor results
    if multinet.tier != EvidenceTier::BondedE2e && multinet.tier != EvidenceTier::LiveInterface {
        reasons.push(format!(
            "MultiNet evidence tier is {} — synthetic or loopback runs cannot be compared to physical competitor measurements",
            multinet.tier
        ));
    }

    // RULE 2: Disallow single-interface vs multi-WAN bonded comparison
    let mn_is_bonded = multinet.is_bonded
        || multinet.tier == EvidenceTier::BondedE2e
        || multinet.active_wans.len() > 1;
    let comp_is_bonded =
        competitor.active_wans.len() > 1 || !competitor.bonding_mode.eq_ignore_ascii_case("single");
    if mn_is_bonded != comp_is_bonded {
        reasons.push(format!(
            "Topology mismatch: MultiNet is {} while Competitor is {}",
            if mn_is_bonded {
                "bonded multi-WAN"
            } else {
                "single-interface baseline"
            },
            if comp_is_bonded {
                "bonded multi-WAN"
            } else {
                "single-interface baseline"
            }
        ));
    }

    // RULE 3: Minimum sample count and finite non-negative values
    if multinet.samples_mbps.len() < 5 {
        reasons.push(format!(
            "MultiNet has insufficient samples: required >= 5, got {}",
            multinet.samples_mbps.len()
        ));
    }
    for s in &multinet.samples_mbps {
        if *s < 0.0 || !s.is_finite() {
            reasons.push(format!(
                "MultiNet contains invalid throughput sample: {}",
                s
            ));
            break;
        }
    }
    if competitor.samples_mbps.len() < 5 {
        reasons.push(format!(
            "Competitor has insufficient samples: required >= 5, got {}",
            competitor.samples_mbps.len()
        ));
    }
    for s in &competitor.samples_mbps {
        if *s < 0.0 || !s.is_finite() {
            reasons.push(format!(
                "Competitor contains invalid throughput sample: {}",
                s
            ));
            break;
        }
    }

    // RULE 4: Operating System platform
    if !multinet.os.eq_ignore_ascii_case(&competitor.os) {
        reasons.push(format!(
            "Operating systems mismatch: MultiNet on '{}', Competitor on '{}'",
            multinet.os, competitor.os
        ));
    }

    // RULE 5: Hardware profile
    if !multinet.hardware_profile.trim().is_empty()
        && !competitor.hardware_profile.trim().is_empty()
        && !multinet
            .hardware_profile
            .eq_ignore_ascii_case(&competitor.hardware_profile)
    {
        reasons.push(format!(
            "Hardware profile mismatch: MultiNet is '{}', Competitor is '{}'",
            multinet.hardware_profile, competitor.hardware_profile
        ));
    }

    // RULE 6: Active WAN interfaces (count and identities)
    let mut mn_wans = multinet.active_wans.clone();
    let mut comp_wans = competitor.active_wans.clone();
    mn_wans.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    comp_wans.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    if mn_wans.len() != comp_wans.len()
        || !mn_wans
            .iter()
            .zip(&comp_wans)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        reasons.push(format!(
            "Active WAN interfaces mismatch: MultiNet has {:?}, Competitor has {:?}",
            multinet.active_wans, competitor.active_wans
        ));
    }

    // RULE 7: Per-WAN baselines within 15% tolerance
    if multinet.baseline_per_wan_mbps.len() != competitor.baseline_per_wan_mbps.len() {
        if !multinet.baseline_per_wan_mbps.is_empty()
            || !competitor.baseline_per_wan_mbps.is_empty()
        {
            reasons.push(format!(
                "Baseline per-WAN count mismatch: MultiNet has {}, Competitor has {}",
                multinet.baseline_per_wan_mbps.len(),
                competitor.baseline_per_wan_mbps.len()
            ));
        }
    } else {
        for (idx, (mn_base, comp_base)) in multinet
            .baseline_per_wan_mbps
            .iter()
            .zip(&competitor.baseline_per_wan_mbps)
            .enumerate()
        {
            let max_val = mn_base.max(*comp_base);
            if max_val > 0.0 {
                let diff_ratio = (mn_base - comp_base).abs() / max_val;
                if diff_ratio > 0.15 {
                    reasons.push(format!(
                        "WAN #{} baseline differs by {:.1}% (> 15% tolerance): MultiNet {:.1} Mbps vs Competitor {:.1} Mbps",
                        idx + 1,
                        diff_ratio * 100.0,
                        mn_base,
                        comp_base
                    ));
                }
            }
        }
    }

    // RULE 8: Bonding mode (when both are bonded)
    if mn_is_bonded
        && !multinet.bonding_mode.trim().is_empty()
        && !competitor.bonding_mode.trim().is_empty()
        && !multinet
            .bonding_mode
            .eq_ignore_ascii_case(&competitor.bonding_mode)
    {
        reasons.push(format!(
            "Bonding mode mismatch: MultiNet is '{}', Competitor is '{}'",
            multinet.bonding_mode, competitor.bonding_mode
        ));
    }

    // RULE 9: Protocol
    if !multinet.protocol.trim().is_empty()
        && !competitor.protocol.trim().is_empty()
        && !multinet.protocol.eq_ignore_ascii_case(&competitor.protocol)
    {
        reasons.push(format!(
            "Protocol mismatch: MultiNet used '{}', Competitor used '{}'",
            multinet.protocol, competitor.protocol
        ));
    }

    // RULE 10: Transfer Direction
    if !multinet
        .test_direction
        .eq_ignore_ascii_case(&competitor.test_direction)
    {
        reasons.push(format!(
            "Transfer direction mismatch: MultiNet is '{}', Competitor is '{}'",
            multinet.test_direction, competitor.test_direction
        ));
    }

    // RULE 11: Destination Test Server host / domain
    let normalize_server = |s: &str| {
        s.trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_lowercase()
    };
    if normalize_server(&multinet.test_server_url) != normalize_server(&competitor.test_server_url)
    {
        reasons.push(format!(
            "Test server mismatch: MultiNet '{}', Competitor '{}'",
            multinet.test_server_url, competitor.test_server_url
        ));
    }

    let status = if !reasons.is_empty() {
        ComparabilityStatus::NonComparable { reasons }
    } else {
        let (m_p50, m_p95) = calculate_percentiles(&multinet.samples_mbps).unwrap_or((0.0, 0.0));
        let (c_p50, c_p95) = calculate_percentiles(&competitor.samples_mbps).unwrap_or((0.0, 0.0));
        let ratio = if c_p50 > 0.0 { m_p50 / c_p50 } else { 0.0 };
        let winner = if m_p50 > c_p50 * 1.02 {
            "MultiNet".to_string()
        } else if c_p50 > m_p50 * 1.02 {
            competitor.product.clone()
        } else {
            "Parity (within 2%)".to_string()
        };

        ComparabilityStatus::Comparable {
            multinet_p50_mbps: m_p50,
            multinet_p95_mbps: m_p95,
            competitor_p50_mbps: c_p50,
            competitor_p95_mbps: c_p95,
            ratio_p50: ratio,
            winner,
        }
    };

    ComparisonReport {
        report_id: uuid::Uuid::new_v4().to_string(),
        multinet_run_id: multinet.run_id.clone(),
        multinet_tier: multinet.tier,
        competitor_record_id: competitor.record_id.clone(),
        competitor_product: competitor.product.clone(),
        status,
    }
}

/// Parse competitor CSV format:
/// record_id,product,version,date_utc,operator,os,hardware,wans,mode,server,direction,protocol,samples_csv
pub fn parse_competitor_csv(csv_content: &str) -> Result<Vec<CompetitorRecord>, CompetitorError> {
    let mut records = Vec::new();
    let mut lines = csv_content.lines().peekable();

    // Check header, skipping comment or empty lines
    let mut found_header = false;
    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            lines.next();
            continue;
        }
        if trimmed.contains("product") && trimmed.contains("samples") {
            found_header = true;
            lines.next();
            break;
        } else {
            return Err(CompetitorError::CsvParse(
                "missing required header fields (product, samples)".into(),
            ));
        }
    }

    if !found_header {
        return Err(CompetitorError::CsvParse(
            "missing required header fields (product, samples)".into(),
        ));
    }

    for (line_idx, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(';').map(|p| p.trim()).collect();
        if parts.len() < 13 {
            return Err(CompetitorError::CsvParse(format!(
                "line {}: expected at least 13 semicolon-delimited fields, found {}",
                line_idx + 2,
                parts.len()
            )));
        }

        let samples_raw = parts[12];
        let mut samples = Vec::new();
        for s in samples_raw.split(',') {
            let val: f64 = s.trim().parse().map_err(|e| {
                CompetitorError::CsvParse(format!("invalid float in samples '{}': {}", s, e))
            })?;
            samples.push(val);
        }

        let record = CompetitorRecord {
            record_id: parts[0].to_string(),
            product: parts[1].to_string(),
            version: parts[2].to_string(),
            date_utc: parts[3].to_string(),
            operator: parts[4].to_string(),
            os: parts[5].to_string(),
            hardware_profile: parts[6].to_string(),
            active_wans: parts[7].split(',').map(|w| w.trim().to_string()).collect(),
            baseline_per_wan_mbps: vec![],
            bonding_mode: parts[8].to_string(),
            test_server_url: parts[9].to_string(),
            test_direction: parts[10].to_string(),
            protocol: parts[11].to_string(),
            samples_mbps: samples,
            latency_ms: None,
            jitter_ms: None,
            loss_pct: None,
            notes: None,
        };

        record.validate()?;
        records.push(record);
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_comp_record() -> CompetitorRecord {
        CompetitorRecord {
            record_id: "comp-1".into(),
            product: "Speedify".into(),
            version: "14.2.0".into(),
            date_utc: "2026-11-10T10:00:00Z".into(),
            operator: "QA".into(),
            os: "windows".into(),
            hardware_profile: "Intel i7".into(),
            active_wans: vec!["eth0".into(), "wlan0".into()],
            baseline_per_wan_mbps: vec![100.0, 50.0],
            bonding_mode: "Speed".into(),
            test_server_url: "https://speedtest.example.com".into(),
            test_direction: "download".into(),
            protocol: "TCP".into(),
            samples_mbps: vec![120.0, 125.0, 122.0, 124.0, 126.0],
            latency_ms: Some(15.0),
            jitter_ms: Some(2.0),
            loss_pct: Some(0.1),
            notes: None,
        }
    }

    fn valid_multinet_input() -> MultiNetComparisonInput {
        MultiNetComparisonInput {
            run_id: "mn-run-1".into(),
            tier: EvidenceTier::BondedE2e,
            os: "windows".into(),
            os_version: Some("11".into()),
            hardware_profile: "Intel i7".into(),
            active_wans: vec!["eth0".into(), "wlan0".into()],
            baseline_per_wan_mbps: vec![100.0, 50.0],
            bonding_mode: "Speed".into(),
            test_server_url: "https://speedtest.example.com".into(),
            test_direction: "download".into(),
            protocol: "TCP".into(),
            samples_mbps: vec![130.0, 132.0, 131.0, 133.0, 135.0],
            duration_secs: Some(30),
            timestamp_utc: Some("2026-11-10T10:05:00Z".into()),
            is_bonded: true,
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_synthetic() {
        let comp = valid_comp_record();
        let mut mn = valid_multinet_input();
        mn.tier = EvidenceTier::Synthetic;

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("synthetic or loopback")));
            }
            _ => panic!("Expected NonComparable due to synthetic tier"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_single_vs_bonded() {
        let comp = valid_comp_record(); // bonded (2 wans)
        let mut mn = valid_multinet_input();
        mn.tier = EvidenceTier::LiveInterface;
        mn.is_bonded = false;
        mn.active_wans = vec!["eth0".into()];
        mn.bonding_mode = "single".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("Topology mismatch")));
            }
            _ => panic!("Expected NonComparable due to topology mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_insufficient_samples() {
        let comp = valid_comp_record();
        let mut mn = valid_multinet_input();
        mn.samples_mbps = vec![100.0, 102.0]; // < 5 samples

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("insufficient samples")));
            }
            _ => panic!("Expected NonComparable due to insufficient samples"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_os_mismatch() {
        let comp = valid_comp_record();
        let mut mn = valid_multinet_input();
        mn.os = "linux".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Operating systems mismatch")));
            }
            _ => panic!("Expected NonComparable due to OS mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_hardware_mismatch() {
        let comp = valid_comp_record();
        let mut mn = valid_multinet_input();
        mn.hardware_profile = "AMD Threadripper".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Hardware profile mismatch")));
            }
            _ => panic!("Expected NonComparable due to hardware mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_active_wans_mismatch() {
        let comp = valid_comp_record(); // eth0, wlan0
        let mut mn = valid_multinet_input();
        mn.active_wans = vec!["eth0".into(), "lte0".into()];

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Active WAN interfaces mismatch")));
            }
            _ => panic!("Expected NonComparable due to active WANs mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_baseline_difference_over_15pct() {
        let comp = valid_comp_record(); // [100.0, 50.0]
        let mut mn = valid_multinet_input();
        mn.baseline_per_wan_mbps = vec![100.0, 25.0]; // 50% difference on WAN 2!

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("baseline differs by")));
            }
            _ => panic!("Expected NonComparable due to baseline tolerance violation"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_bonding_mode_mismatch() {
        let comp = valid_comp_record(); // "Speed"
        let mut mn = valid_multinet_input();
        mn.bonding_mode = "Redundant".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("Bonding mode mismatch")));
            }
            _ => panic!("Expected NonComparable due to bonding mode mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_protocol_mismatch() {
        let comp = valid_comp_record(); // "TCP"
        let mut mn = valid_multinet_input();
        mn.protocol = "UDP".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("Protocol mismatch")));
            }
            _ => panic!("Expected NonComparable due to protocol mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_direction_mismatch() {
        let comp = valid_comp_record(); // "download"
        let mut mn = valid_multinet_input();
        mn.test_direction = "upload".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Transfer direction mismatch")));
            }
            _ => panic!("Expected NonComparable due to direction mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_rejects_server_mismatch() {
        let comp = valid_comp_record();
        let mut mn = valid_multinet_input();
        mn.test_server_url = "https://other-server.example.org".into();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("Test server mismatch")));
            }
            _ => panic!("Expected NonComparable due to server mismatch"),
        }
    }

    #[test]
    fn test_competitor_comparability_passes_valid_physical() {
        let comp = valid_comp_record();
        let mn = valid_multinet_input();

        let report = evaluate_comparability(&mn, &comp);
        match report.status {
            ComparabilityStatus::Comparable {
                winner,
                multinet_p50_mbps,
                competitor_p50_mbps,
                ratio_p50,
                ..
            } => {
                assert_eq!(winner, "MultiNet");
                assert!(multinet_p50_mbps > competitor_p50_mbps);
                assert!(ratio_p50 > 1.0);
            }
            ComparabilityStatus::NonComparable { reasons } => {
                panic!("Expected Comparable status, got reasons: {:?}", reasons);
            }
        }
    }
}
