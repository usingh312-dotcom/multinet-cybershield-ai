//! Competitor measurement import, parsing, and fair comparability protocol.
//!
//! Enforces the "Apples-to-Apples" comparison rules:
//! 1. Same physical hardware and operating system platform.
//! 2. Same active WAN links and standalone baselines.
//! 3. Same destination test server.
//! 4. Minimum of 5 measured samples (10 recommended).
//! 5. Disallows comparing synthetic scheduler outputs against competitor WAN results.

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

/// Evaluates whether a MultiNet run and a Competitor record satisfy comparability rules.
pub fn evaluate_comparability(
    multinet_run_id: &str,
    multinet_tier: EvidenceTier,
    multinet_os: &str,
    multinet_server: &str,
    multinet_direction: &str,
    multinet_samples_mbps: &[f64],
    competitor: &CompetitorRecord,
) -> ComparisonReport {
    let mut reasons = Vec::new();

    // RULE 1: Never compare synthetic simulations to real competitor results
    if multinet_tier != EvidenceTier::BondedE2e && multinet_tier != EvidenceTier::LiveInterface {
        reasons.push(format!(
            "MultiNet evidence tier is {} — synthetic or loopback runs cannot be compared to physical competitor measurements",
            multinet_tier
        ));
    }

    // RULE 2: Minimum sample count
    if multinet_samples_mbps.len() < 5 {
        reasons.push(format!(
            "MultiNet has insufficient samples: required >= 5, got {}",
            multinet_samples_mbps.len()
        ));
    }
    if competitor.samples_mbps.len() < 5 {
        reasons.push(format!(
            "Competitor has insufficient samples: required >= 5, got {}",
            competitor.samples_mbps.len()
        ));
    }

    // RULE 3: Same OS
    if !multinet_os.eq_ignore_ascii_case(&competitor.os) {
        reasons.push(format!(
            "Operating systems mismatch: MultiNet on '{}', Competitor on '{}'",
            multinet_os, competitor.os
        ));
    }

    // RULE 4: Same Direction
    if !multinet_direction.eq_ignore_ascii_case(&competitor.test_direction) {
        reasons.push(format!(
            "Transfer direction mismatch: MultiNet is '{}', Competitor is '{}'",
            multinet_direction, competitor.test_direction
        ));
    }

    // RULE 5: Same Test Server host/domain
    let normalize_server = |s: &str| {
        s.trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_lowercase()
    };
    if normalize_server(multinet_server) != normalize_server(&competitor.test_server_url) {
        reasons.push(format!(
            "Test server mismatch: MultiNet '{}', Competitor '{}'",
            multinet_server, competitor.test_server_url
        ));
    }

    let status = if !reasons.is_empty() {
        ComparabilityStatus::NonComparable { reasons }
    } else {
        let (m_p50, m_p95) = calculate_percentiles(multinet_samples_mbps).unwrap_or((0.0, 0.0));
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
        multinet_run_id: multinet_run_id.to_string(),
        multinet_tier,
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

    // Check header
    if let Some(header) = lines.next() {
        if !header.contains("product") || !header.contains("samples") {
            return Err(CompetitorError::CsvParse(
                "missing required header fields (product, samples)".into(),
            ));
        }
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

    #[test]
    fn test_competitor_comparability_rejects_synthetic() {
        let competitor = CompetitorRecord {
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
        };

        let report = evaluate_comparability(
            "run-synth",
            EvidenceTier::Synthetic,
            "windows",
            "https://speedtest.example.com",
            "download",
            &[130.0, 132.0, 131.0, 133.0, 135.0],
            &competitor,
        );

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("synthetic or loopback")));
            }
            _ => panic!("Expected NonComparable due to synthetic tier"),
        }
    }

    #[test]
    fn test_competitor_comparability_passes_valid_physical() {
        let competitor = CompetitorRecord {
            record_id: "comp-1".into(),
            product: "Speedify".into(),
            version: "14.2.0".into(),
            date_utc: "2026-11-10T10:00:00Z".into(),
            operator: "QA".into(),
            os: "windows".into(),
            hardware_profile: "Intel i7".into(),
            active_wans: vec!["eth0".into()],
            baseline_per_wan_mbps: vec![100.0],
            bonding_mode: "Speed".into(),
            test_server_url: "https://speedtest.example.com".into(),
            test_direction: "download".into(),
            protocol: "TCP".into(),
            samples_mbps: vec![90.0, 92.0, 91.0, 93.0, 94.0],
            latency_ms: Some(15.0),
            jitter_ms: Some(2.0),
            loss_pct: Some(0.1),
            notes: None,
        };

        let report = evaluate_comparability(
            "run-live",
            EvidenceTier::LiveInterface,
            "windows",
            "https://speedtest.example.com",
            "download",
            &[95.0, 96.0, 94.0, 97.0, 95.0],
            &competitor,
        );

        match report.status {
            ComparabilityStatus::Comparable { winner, .. } => {
                assert_eq!(winner, "MultiNet");
            }
            _ => panic!("Expected Comparable status"),
        }
    }
}
