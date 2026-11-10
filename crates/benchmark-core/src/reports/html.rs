//! Self-contained HTML benchmark report generator with print stylesheets.

use crate::models::{BenchmarkRun, TestStatus};

/// Escapes HTML special characters: `&`, `<`, `>`, `"`, `'`.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

pub fn generate_html_report(run: &BenchmarkRun) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!(
        "<title>MultiNet Benchmark Report — {}</title>\n",
        escape_html(&run.run_id)
    ));
    html.push_str("<style>\n");
    html.push_str(r#"
        :root {
            --bg: #0f172a;
            --surface: #1e293b;
            --border: #334155;
            --text: #f8fafc;
            --text-dim: #94a3b8;
            --primary: #38bdf8;
            --pass: #22c55e;
            --fail: #ef4444;
            --blocked: #f59e0b;
            --skipped: #64748b;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background: var(--bg);
            color: var(--text);
            margin: 0;
            padding: 24px;
            line-height: 1.5;
        }
        .container {
            max-width: 1100px;
            margin: 0 auto;
        }
        header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            border-bottom: 1px solid var(--border);
            padding-bottom: 20px;
            margin-bottom: 24px;
        }
        h1 { margin: 0 0 8px 0; font-size: 24px; font-weight: 700; color: #fff; }
        .meta-line { font-size: 13px; color: var(--text-dim); }
        .btn-print {
            background: var(--primary);
            color: #000;
            border: none;
            padding: 8px 16px;
            border-radius: 6px;
            font-weight: 600;
            cursor: pointer;
            font-size: 14px;
        }
        .summary-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
            gap: 16px;
            margin-bottom: 28px;
        }
        .card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px;
            text-align: center;
        }
        .card .num { font-size: 28px; font-weight: 700; margin-bottom: 4px; }
        .card .label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-dim); }
        .num-pass { color: var(--pass); }
        .num-fail { color: var(--fail); }
        .num-blocked { color: var(--blocked); }
        .num-skipped { color: var(--skipped); }

        table {
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 32px;
            background: var(--surface);
            border-radius: 8px;
            overflow: hidden;
            border: 1px solid var(--border);
        }
        th, td {
            padding: 12px 16px;
            text-align: left;
            border-bottom: 1px solid var(--border);
            font-size: 14px;
        }
        th {
            background: #111827;
            font-weight: 600;
            color: var(--text-dim);
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        tr:last-child td { border-bottom: none; }

        .badge {
            display: inline-block;
            padding: 3px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 700;
            text-transform: uppercase;
        }
        .badge-pass { background: rgba(34, 197, 94, 0.15); color: var(--pass); border: 1px solid var(--pass); }
        .badge-fail { background: rgba(239, 68, 68, 0.15); color: var(--fail); border: 1px solid var(--fail); }
        .badge-blocked { background: rgba(245, 158, 11, 0.15); color: var(--blocked); border: 1px solid var(--blocked); }
        .badge-skipped { background: rgba(100, 116, 139, 0.15); color: var(--skipped); border: 1px solid var(--skipped); }
        .badge-tier { background: rgba(56, 189, 248, 0.15); color: var(--primary); border: 1px solid var(--primary); }

        .callout-blocked {
            font-size: 12px;
            color: #fbbf24;
            background: rgba(245, 158, 11, 0.08);
            padding: 6px 10px;
            border-radius: 4px;
            margin-top: 4px;
        }

        @media print {
            body { background: #fff; color: #000; padding: 0; }
            .btn-print { display: none; }
            .card { background: #fff; border: 1px solid #ccc; }
            table { background: #fff; border: 1px solid #ccc; }
            th { background: #eee; color: #000; }
            td { border-bottom: 1px solid #ddd; }
            .badge { border: 1px solid #000; }
            .badge-pass { color: #15803d; }
            .badge-fail { color: #b91c1c; }
            .badge-blocked { color: #b45309; }
            .callout-blocked { background: #fffbeb; color: #92400e; border: 1px solid #fde68a; }
            @page { margin: 15mm; }
        }
    "#);
    html.push_str("</style>\n</head>\n<body>\n<div class=\"container\">\n");

    // Header
    html.push_str("<header>\n<div>\n");
    html.push_str("<h1>MultiNet Benchmark Lab</h1>\n");
    html.push_str(&format!(
        "<div class=\"meta-line\">Run ID: <strong>{}</strong> | Schema: {} | Tier: <span class=\"badge badge-tier\">{}</span></div>\n",
        escape_html(&run.run_id), escape_html(&run.schema_version), run.tier
    ));
    html.push_str(&format!(
        "<div class=\"meta-line\">Profile: <strong>{}</strong> | Platform: {} ({}) | Commit: <code>{}</code></div>\n",
        escape_html(&run.profile_name),
        escape_html(&run.metadata.os),
        escape_html(&run.metadata.architecture),
        escape_html(&run.metadata.commit_sha[..run.metadata.commit_sha.len().min(8)])
    ));
    html.push_str("</div>\n");
    html.push_str(
        "<button class=\"btn-print\" onclick=\"window.print()\">Print / Save as PDF</button>\n",
    );
    html.push_str("</header>\n");

    // Summary Cards
    html.push_str("<div class=\"summary-grid\">\n");
    html.push_str(&format!("<div class=\"card\"><div class=\"num\">{}</div><div class=\"label\">Total Tests</div></div>\n", run.summary.total_tests));
    html.push_str(&format!("<div class=\"card\"><div class=\"num num-pass\">{}</div><div class=\"label\">Passed</div></div>\n", run.summary.passed));
    html.push_str(&format!("<div class=\"card\"><div class=\"num num-fail\">{}</div><div class=\"label\">Failed</div></div>\n", run.summary.failed));
    html.push_str(&format!("<div class=\"card\"><div class=\"num num-blocked\">{}</div><div class=\"label\">Blocked</div></div>\n", run.summary.blocked));
    html.push_str(&format!("<div class=\"card\"><div class=\"num num-skipped\">{}</div><div class=\"label\">Skipped</div></div>\n", run.summary.skipped));
    html.push_str("</div>\n");

    // Detailed Table
    html.push_str("<h2>Test Results</h2>\n");
    html.push_str("<table>\n<thead>\n<tr>\n");
    html.push_str("<th>Test Case</th><th>Evidence Tier</th><th>Status</th><th>Throughput</th><th>Latency</th><th>Notes & Details</th>\n");
    html.push_str("</tr>\n</thead>\n<tbody>\n");

    for t in &run.tests {
        let status_badge = match t.status {
            TestStatus::Pass => "<span class=\"badge badge-pass\">PASS</span>",
            TestStatus::Fail => "<span class=\"badge badge-fail\">FAIL</span>",
            TestStatus::Blocked => "<span class=\"badge badge-blocked\">BLOCKED</span>",
            TestStatus::Skipped => "<span class=\"badge badge-skipped\">SKIPPED</span>",
            TestStatus::NotTested => "<span class=\"badge badge-skipped\">NOT TESTED</span>",
        };

        let tier_badge = format!("<span class=\"badge badge-tier\">{}</span>", t.tier);

        let (tp_str, lat_str) = if let Some(ref m) = t.metrics {
            (
                format!("{:.1} Mbps", m.throughput_mbps),
                format!("{:.1} ms", m.latency_ms),
            )
        } else {
            ("—".to_string(), "—".to_string())
        };

        let mut details = String::new();
        if let Some(ref r) = t.reason {
            let escaped_reason = escape_html(r);
            if t.status == TestStatus::Blocked {
                details.push_str(&format!(
                    "<div class=\"callout-blocked\">{}</div>",
                    escaped_reason
                ));
            } else {
                details.push_str(&format!("<div>{}</div>", escaped_reason));
            }
        }
        if !t.per_path_metrics.is_empty() {
            details.push_str("<div style=\"margin-top:6px; font-size:12px; color:var(--text-dim);\">Path Share: ");
            for p in &t.per_path_metrics {
                details.push_str(&format!(
                    "{}: {:.1}% | ",
                    escape_html(&p.path_id),
                    p.packet_share_pct
                ));
            }
            details.push_str("</div>");
        }

        html.push_str(&format!(
            "<tr>\n<td><strong>{}</strong><br><small style=\"color:var(--text-dim)\">{}</small></td>\n<td>{}</td>\n<td>{}</td>\n<td>{}</td>\n<td>{}</td>\n<td>{}</td>\n</tr>\n",
            escape_html(&t.test_name), escape_html(&t.test_id), tier_badge, status_badge, tp_str, lat_str, details
        ));
    }

    html.push_str("</tbody>\n</table>\n");
    html.push_str("</div>\n</body>\n</html>\n");

    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        EvidenceTier, RunSummary, RunTimestamps, SystemMetadata, TestCaseResult, TestStatus,
    };

    #[test]
    fn test_escape_html_replaces_all_special_chars() {
        assert_eq!(
            escape_html("<script>alert('xss' & \"attack\")</script>"),
            "&lt;script&gt;alert(&#39;xss&#39; &amp; &quot;attack&quot;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("Safe string 123"), "Safe string 123");
        assert_eq!(escape_html("Unicode: 日本語 / 🚀"), "Unicode: 日本語 / 🚀");
    }

    #[test]
    fn test_html_report_escapes_xss_payloads_in_reasons_and_names() {
        let run = BenchmarkRun::new(
            "xss-test-run<script>".to_string(),
            EvidenceTier::Synthetic,
            "test<script>".to_string(),
            Some(42),
            RunTimestamps {
                start_utc: "2026-11-10T00:00:00Z".to_string(),
                end_utc: "2026-11-10T00:01:00Z".to_string(),
                duration_ms: 60000,
            },
            SystemMetadata {
                app_version: "0.1.0".to_string(),
                commit_sha: "abcd1234<script>".to_string(),
                branch: "test".to_string(),
                os: "windows<img onerror=alert(1)>".to_string(),
                os_version: "11".to_string(),
                architecture: "x86_64".to_string(),
                cpu_model: "AMD".to_string(),
                ram_total_bytes: 16000000000,
            },
            vec![TestCaseResult {
                test_id: "id_xss<script>".to_string(),
                test_name: "Test Name <marquee>".to_string(),
                tier: EvidenceTier::Synthetic,
                status: TestStatus::Blocked,
                reason: Some("Blocked because <script>alert(1)</script> & \"quotes\"".to_string()),
                metrics: None,
                per_path_metrics: vec![],
                raw_samples: vec![],
            }],
        );

        let html = generate_html_report(&run);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<marquee>"));
        assert!(!html.contains("<img onerror=alert(1)>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;marquee&gt;"));
        assert!(html.contains("&amp; &quot;quotes&quot;"));
    }
}
