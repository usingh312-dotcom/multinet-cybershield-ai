//! Markdown summary generator for engineering reports.

use crate::models::BenchmarkRun;

pub fn generate_summary_markdown(run: &BenchmarkRun) -> String {
    let mut md = String::new();
    md.push_str(&format!("# MultiNet Benchmark Lab — Run Summary\n\n"));
    md.push_str(&format!("- **Run ID:** `{}`\n", run.run_id));
    md.push_str(&format!("- **Schema Version:** `{}`\n", run.schema_version));
    md.push_str(&format!("- **Evidence Tier:** `{}`\n", run.tier));
    md.push_str(&format!("- **Profile:** `{}`\n", run.profile_name));
    if let Some(seed) = run.seed {
        md.push_str(&format!("- **Deterministic Seed:** `{}`\n", seed));
    }
    md.push_str(&format!(
        "- **Start (UTC):** {}\n",
        run.timestamps.start_utc
    ));
    md.push_str(&format!("- **End (UTC):** {}\n", run.timestamps.end_utc));
    md.push_str(&format!(
        "- **Duration:** {} ms\n",
        run.timestamps.duration_ms
    ));
    md.push_str(&format!(
        "- **OS / Arch:** {} / {}\n",
        run.metadata.os, run.metadata.architecture
    ));
    md.push_str(&format!(
        "- **Commit SHA:** `{}`\n",
        run.metadata.commit_sha
    ));
    md.push_str("\n---\n\n");

    md.push_str("## Overall Test Summary\n\n");
    md.push_str("| Metric | Count |\n");
    md.push_str("| :--- | :---: |\n");
    md.push_str(&format!(
        "| Total Tests | **{}** |\n",
        run.summary.total_tests
    ));
    md.push_str(&format!(
        "| Passed | <span style=\"color:green\">**{}**</span> |\n",
        run.summary.passed
    ));
    md.push_str(&format!(
        "| Failed | <span style=\"color:red\">**{}**</span> |\n",
        run.summary.failed
    ));
    md.push_str(&format!(
        "| Blocked | <span style=\"color:orange\">**{}**</span> |\n",
        run.summary.blocked
    ));
    md.push_str(&format!("| Skipped | {} |\n", run.summary.skipped));
    md.push_str(&format!("| Not Tested | {} |\n", run.summary.not_tested));
    md.push_str("\n---\n\n");

    md.push_str("## Detailed Test Results\n\n");
    md.push_str("| Test ID | Test Name | Tier | Status | Throughput (Mbps) | Latency (ms) | Notes / Blocked Reason |\n");
    md.push_str("| :--- | :--- | :---: | :---: | :---: | :---: | :--- |\n");

    for t in &run.tests {
        let (mbps, lat) = if let Some(ref m) = t.metrics {
            (
                format!("{:.1}", m.throughput_mbps),
                format!("{:.1}", m.latency_ms),
            )
        } else {
            ("-".to_string(), "-".to_string())
        };

        let note = t.reason.as_deref().unwrap_or("-");
        md.push_str(&format!(
            "| `{}` | {} | `{}` | **{}** | {} | {} | {} |\n",
            t.test_id, t.test_name, t.tier, t.status, mbps, lat, note
        ));
    }

    md.push_str("\n");
    md
}
