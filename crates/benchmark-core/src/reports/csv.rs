//! Tabular CSV exporter for benchmark runs and competitor comparisons.

use crate::competitor::{ComparabilityStatus, ComparisonReport};
use crate::models::BenchmarkRun;

pub fn generate_run_csv(run: &BenchmarkRun) -> String {
    let mut csv = String::new();
    csv.push_str("test_id,test_name,tier,status,throughput_mbps,latency_ms,jitter_ms,loss_pct,packets_sent,packets_received,bytes_sent,bytes_received,reason\n");

    for t in &run.tests {
        let (mbps, lat, jit, loss, psent, precv, bsent, brecv) = if let Some(ref m) = t.metrics {
            (
                m.throughput_mbps.to_string(),
                m.latency_ms.to_string(),
                m.jitter_ms.to_string(),
                m.loss_pct.to_string(),
                m.packets_sent.to_string(),
                m.packets_received.to_string(),
                m.bytes_sent.to_string(),
                m.bytes_received.to_string(),
            )
        } else {
            (
                "0".into(),
                "0".into(),
                "0".into(),
                "0".into(),
                "0".into(),
                "0".into(),
                "0".into(),
                "0".into(),
            )
        };

        let sanitized_reason = t
            .reason
            .as_deref()
            .unwrap_or("")
            .replace(',', ";")
            .replace('\n', " ");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            t.test_id,
            t.test_name.replace(',', ";"),
            t.tier,
            t.status,
            mbps,
            lat,
            jit,
            loss,
            psent,
            precv,
            bsent,
            brecv,
            sanitized_reason
        ));
    }

    csv
}

pub fn generate_comparison_csv(comparison: &ComparisonReport) -> String {
    let mut csv = String::new();
    csv.push_str("report_id,multinet_run_id,competitor_record_id,competitor_product,status,multinet_p50_mbps,competitor_p50_mbps,ratio,winner,notes\n");

    match &comparison.status {
        ComparabilityStatus::Comparable {
            multinet_p50_mbps,
            competitor_p50_mbps,
            ratio_p50,
            winner,
            ..
        } => {
            csv.push_str(&format!(
                "{},{},{},{},COMPARABLE,{:.2},{:.2},{:.2},{},\"\"\n",
                comparison.report_id,
                comparison.multinet_run_id,
                comparison.competitor_record_id,
                comparison.competitor_product,
                multinet_p50_mbps,
                competitor_p50_mbps,
                ratio_p50,
                winner
            ));
        }
        ComparabilityStatus::NonComparable { reasons } => {
            let note = reasons.join(" | ").replace('"', "\"\"");
            csv.push_str(&format!(
                "{},{},{},{},NON_COMPARABLE,0.0,0.0,0.0,None,\"{}\"\n",
                comparison.report_id,
                comparison.multinet_run_id,
                comparison.competitor_record_id,
                comparison.competitor_product,
                note
            ));
        }
    }

    csv
}
