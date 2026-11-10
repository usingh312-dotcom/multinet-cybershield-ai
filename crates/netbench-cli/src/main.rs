//! Operator CLI for MultiNet Benchmark Lab.
//!
//! Subcommands:
//! - `run`: Execute deterministic synthetic & loopback benchmark suite
//! - `baseline`: Execute single-interface live baseline speed test (requires opt-in)
//! - `competitor-import`: Import and validate third-party (Speedify) test CSV/JSON
//! - `compare`: Generate comparison report between MultiNet run and competitor record

use benchmark_core::competitor::{
    evaluate_comparability, parse_competitor_csv, CompetitorRecord, MultiNetComparisonInput,
};
use benchmark_core::execute_deterministic_benchmark;
use benchmark_core::models::{
    BenchmarkRun, EvidenceTier, RunTimestamps, SystemMetadata, TestStatus,
};
use benchmark_core::providers::live::{run_live_baseline_series, BaselineSeriesConfig};
use benchmark_core::reports;
use speed_test::is_loopback_url_str;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

mod interface_validator;
use interface_validator::validate_interface_and_ip;

fn print_usage() {
    eprintln!(
        r#"netbench-cli — MultiNet Benchmark Lab CLI

USAGE:
    netbench-cli <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    run                 Execute deterministic synthetic & loopback benchmark suite
    baseline            Execute single-interface live baseline (requires opt-in)
    competitor-import   Import and validate competitor (Speedify) test data
    compare             Compare a MultiNet benchmark run with a competitor record

OPTIONS FOR 'run':
    --profile <name>    Profile name (default: synthetic-3wan)
    --seed <u64>        Deterministic random seed (default: 42)
    --output <dir>      Output artifacts directory (default: artifacts/benchmark/runs)

OPTIONS FOR 'baseline':
    --server-url <url>              Target speed test server URL
    --interface-local-ip <ip>       Local IP of adapter to test
    --enable-live-network-tests     Explicit opt-in required for live network tests
    --output <dir>                  Output artifacts directory

OPTIONS FOR 'competitor-import':
    --input <path>      Path to competitor CSV or JSON file
    --output <dir>      Output artifacts directory

OPTIONS FOR 'compare':
    --multi-net <run_id>    MultiNet run directory or ID
    --competitor <path>     Competitor imported JSON file or ID
    --output <dir>          Output directory for comparison report
"#
    );
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "run" => handle_run(&args[2..]),
        "baseline" => handle_baseline(&args[2..]).await,
        "competitor-import" => handle_competitor_import(&args[2..]),
        "compare" => handle_compare(&args[2..]),
        "--help" | "-h" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        unknown => {
            eprintln!("Unknown subcommand: {}", unknown);
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn handle_run(args: &[String]) -> ExitCode {
    let mut profile = "synthetic-3wan".to_string();
    let mut seed = 42u64;
    let mut output_dir = PathBuf::from("artifacts/benchmark/runs");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" if i + 1 < args.len() => {
                profile = args[i + 1].clone();
                i += 2;
            }
            "--seed" if i + 1 < args.len() => {
                seed = args[i + 1].parse().unwrap_or(42);
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ => i += 1,
        }
    }

    println!(
        "[netbench-cli] Starting benchmark run: profile='{}', seed={}",
        profile, seed
    );
    match execute_deterministic_benchmark(&profile, seed, Some(&output_dir)) {
        Ok(run) => {
            println!("[netbench-cli] Run completed successfully!");
            println!("[netbench-cli] Run ID: {}", run.run_id);
            println!(
                "[netbench-cli] Total tests: {}, Passed: {}, Failed: {}, Blocked: {}, Skipped: {}",
                run.summary.total_tests,
                run.summary.passed,
                run.summary.failed,
                run.summary.blocked,
                run.summary.skipped
            );
            println!(
                "[netbench-cli] Artifacts written to: {}",
                output_dir.join(&run.run_id).display()
            );

            if run.summary.failed > 0 {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("[netbench-cli] Benchmark run error: {}", e);
            ExitCode::from(1)
        }
    }
}

async fn handle_baseline(args: &[String]) -> ExitCode {
    let mut server_url = String::new();
    let mut raw_interface_ip: Option<String> = None;
    let mut interface_name: Option<String> = None;
    let mut sample_count: usize = 5; // Default 5 samples required for competitor comparison qualification
    let mut live_opt_in = false;
    let mut allow_self_signed = false;
    let mut output_dir = PathBuf::from("artifacts/benchmark/runs");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--server-url" if i + 1 < args.len() => {
                server_url = args[i + 1].clone();
                i += 2;
            }
            "--interface" if i + 1 < args.len() => {
                interface_name = Some(args[i + 1].clone());
                i += 2;
            }
            "--interface-local-ip" => {
                if i + 1 < args.len() {
                    raw_interface_ip = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("[netbench-cli] ERROR: '--interface-local-ip' requires a valid IP address argument.");
                    return ExitCode::from(1);
                }
            }
            "--samples" if i + 1 < args.len() => {
                match args[i + 1].parse::<usize>() {
                    Ok(n) if (1..=50).contains(&n) => {
                        sample_count = n;
                    }
                    _ => {
                        eprintln!("[netbench-cli] ERROR: '--samples' must be an integer between 1 and 50.");
                        return ExitCode::from(1);
                    }
                }
                i += 2;
            }
            "--enable-live-network-tests" => {
                live_opt_in = true;
                i += 1;
            }
            "--allow-self-signed-loopback" => {
                allow_self_signed = true;
                i += 1;
            }
            "--output" if i + 1 < args.len() => {
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ => i += 1,
        }
    }

    if !live_opt_in {
        eprintln!("[netbench-cli] ERROR: Live network tests require explicit opt-in via '--enable-live-network-tests'.");
        eprintln!("[netbench-cli] Aborting to protect local network stack and prevent unexpected data usage.");
        return ExitCode::from(1);
    }

    if server_url.is_empty() {
        eprintln!("[netbench-cli] ERROR: --server-url is required for live baseline test.");
        return ExitCode::from(1);
    }

    if allow_self_signed && !is_loopback_url_str(&server_url) {
        eprintln!("[netbench-cli] ERROR: '--allow-self-signed-loopback' is strictly forbidden for non-loopback endpoints.");
        return ExitCode::from(1);
    }

    // Fail-closed interface IP validation
    let discovered_ifaces = interface_discovery::discover_interfaces().unwrap_or_default();
    let interface_ip = match validate_interface_and_ip(
        raw_interface_ip.as_deref(),
        interface_name.as_deref(),
        &discovered_ifaces,
        true,
    ) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("[netbench-cli] ERROR: Interface validation failed: {}", e);
            eprintln!("[netbench-cli] Refusing to fall back to default route when an explicit interface IP was specified.");
            return ExitCode::from(1);
        }
    };

    println!(
        "[netbench-cli] Live baseline test starting against: {}",
        server_url
    );
    println!(
        "[netbench-cli] Requested trials: {} independent sample(s)",
        sample_count
    );
    if let Some(ip) = interface_ip {
        println!("[netbench-cli] Binding to local interface IP: {}", ip);
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let start_instant = Instant::now();
    let run_id = uuid::Uuid::new_v4().to_string();
    let start_utc = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
        .to_string();

    let config = BaselineSeriesConfig {
        server_url: server_url.clone(),
        interface_local_ip: interface_ip,
        allow_loopback_cert: allow_self_signed,
        sample_count,
    };
    let result = run_live_baseline_series(config, cancel).await;
    let elapsed = start_instant.elapsed();
    let end_utc = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
        .to_string();

    let timestamps = RunTimestamps {
        start_utc,
        end_utc,
        duration_ms: elapsed.as_millis().max(1) as u64,
    };

    let mut run = BenchmarkRun::new(
        run_id.clone(),
        EvidenceTier::LiveInterface,
        "live-interface-baseline".to_string(),
        None,
        timestamps,
        SystemMetadata::default(),
        vec![result],
    );
    run.server_url = Some(server_url);
    run.test_direction = Some("download".to_string());
    run.protocol = Some("https".to_string());
    run.duration_secs = Some(elapsed.as_secs().max(1));
    if let Some(ip) = interface_ip {
        run.active_wans = Some(vec![ip.to_string()]);
    }

    let run_dir = output_dir.join(&run_id);
    if let Err(e) = fs::create_dir_all(&run_dir) {
        eprintln!("[netbench-cli] ERROR creating artifact dir: {}", e);
        return ExitCode::from(1);
    }

    let writes = [
        ("manifest.json", reports::json::generate_manifest_json(&run)),
        ("results.json", reports::json::generate_results_json(&run)),
        ("report.html", reports::html::generate_html_report(&run)),
        (
            "summary.md",
            reports::markdown::generate_summary_markdown(&run),
        ),
        ("results.csv", reports::csv::generate_run_csv(&run)),
    ];

    for (fname, content) in writes {
        if let Err(e) = fs::write(run_dir.join(fname), content) {
            eprintln!("[netbench-cli] ERROR writing {}: {}", fname, e);
            return ExitCode::from(1);
        }
    }

    println!(
        "[netbench-cli] Baseline completed. Status: {}",
        run.tests[0].status
    );
    println!(
        "[netbench-cli] Captured samples: {} trial(s)",
        run.tests[0].raw_samples.len()
    );
    println!(
        "[netbench-cli] All 5 artifacts written to: {}",
        run_dir.display()
    );

    if run.tests[0].status != TestStatus::Pass {
        if let Some(ref r) = run.tests[0].reason {
            eprintln!("[netbench-cli] Failure reason: {}", r);
        }
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn handle_competitor_import(args: &[String]) -> ExitCode {
    let mut input_path = PathBuf::new();
    let mut output_dir = PathBuf::from("artifacts/benchmark/competitor");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" if i + 1 < args.len() => {
                input_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ => i += 1,
        }
    }

    if !input_path.exists() {
        eprintln!(
            "[netbench-cli] ERROR: Input file does not exist: {}",
            input_path.display()
        );
        return ExitCode::from(1);
    }

    let content = match fs::read_to_string(&input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[netbench-cli] ERROR reading input file: {}", e);
            return ExitCode::from(1);
        }
    };

    let records: Vec<CompetitorRecord> =
        if input_path.extension().map(|e| e == "json").unwrap_or(false) {
            match serde_json::from_str::<CompetitorRecord>(&content) {
                Ok(r) => vec![r],
                Err(_) => match serde_json::from_str::<Vec<CompetitorRecord>>(&content) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[netbench-cli] JSON parse error: {}", e);
                        return ExitCode::from(1);
                    }
                },
            }
        } else {
            match parse_competitor_csv(&content) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[netbench-cli] CSV parse error: {}", e);
                    return ExitCode::from(1);
                }
            }
        };

    let _ = fs::create_dir_all(&output_dir);
    for r in &records {
        println!(
            "[netbench-cli] Imported competitor record: '{}' (v{}) - {} samples",
            r.product,
            r.version,
            r.samples_mbps.len()
        );
        let target_file = output_dir.join(format!("{}.json", r.record_id));
        let _ = fs::write(&target_file, serde_json::to_string_pretty(r).unwrap());
    }

    ExitCode::SUCCESS
}

fn handle_compare(args: &[String]) -> ExitCode {
    let mut multinet_run_dir = PathBuf::new();
    let mut competitor_path = PathBuf::new();
    let mut output_dir = PathBuf::from("artifacts/benchmark/comparisons");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--multi-net" if i + 1 < args.len() => {
                multinet_run_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--competitor" if i + 1 < args.len() => {
                competitor_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ => i += 1,
        }
    }

    // Load MultiNet run
    let mn_results_path = if multinet_run_dir.is_dir() {
        multinet_run_dir.join("results.json")
    } else {
        multinet_run_dir.clone()
    };

    if !mn_results_path.exists() {
        eprintln!(
            "[netbench-cli] ERROR: MultiNet results file not found at: {}",
            mn_results_path.display()
        );
        return ExitCode::from(1);
    }

    let mn_content = fs::read_to_string(&mn_results_path).unwrap();
    let mn_run: BenchmarkRun = match serde_json::from_str(&mn_content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[netbench-cli] ERROR parsing MultiNet results: {}", e);
            return ExitCode::from(1);
        }
    };

    // Load Competitor record
    if !competitor_path.exists() {
        eprintln!(
            "[netbench-cli] ERROR: Competitor file not found at: {}",
            competitor_path.display()
        );
        return ExitCode::from(1);
    }
    let comp_content = fs::read_to_string(&competitor_path).unwrap();
    let comp_record: CompetitorRecord = match serde_json::from_str(&comp_content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[netbench-cli] ERROR parsing Competitor record: {}", e);
            return ExitCode::from(1);
        }
    };

    let multinet_input = MultiNetComparisonInput::from_benchmark_run(&mn_run);
    let comparison = evaluate_comparability(&multinet_input, &comp_record);

    if let Err(e) = fs::create_dir_all(&output_dir) {
        eprintln!("[netbench-cli] ERROR creating comparison dir: {}", e);
        return ExitCode::from(1);
    }
    let json_out = serde_json::to_string_pretty(&comparison).unwrap();
    let json_path = output_dir.join(format!("{}.json", comparison.report_id));
    if let Err(e) = fs::write(&json_path, json_out) {
        eprintln!("[netbench-cli] ERROR writing json comparison: {}", e);
        return ExitCode::from(1);
    }

    let csv_out = reports::csv::generate_comparison_csv(&comparison);
    let csv_path = output_dir.join(format!("{}.csv", comparison.report_id));
    if let Err(e) = fs::write(&csv_path, csv_out) {
        eprintln!("[netbench-cli] ERROR writing csv comparison: {}", e);
        return ExitCode::from(1);
    }

    println!("[netbench-cli] Comparison evaluated!");
    match &comparison.status {
        benchmark_core::competitor::ComparabilityStatus::Comparable {
            winner, ratio_p50, ..
        } => {
            println!("[netbench-cli] Status: COMPARABLE");
            println!(
                "[netbench-cli] Winner: {} (ratio: {:.2})",
                winner, ratio_p50
            );
        }
        benchmark_core::competitor::ComparabilityStatus::NonComparable { reasons } => {
            println!("[netbench-cli] Status: NON_COMPARABLE");
            for r in reasons {
                println!("  - {}", r);
            }
        }
    }
    println!("[netbench-cli] Reports saved to: {}", json_path.display());

    ExitCode::SUCCESS
}
