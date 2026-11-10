//! MultiNet Benchmark Lab Core
//!
//! Provides data contracts, deterministic measurement providers, impairment
//! simulation, competitor comparison protocols, and multi-format report generation.

pub mod competitor;
pub mod impairment;
pub mod models;
pub mod providers;
pub mod reports;

use models::{BenchmarkRun, EvidenceTier, RunTimestamps, SystemMetadata};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// High-level orchestrator for executing a deterministic synthetic and loopback benchmark run.
pub fn execute_deterministic_benchmark(
    profile_name: &str,
    seed: u64,
    output_dir: Option<&Path>,
) -> Result<BenchmarkRun, Box<dyn std::error::Error>> {
    let start_instant = Instant::now();
    let run_id = uuid::Uuid::new_v4().to_string();
    let start_utc = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)?
        .to_string();

    // 1. Execute synthetic scheduler scenarios
    let mut tests = providers::synthetic::run_synthetic_suite(seed);

    // 2. Execute local loopback framing and socket test
    tests.extend(providers::loopback::run_loopback_suite());

    // 3. Formally register bonded E2E suite (correctly reported as BLOCKED)
    tests.extend(providers::bonded_e2e::get_bonded_e2e_suite());

    let elapsed = start_instant.elapsed();
    let end_utc = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)?
        .to_string();

    let timestamps = RunTimestamps {
        start_utc,
        end_utc,
        duration_ms: elapsed.as_millis().max(1) as u64,
    };

    let metadata = SystemMetadata {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        commit_sha: option_env!("GITHUB_SHA").unwrap_or("local-dev").to_string(),
        branch: option_env!("GITHUB_REF_NAME")
            .unwrap_or("feature/nm-08.0-benchmark-lab")
            .to_string(),
        os: std::env::consts::OS.to_string(),
        os_version: "10.0".to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        cpu_model: "Intel/AMD x86_64".to_string(),
        ram_total_bytes: 16 * 1024 * 1024 * 1024,
    };

    let run = BenchmarkRun::new(
        run_id.clone(),
        EvidenceTier::Synthetic,
        profile_name.to_string(),
        Some(seed),
        timestamps,
        metadata,
        tests,
    );

    // If an output directory was requested, write all 5 standard artifact formats
    if let Some(dir) = output_dir {
        let run_dir = dir.join(&run_id);
        fs::create_dir_all(&run_dir)?;

        let manifest = reports::json::generate_manifest_json(&run);
        fs::write(run_dir.join("manifest.json"), manifest)?;

        let results = reports::json::generate_results_json(&run);
        fs::write(run_dir.join("results.json"), results)?;

        let summary_md = reports::markdown::generate_summary_markdown(&run);
        fs::write(run_dir.join("summary.md"), summary_md)?;

        let html = reports::html::generate_html_report(&run);
        fs::write(run_dir.join("report.html"), html)?;

        let csv = reports::csv::generate_run_csv(&run);
        fs::write(run_dir.join("results.csv"), csv)?;
    }

    Ok(run)
}
