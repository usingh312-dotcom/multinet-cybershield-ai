//! JSON report and manifest generators.

use crate::models::BenchmarkRun;
use serde_json::json;

/// Generates `manifest.json` containing run configuration and provenance.
pub fn generate_manifest_json(run: &BenchmarkRun) -> String {
    let manifest = json!({
        "schema_version": run.schema_version,
        "run_id": run.run_id,
        "timestamps": run.timestamps,
        "metadata": run.metadata,
        "tier": run.tier,
        "profile_name": run.profile_name,
        "seed": run.seed,
        "summary": run.summary,
    });
    serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
}

/// Generates `results.json` containing complete test records and metrics.
pub fn generate_results_json(run: &BenchmarkRun) -> String {
    serde_json::to_string_pretty(run).unwrap_or_else(|_| "{}".to_string())
}
