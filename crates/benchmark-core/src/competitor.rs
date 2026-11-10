//! Competitor measurement import, parsing, and fair comparability protocol.
//!
//! Enforces the strict "Apples-to-Apples" comparison rules:
//! 1. Evidence Tier Eligibility (BondedE2e or LiveInterface only).
//! 2. Topology Match (bonded vs single-path).
//! 3. Minimum Sample Count (>= 5 samples on both sides).
//! 4. Sample Validity (finite, non-NaN, positive throughput).
//! 5. Operating System Platform Compatibility.
//! 6. Operating System Version Compatibility.
//! 7. Hardware Profile Compatibility.
//! 8. Active WAN Count and Identity Matching.
//! 9. Usable Per-WAN Baseline Data Existence (both sides must contain valid per-WAN baselines).
//! 10. Per-WAN Baseline Equivalence Within 15% Tolerance.
//! 11. Bonding Mode Match (when bonded).
//! 12. Protocol Match (TCP/UDP/HTTPS).
//! 13. Transfer Direction Match (download/upload/bidirectional).
//! 14. Destination Test Server Host/Domain Match.
//! 15. Test Duration Compatibility (within 20% tolerance).
//! 16. Measurement Timestamps & Maximum Comparison Time Window (strict RFC3339 UTC, max gap <= 24h).

use crate::models::{calculate_percentiles, EvidenceTier};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default maximum allowable time difference between MultiNet and Competitor measurements (24 hours).
pub const DEFAULT_MAX_COMPARISON_WINDOW_SECS: u64 = 86_400;

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
    #[serde(default)]
    pub os_version: Option<String>,
    pub hardware_profile: String,
    pub active_wans: Vec<String>,
    pub baseline_per_wan_mbps: Vec<f64>,
    pub bonding_mode: String,
    pub test_server_url: String,
    pub test_direction: String,
    pub protocol: String,
    #[serde(default)]
    pub duration_secs: Option<u64>,
    pub samples_mbps: Vec<f64>,
    #[serde(default)]
    pub latency_ms: Option<f64>,
    #[serde(default)]
    pub jitter_ms: Option<f64>,
    #[serde(default)]
    pub loss_pct: Option<f64>,
    #[serde(default)]
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
            if *s <= 0.0 || !s.is_finite() {
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

        let mut baselines = run.wan_baselines.clone().unwrap_or_default();
        if baselines.is_empty()
            && run
                .active_wans
                .as_ref()
                .map(|w| w.len() == 1)
                .unwrap_or(false)
            && !samples.is_empty()
        {
            let mean = samples.iter().sum::<f64>() / (samples.len() as f64);
            baselines.push(mean);
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
            baseline_per_wan_mbps: baselines,
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
                .or(Some((run.timestamps.duration_ms / 1000).max(1))),
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

/// Evaluates whether a MultiNet run and a Competitor record satisfy all 16 comparability rules.
pub fn evaluate_comparability(
    multinet: &MultiNetComparisonInput,
    competitor: &CompetitorRecord,
) -> ComparisonReport {
    evaluate_comparability_with_window(multinet, competitor, DEFAULT_MAX_COMPARISON_WINDOW_SECS)
}

/// Evaluates comparability with a caller-specified maximum time-window in seconds.
pub fn evaluate_comparability_with_window(
    multinet: &MultiNetComparisonInput,
    competitor: &CompetitorRecord,
    max_window_secs: u64,
) -> ComparisonReport {
    let mut reasons = Vec::new();

    // GATE 1: Evidence Tier Eligibility
    if multinet.tier != EvidenceTier::BondedE2e && multinet.tier != EvidenceTier::LiveInterface {
        reasons.push(format!(
            "Gate 1 [Evidence Tier]: MultiNet evidence tier is {} — synthetic or loopback runs cannot be compared to physical competitor measurements",
            multinet.tier
        ));
    }

    // GATE 2: Topology Match (Bonded vs Single-Interface)
    let mn_is_bonded = multinet.is_bonded
        || multinet.tier == EvidenceTier::BondedE2e
        || multinet.active_wans.len() > 1;
    let comp_is_bonded = competitor.active_wans.len() > 1
        || (!competitor.bonding_mode.eq_ignore_ascii_case("single")
            && !competitor.bonding_mode.trim().is_empty());
    if mn_is_bonded != comp_is_bonded {
        reasons.push(format!(
            "Gate 2 [Topology]: MultiNet is {} while Competitor is {}",
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

    // GATE 3: Minimum Sample Count (>= 5 samples on both sides)
    if multinet.samples_mbps.len() < 5 {
        reasons.push(format!(
            "Gate 3 [Sample Count]: MultiNet has insufficient samples: required >= 5, got {}",
            multinet.samples_mbps.len()
        ));
    }
    if competitor.samples_mbps.len() < 5 {
        reasons.push(format!(
            "Gate 3 [Sample Count]: Competitor has insufficient samples: required >= 5, got {}",
            competitor.samples_mbps.len()
        ));
    }

    // GATE 4: Finite & Valid Sample Values
    for s in &multinet.samples_mbps {
        if *s <= 0.0 || !s.is_finite() {
            reasons.push(format!(
                "Gate 4 [Sample Validity]: MultiNet contains invalid or non-positive throughput sample: {}",
                s
            ));
            break;
        }
    }
    for s in &competitor.samples_mbps {
        if *s <= 0.0 || !s.is_finite() {
            reasons.push(format!(
                "Gate 4 [Sample Validity]: Competitor contains invalid or non-positive throughput sample: {}",
                s
            ));
            break;
        }
    }

    // GATE 5: Operating System Platform Compatibility
    if !multinet.os.trim().is_empty() && !competitor.os.trim().is_empty() {
        if !multinet.os.eq_ignore_ascii_case(&competitor.os) {
            reasons.push(format!(
                "Gate 5 [OS Platform]: Operating system platform mismatch: MultiNet on '{}', Competitor on '{}'",
                multinet.os, competitor.os
            ));
        }
    } else {
        reasons.push(
            "Gate 5 [OS Platform]: Operating system platform missing on one or both records"
                .to_string(),
        );
    }

    // GATE 6: Operating System Version Compatibility
    match (&multinet.os_version, &competitor.os_version) {
        (Some(mn_ver), Some(comp_ver))
            if !mn_ver.trim().is_empty() && !comp_ver.trim().is_empty() =>
        {
            if !mn_ver.eq_ignore_ascii_case(comp_ver) {
                reasons.push(format!(
                    "Gate 6 [OS Version]: OS version mismatch: MultiNet has '{}', Competitor has '{}'",
                    mn_ver, comp_ver
                ));
            }
        }
        (None, _) | (_, None) => {
            reasons.push(format!(
                "Gate 6 [OS Version]: OS version missing: MultiNet has {:?}, Competitor has {:?}",
                multinet.os_version, competitor.os_version
            ));
        }
        _ => {
            reasons.push("Gate 6 [OS Version]: OS version cannot be empty".to_string());
        }
    }

    // GATE 7: Hardware Profile Compatibility
    if !multinet.hardware_profile.trim().is_empty()
        && !competitor.hardware_profile.trim().is_empty()
    {
        if !multinet
            .hardware_profile
            .eq_ignore_ascii_case(&competitor.hardware_profile)
        {
            reasons.push(format!(
                "Gate 7 [Hardware Profile]: Hardware profile mismatch: MultiNet is '{}', Competitor is '{}'",
                multinet.hardware_profile, competitor.hardware_profile
            ));
        }
    } else {
        reasons.push(
            "Gate 7 [Hardware Profile]: Hardware profile missing on one or both records"
                .to_string(),
        );
    }

    // GATE 8: Active WAN Count and Identity Matching
    if multinet.active_wans.is_empty() || competitor.active_wans.is_empty() {
        reasons.push(format!(
            "Gate 8 [Active WANs]: Active WAN interfaces missing: MultiNet has {:?}, Competitor has {:?}",
            multinet.active_wans, competitor.active_wans
        ));
    } else {
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
                "Gate 8 [Active WANs]: Active WAN interfaces mismatch: MultiNet has {:?}, Competitor has {:?}",
                multinet.active_wans, competitor.active_wans
            ));
        }
    }

    // GATE 9: Usable Per-WAN Baseline Data Existence
    if multinet.baseline_per_wan_mbps.is_empty() {
        reasons.push(
            "Gate 9 [Per-WAN Baselines]: MultiNet per-WAN baselines missing or empty".to_string(),
        );
    }
    if competitor.baseline_per_wan_mbps.is_empty() {
        reasons.push(
            "Gate 9 [Per-WAN Baselines]: Competitor per-WAN baselines missing or empty".to_string(),
        );
    }
    if !multinet.baseline_per_wan_mbps.is_empty()
        && multinet.baseline_per_wan_mbps.len() != multinet.active_wans.len()
    {
        reasons.push(format!(
            "Gate 9 [Per-WAN Baselines]: MultiNet per-WAN baselines count ({}) does not match active WAN count ({})",
            multinet.baseline_per_wan_mbps.len(),
            multinet.active_wans.len()
        ));
    }
    if !competitor.baseline_per_wan_mbps.is_empty()
        && competitor.baseline_per_wan_mbps.len() != competitor.active_wans.len()
    {
        reasons.push(format!(
            "Gate 9 [Per-WAN Baselines]: Competitor per-WAN baselines count ({}) does not match active WAN count ({})",
            competitor.baseline_per_wan_mbps.len(),
            competitor.active_wans.len()
        ));
    }

    // GATE 10: Per-WAN Baseline Equivalence Within 15% Tolerance
    if !multinet.baseline_per_wan_mbps.is_empty()
        && !competitor.baseline_per_wan_mbps.is_empty()
        && multinet.baseline_per_wan_mbps.len() == competitor.baseline_per_wan_mbps.len()
    {
        for (idx, (mn_base, comp_base)) in multinet
            .baseline_per_wan_mbps
            .iter()
            .zip(&competitor.baseline_per_wan_mbps)
            .enumerate()
        {
            if *mn_base <= 0.0
                || !mn_base.is_finite()
                || *comp_base <= 0.0
                || !comp_base.is_finite()
            {
                reasons.push(format!(
                    "Gate 10 [Baseline Tolerance]: WAN #{} has invalid baseline value: MultiNet={:?}, Competitor={:?}",
                    idx + 1, mn_base, comp_base
                ));
            } else {
                let max_val = mn_base.max(*comp_base);
                let diff_ratio = (mn_base - comp_base).abs() / max_val;
                if diff_ratio > 0.15 {
                    reasons.push(format!(
                        "Gate 10 [Baseline Tolerance]: WAN #{} baseline differs by {:.1}% (> 15% tolerance): MultiNet {:.1} Mbps vs Competitor {:.1} Mbps",
                        idx + 1,
                        diff_ratio * 100.0,
                        mn_base,
                        comp_base
                    ));
                }
            }
        }
    }

    // GATE 11: Bonding Mode Match (when bonded)
    if mn_is_bonded {
        if multinet.bonding_mode.trim().is_empty() || competitor.bonding_mode.trim().is_empty() {
            reasons.push(
                "Gate 11 [Bonding Mode]: Bonding mode missing on one or both bonded records"
                    .to_string(),
            );
        } else if !multinet
            .bonding_mode
            .eq_ignore_ascii_case(&competitor.bonding_mode)
        {
            reasons.push(format!(
                "Gate 11 [Bonding Mode]: Bonding mode mismatch: MultiNet is '{}', Competitor is '{}'",
                multinet.bonding_mode, competitor.bonding_mode
            ));
        }
    }

    // GATE 12: Protocol Match
    if multinet.protocol.trim().is_empty() || competitor.protocol.trim().is_empty() {
        reasons.push("Gate 12 [Protocol]: Protocol missing on one or both records".to_string());
    } else if !multinet.protocol.eq_ignore_ascii_case(&competitor.protocol) {
        reasons.push(format!(
            "Gate 12 [Protocol]: Protocol mismatch: MultiNet used '{}', Competitor used '{}'",
            multinet.protocol, competitor.protocol
        ));
    }

    // GATE 13: Transfer Direction Match
    if multinet.test_direction.trim().is_empty() || competitor.test_direction.trim().is_empty() {
        reasons.push(
            "Gate 13 [Direction]: Transfer direction missing on one or both records".to_string(),
        );
    } else if !multinet
        .test_direction
        .eq_ignore_ascii_case(&competitor.test_direction)
    {
        reasons.push(format!(
            "Gate 13 [Direction]: Transfer direction mismatch: MultiNet is '{}', Competitor is '{}'",
            multinet.test_direction, competitor.test_direction
        ));
    }

    // GATE 14: Destination Test Server Host/Domain Match
    let normalize_server = |s: &str| {
        s.trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase()
    };
    let mn_server = normalize_server(&multinet.test_server_url);
    let comp_server = normalize_server(&competitor.test_server_url);
    if mn_server.is_empty() || comp_server.is_empty() {
        reasons.push(
            "Gate 14 [Test Server]: Destination test server missing on one or both records"
                .to_string(),
        );
    } else if mn_server != comp_server {
        reasons.push(format!(
            "Gate 14 [Test Server]: Test server mismatch: MultiNet '{}', Competitor '{}'",
            multinet.test_server_url, competitor.test_server_url
        ));
    }

    // GATE 15: Test Duration Compatibility
    match (multinet.duration_secs, competitor.duration_secs) {
        (Some(mn_dur), Some(comp_dur)) => {
            if mn_dur == 0 || comp_dur == 0 {
                reasons.push(
                    "Gate 15 [Duration]: Test duration must be strictly greater than 0".to_string(),
                );
            } else {
                let max_dur = mn_dur.max(comp_dur);
                let diff_ratio = (mn_dur as f64 - comp_dur as f64).abs() / (max_dur as f64);
                if diff_ratio > 0.20 {
                    reasons.push(format!(
                        "Gate 15 [Duration]: Test duration mismatch: MultiNet tested for {}s, Competitor tested for {}s (> 20% tolerance)",
                        mn_dur, comp_dur
                    ));
                }
            }
        }
        (None, _) | (_, None) => {
            reasons.push(format!(
                "Gate 15 [Duration]: Test duration missing: MultiNet has {:?}, Competitor has {:?}",
                multinet.duration_secs, competitor.duration_secs
            ));
        }
    }

    // GATE 16: Measurement Timestamps & Comparison Time Window
    let parse_rfc3339_strict = |ts_str: &str, side: &str| -> Result<time::OffsetDateTime, String> {
        let trimmed = ts_str.trim();
        if trimmed.is_empty() {
            return Err(format!("Gate 16 [Timestamp]: {} timestamp is empty", side));
        }
        // Strict check: date-only strings without time component (e.g. "YYYY-MM-DD") are rejected
        if trimmed.len() == 10
            && trimmed.chars().filter(|&c| c == '-').count() == 2
            && !trimmed.contains('T')
        {
            return Err(format!(
                "Gate 16 [Timestamp]: {} timestamp '{}' is date-only: must include full UTC time in RFC3339 format",
                side, trimmed
            ));
        }
        time::OffsetDateTime::parse(trimmed, &time::format_description::well_known::Rfc3339)
            .map_err(|e| {
                format!(
                    "Gate 16 [Timestamp]: {} timestamp '{}' is not valid RFC3339 format: {}",
                    side, trimmed, e
                )
            })
    };

    let mn_ts = multinet.timestamp_utc.as_deref().unwrap_or("");
    let comp_ts = &competitor.date_utc;
    match (
        parse_rfc3339_strict(mn_ts, "MultiNet"),
        parse_rfc3339_strict(comp_ts, "Competitor"),
    ) {
        (Ok(t_mn), Ok(t_comp)) => {
            let diff_secs = (t_mn - t_comp).whole_seconds().abs() as u64;
            if diff_secs > max_window_secs {
                let diff_hours = diff_secs as f64 / 3600.0;
                let max_hours = max_window_secs as f64 / 3600.0;
                reasons.push(format!(
                    "Gate 16 [Timestamp]: Time gap between MultiNet ({}) and Competitor ({}) is {:.1} hours, exceeding maximum allowed window of {:.1} hours",
                    mn_ts, comp_ts, diff_hours, max_hours
                ));
            }
        }
        (Err(e1), Err(e2)) => {
            reasons.push(e1);
            reasons.push(e2);
        }
        (Err(e), _) | (_, Err(e)) => {
            reasons.push(e);
        }
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
/// Canonical (16 fields):
/// record_id;product;version;date_utc;operator;os;os_version;hardware;wans;wan_baselines;mode;server;direction;protocol;duration_secs;samples
/// Legacy (13 fields):
/// record_id;product;version;date_utc;operator;os;hardware;wans;mode;server;direction;protocol;samples
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
        if parts.len() == 16 {
            // Canonical 16-field schema
            let samples_raw = parts[15];
            let mut samples = Vec::new();
            for s in samples_raw.split(',') {
                let val: f64 = s.trim().parse().map_err(|e| {
                    CompetitorError::CsvParse(format!("invalid float in samples '{}': {}", s, e))
                })?;
                samples.push(val);
            }

            let baselines_raw = parts[9];
            let mut baselines = Vec::new();
            if !baselines_raw.trim().is_empty() {
                for b in baselines_raw.split(',') {
                    let val: f64 = b.trim().parse().map_err(|e| {
                        CompetitorError::CsvParse(format!(
                            "invalid float in wan_baselines '{}': {}",
                            b, e
                        ))
                    })?;
                    baselines.push(val);
                }
            }

            let duration_secs: u64 = parts[14].parse().map_err(|e| {
                CompetitorError::CsvParse(format!("invalid duration_secs '{}': {}", parts[14], e))
            })?;

            let record = CompetitorRecord {
                record_id: parts[0].to_string(),
                product: parts[1].to_string(),
                version: parts[2].to_string(),
                date_utc: parts[3].to_string(),
                operator: parts[4].to_string(),
                os: parts[5].to_string(),
                os_version: Some(parts[6].to_string()),
                hardware_profile: parts[7].to_string(),
                active_wans: parts[8].split(',').map(|w| w.trim().to_string()).collect(),
                baseline_per_wan_mbps: baselines,
                bonding_mode: parts[10].to_string(),
                test_server_url: parts[11].to_string(),
                test_direction: parts[12].to_string(),
                protocol: parts[13].to_string(),
                duration_secs: Some(duration_secs),
                samples_mbps: samples,
                latency_ms: None,
                jitter_ms: None,
                loss_pct: None,
                notes: None,
            };

            record.validate()?;
            records.push(record);
        } else if parts.len() == 13 {
            // Backward-compatible 13-field legacy schema
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
                os_version: None,
                hardware_profile: parts[6].to_string(),
                active_wans: parts[7].split(',').map(|w| w.trim().to_string()).collect(),
                baseline_per_wan_mbps: vec![],
                bonding_mode: parts[8].to_string(),
                test_server_url: parts[9].to_string(),
                test_direction: parts[10].to_string(),
                protocol: parts[11].to_string(),
                duration_secs: None,
                samples_mbps: samples,
                latency_ms: None,
                jitter_ms: None,
                loss_pct: None,
                notes: None,
            };

            record.validate()?;
            records.push(record);
        } else {
            return Err(CompetitorError::CsvParse(format!(
                "line {}: expected 16 fields (canonical) or 13 fields (legacy), found {}",
                line_idx + 2,
                parts.len()
            )));
        }
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
            os_version: Some("11".into()),
            hardware_profile: "Intel i7".into(),
            active_wans: vec!["eth0".into(), "wlan0".into()],
            baseline_per_wan_mbps: vec![100.0, 50.0],
            bonding_mode: "Speed".into(),
            test_server_url: "https://speedtest.example.com".into(),
            test_direction: "download".into(),
            protocol: "TCP".into(),
            duration_secs: Some(60),
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
            duration_secs: Some(60),
            timestamp_utc: Some("2026-11-10T10:15:00Z".into()),
            is_bonded: true,
            samples_mbps: vec![130.0, 135.0, 132.0, 134.0, 136.0],
        }
    }

    #[test]
    fn test_competitor_comparability_positive_passes_all_16_gates() {
        let mn = valid_multinet_input();
        let comp = valid_comp_record();
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::Comparable {
                winner, ratio_p50, ..
            } => {
                assert_eq!(winner, "MultiNet");
                assert!(ratio_p50 > 1.0);
            }
            ComparabilityStatus::NonComparable { reasons } => {
                panic!("Expected Comparable, got NonComparable: {:?}", reasons);
            }
        }
    }

    #[test]
    fn test_gate_1_rejects_synthetic_evidence_tier() {
        let mut mn = valid_multinet_input();
        mn.tier = EvidenceTier::Synthetic;
        let comp = valid_comp_record();
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 1") && r.contains("synthetic or loopback")));
            }
            _ => panic!("Expected Gate 1 rejection"),
        }
    }

    #[test]
    fn test_gate_2_rejects_topology_mismatch_single_vs_bonded() {
        let mut mn = valid_multinet_input();
        mn.is_bonded = false;
        mn.tier = EvidenceTier::LiveInterface;
        mn.active_wans = vec!["eth0".into()];
        mn.baseline_per_wan_mbps = vec![100.0];

        let comp = valid_comp_record(); // bonded with 2 WANs
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 2") && r.contains("Topology")));
            }
            _ => panic!("Expected Gate 2 rejection"),
        }
    }

    #[test]
    fn test_gate_3_rejects_insufficient_samples_under_five() {
        let mut mn = valid_multinet_input();
        mn.samples_mbps = vec![100.0, 101.0, 102.0, 103.0]; // only 4
        let comp = valid_comp_record();
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 3") && r.contains("required >= 5, got 4")));
            }
            _ => panic!("Expected Gate 3 rejection"),
        }
    }

    #[test]
    fn test_gate_4_rejects_invalid_or_negative_sample_values() {
        let mut mn = valid_multinet_input();
        mn.samples_mbps = vec![100.0, 101.0, -5.0, 103.0, 104.0];
        let comp = valid_comp_record();
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 4") && r.contains("invalid or non-positive")));
            }
            _ => panic!("Expected Gate 4 rejection"),
        }
    }

    #[test]
    fn test_gate_5_rejects_operating_system_platform_mismatch() {
        let mut mn = valid_multinet_input();
        mn.os = "linux".into();
        let comp = valid_comp_record(); // windows
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 5") && r.contains("OS Platform")));
            }
            _ => panic!("Expected Gate 5 rejection"),
        }
    }

    #[test]
    fn test_gate_6_rejects_os_version_mismatch_or_missing() {
        let mut mn = valid_multinet_input();
        mn.os_version = Some("10".into());
        let comp = valid_comp_record(); // 11
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 6") && r.contains("OS Version")));
            }
            _ => panic!("Expected Gate 6 rejection"),
        }

        let mut comp_no_ver = valid_comp_record();
        comp_no_ver.os_version = None;
        let report_no_ver = evaluate_comparability(&valid_multinet_input(), &comp_no_ver);
        match report_no_ver.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 6") && r.contains("missing")));
            }
            _ => panic!("Expected Gate 6 missing version rejection"),
        }
    }

    #[test]
    fn test_gate_7_rejects_hardware_profile_mismatch_or_missing() {
        let mut mn = valid_multinet_input();
        mn.hardware_profile = "AMD Ryzen 9".into();
        let comp = valid_comp_record(); // Intel i7
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 7") && r.contains("Hardware Profile")));
            }
            _ => panic!("Expected Gate 7 rejection"),
        }
    }

    #[test]
    fn test_gate_8_rejects_active_wan_count_and_identity_mismatch() {
        let mut mn = valid_multinet_input();
        mn.active_wans = vec!["eth0".into(), "cell0".into()];
        let comp = valid_comp_record(); // eth0, wlan0
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 8") && r.contains("Active WANs")));
            }
            _ => panic!("Expected Gate 8 rejection"),
        }
    }

    #[test]
    fn test_gate_9_rejects_empty_or_missing_per_wan_baselines() {
        let mut mn = valid_multinet_input();
        mn.baseline_per_wan_mbps = vec![]; // empty
        let comp = valid_comp_record();
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 9") && r.contains("Per-WAN Baselines")));
            }
            _ => panic!("Expected Gate 9 rejection"),
        }
    }

    #[test]
    fn test_gate_10_rejects_per_wan_baseline_difference_over_15pct() {
        let mut mn = valid_multinet_input();
        mn.baseline_per_wan_mbps = vec![120.0, 50.0]; // 120 vs 100 is 16.7% diff
        let comp = valid_comp_record(); // 100.0, 50.0
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 10") && r.contains("> 15% tolerance")));
            }
            _ => panic!("Expected Gate 10 rejection"),
        }
    }

    #[test]
    fn test_gate_11_rejects_bonding_mode_mismatch_when_bonded() {
        let mut mn = valid_multinet_input();
        mn.bonding_mode = "Redundant".into();
        let comp = valid_comp_record(); // Speed
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 11") && r.contains("Bonding Mode")));
            }
            _ => panic!("Expected Gate 11 rejection"),
        }
    }

    #[test]
    fn test_gate_12_rejects_protocol_mismatch() {
        let mut mn = valid_multinet_input();
        mn.protocol = "UDP".into();
        let comp = valid_comp_record(); // TCP
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 12") && r.contains("Protocol")));
            }
            _ => panic!("Expected Gate 12 rejection"),
        }
    }

    #[test]
    fn test_gate_13_rejects_transfer_direction_mismatch() {
        let mut mn = valid_multinet_input();
        mn.test_direction = "upload".into();
        let comp = valid_comp_record(); // download
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 13") && r.contains("Direction")));
            }
            _ => panic!("Expected Gate 13 rejection"),
        }
    }

    #[test]
    fn test_gate_14_rejects_destination_test_server_mismatch() {
        let mut mn = valid_multinet_input();
        mn.test_server_url = "https://different-server.example.com".into();
        let comp = valid_comp_record(); // speedtest.example.com
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 14") && r.contains("Test Server")));
            }
            _ => panic!("Expected Gate 14 rejection"),
        }
    }

    #[test]
    fn test_gate_15_rejects_duration_mismatch_or_missing() {
        let mut mn = valid_multinet_input();
        mn.duration_secs = Some(30);
        let comp = valid_comp_record(); // 60s
        let report = evaluate_comparability(&mn, &comp);

        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 15") && r.contains("Duration")));
            }
            _ => panic!("Expected Gate 15 rejection"),
        }
    }

    #[test]
    fn test_gate_16_timestamp_validations() {
        let comp = valid_comp_record();

        // Missing timestamp
        let mut mn_missing = valid_multinet_input();
        mn_missing.timestamp_utc = None;
        let rep_missing = evaluate_comparability(&mn_missing, &comp);
        match rep_missing.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 16") && r.contains("empty")));
            }
            _ => panic!("Expected missing timestamp rejection"),
        }

        // Date-only timestamp
        let mut mn_date_only = valid_multinet_input();
        mn_date_only.timestamp_utc = Some("2026-11-10".to_string());
        let rep_date_only = evaluate_comparability(&mn_date_only, &comp);
        match rep_date_only.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 16") && r.contains("date-only")));
            }
            _ => panic!("Expected date-only timestamp rejection"),
        }

        // Invalid timestamp format
        let mut mn_invalid = valid_multinet_input();
        mn_invalid.timestamp_utc = Some("not-a-timestamp".to_string());
        let rep_invalid = evaluate_comparability(&mn_invalid, &comp);
        match rep_invalid.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|r| r.contains("Gate 16") && r.contains("not valid RFC3339")));
            }
            _ => panic!("Expected invalid timestamp format rejection"),
        }

        // Window boundary test: window = 3600s (1 hour)
        // comp date: 2026-11-10T10:00:00Z
        // Exactly at boundary: 2026-11-10T11:00:00Z (3600s) -> PASS
        let mut mn_exact = valid_multinet_input();
        mn_exact.timestamp_utc = Some("2026-11-10T11:00:00Z".to_string());
        let rep_exact = evaluate_comparability_with_window(&mn_exact, &comp, 3600);
        assert!(matches!(
            rep_exact.status,
            ComparabilityStatus::Comparable { .. }
        ));

        // Just inside boundary: 2026-11-10T10:59:59Z (3599s) -> PASS
        let mut mn_inside = valid_multinet_input();
        mn_inside.timestamp_utc = Some("2026-11-10T10:59:59Z".to_string());
        let rep_inside = evaluate_comparability_with_window(&mn_inside, &comp, 3600);
        assert!(matches!(
            rep_inside.status,
            ComparabilityStatus::Comparable { .. }
        ));

        // Just outside boundary: 2026-11-10T11:00:01Z (3601s) -> REJECT
        let mut mn_outside = valid_multinet_input();
        mn_outside.timestamp_utc = Some("2026-11-10T11:00:01Z".to_string());
        let rep_outside = evaluate_comparability_with_window(&mn_outside, &comp, 3600);
        match rep_outside.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons.iter().any(
                    |r| r.contains("Gate 16") && r.contains("exceeding maximum allowed window")
                ));
            }
            _ => panic!("Expected outside boundary rejection"),
        }
    }

    #[test]
    fn test_csv_parse_canonical_16_fields() {
        let csv = r#"
# Canonical CSV
record_id;product;version;date_utc;operator;os;os_version;hardware;wans;wan_baselines;mode;server;direction;protocol;duration_secs;samples
speedify-can;Speedify;14.2.0;2026-11-10T10:00:00Z;QA;windows;11;x86_64;eth0,wlan0;100.0,50.0;Speed;https://speedtest.example.com;download;TCP;60;120.5,124.2,122.8,125.1,123.9
"#;
        let records = parse_competitor_csv(csv).unwrap();
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.record_id, "speedify-can");
        assert_eq!(r.os_version, Some("11".to_string()));
        assert_eq!(r.baseline_per_wan_mbps, vec![100.0, 50.0]);
        assert_eq!(r.duration_secs, Some(60));
        assert_eq!(r.samples_mbps.len(), 5);
    }

    #[test]
    fn test_csv_parse_legacy_13_fields_imported_and_fails_missing_baselines() {
        let csv = r#"
# Legacy CSV
record_id;product;version;date_utc;operator;os;hardware;wans;mode;server;direction;protocol;samples
speedify-leg;Speedify;14.2.0;2026-11-10T10:00:00Z;QA;windows;x86_64;eth0,wlan0;Speed;https://speedtest.example.com;download;TCP;120.5,124.2,122.8,125.1,123.9
"#;
        let records = parse_competitor_csv(csv).unwrap();
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.os_version, None);
        assert_eq!(r.baseline_per_wan_mbps, Vec::<f64>::new());
        assert_eq!(r.duration_secs, None);

        // When compared with MultiNet, it fails-closed on missing baselines, missing version, etc.
        let report = evaluate_comparability(&valid_multinet_input(), r);
        match report.status {
            ComparabilityStatus::NonComparable { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|reason| reason.contains("Gate 6") && reason.contains("missing")));
                assert!(
                    reasons
                        .iter()
                        .any(|reason| reason.contains("Gate 9")
                            && reason.contains("missing or empty"))
                );
            }
            _ => panic!("Expected legacy record to fail comparison gates"),
        }
    }

    #[test]
    fn test_csv_parse_malformed_float_rejects() {
        let csv = "record_id;product;version;date_utc;operator;os;hardware;wans;mode;server;direction;protocol;samples\n\
                   s1;Speedify;1.0;2026-11-10T00:00:00Z;QA;windows;x86_64;eth0;Speed;srv;down;TCP;100.0,bad_float,102.0,103.0,104.0\n";
        let res = parse_competitor_csv(csv);
        assert!(matches!(res, Err(CompetitorError::CsvParse(_))));
    }

    #[test]
    fn test_csv_parse_missing_header_rejects() {
        let csv = "invalid;header;here\n\
                   s1;Speedify;1.0;2026-11-10T00:00:00Z;QA;windows;x86_64;eth0;Speed;srv;down;TCP;100.0,101.0,102.0,103.0,104.0\n";
        let res = parse_competitor_csv(csv);
        assert!(matches!(res, Err(CompetitorError::CsvParse(_))));
    }
}
