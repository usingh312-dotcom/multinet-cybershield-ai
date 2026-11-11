//! Native Tauri IPC integration tests.
//!
//! These tests drive the **real** `#[tauri::command]` implementations through
//! Tauri's IPC dispatch path (`tauri::test::get_ipc_response`). No command
//! behaviour is re-implemented or stubbed: the SQLite database, the Windows
//! interface inventory, the traffic sampler and the speed-test engine are the
//! production implementations.
//!
//! They are deliberately separate from the browser-preview tests in
//! `apps/desktop/src/test/browser`, which run without any IPC channel at all.

use multinet_cybershield_ai_desktop::db::{self, DatabaseState};
use multinet_cybershield_ai_desktop::diagnostics::DiagnosticsState;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindowBuilder};

/// Builds the application with the production command surface and a real
/// SQLite database in a throwaway directory.
///
/// The `setup` hook of a Tauri application only runs when the runtime
/// signals `Ready`, which the mock runtime never does. The managed state is
/// therefore installed explicitly here, using exactly the same types the
/// production `run()` function manages.
fn build_test_app() -> (
    tauri::App<tauri::test::MockRuntime>,
    tauri::WebviewWindow<tauri::test::MockRuntime>,
) {
    let data_dir = tempfile::tempdir().expect("temp dir");
    let data_path = data_dir.keep();

    let app = multinet_cybershield_ai_desktop::build_app(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("failed to build the test application");

    let database_state =
        tauri::async_runtime::block_on(db::initialize_database(&data_path)).expect("database");
    app.manage(database_state);
    app.manage(Arc::new(DiagnosticsState::new()));

    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("failed to build the test webview");

    (app, webview)
}

fn ipc_request(cmd: &str, body: Value) -> InvokeRequest {
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

/// Dispatches a command and returns the raw JSON response.
fn invoke(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: Value,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(webview, ipc_request(cmd, body))
        .map(|response| response.deserialize::<Value>().unwrap_or(Value::Null))
}

/// Dispatches a command and asserts it succeeded.
fn invoke_ok(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: Value,
) -> Value {
    match invoke(webview, cmd, body) {
        Ok(value) => value,
        Err(error) => panic!("native command `{cmd}` failed: {error}"),
    }
}

#[test]
fn foundation_status_command_returns_the_real_product_contract() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_foundation_status", json!({}));

    assert_eq!(response["product"], "MultiNet CyberShield AI");
    assert_eq!(response["version"], "0.1.0");
    assert!(response["phase"].as_str().is_some());
    assert!(response["note"].as_str().is_some());
}

#[test]
fn database_status_command_reports_applied_migrations() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_database_status", json!({}));

    assert_eq!(response["migrations_applied"], 3);
    assert_eq!(response["latest_version"], 3);
    assert_eq!(response["foreign_keys_enabled"], true);
    assert!(
        response["db_file"]
            .as_str()
            .unwrap()
            .ends_with("multinet-cybershield-ai.db"),
        "unexpected database path: {response}"
    );
}

#[test]
fn interface_inventory_command_returns_real_adapters() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "list_network_interfaces", json!({}));

    let interfaces = response
        .as_array()
        .expect("list_network_interfaces must return an array");

    // The machine running the test always has at least the loopback adapter.
    assert!(
        !interfaces.is_empty(),
        "the operating system must report at least one adapter"
    );

    for interface in interfaces {
        // Every record must carry the platform identifier and classification.
        assert!(interface["id"].is_string());
        assert!(interface["name"].is_string());
        assert!(interface["kind"].is_string());
        assert!(interface["isUp"].is_boolean());
        assert!(interface["isPhysical"].is_boolean());
        assert!(interface["ipv4Addresses"].is_array());
        assert!(interface["ipv6Addresses"].is_array());
    }
}

#[test]
fn traffic_snapshot_command_returns_a_sampler_backed_snapshot() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_traffic_snapshot", json!({}));

    assert!(response["sampledAt"].is_string());
    assert_eq!(response["source"], "GetIfTable2");
    assert!(response["interfaces"].is_array());
    assert!(response["monitorActive"].is_boolean());
    assert!(response["peakCombinedBps"].is_number());
}

/// A fresh installation has no recorded sample yet. The peak query must still
/// return a number instead of failing to decode an INTEGER into an `f64`.
#[test]
fn traffic_snapshot_command_works_before_the_first_sample_is_recorded() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_traffic_snapshot", json!({}));

    assert_eq!(response["peakCombinedBps"], 0.0);
    assert_eq!(response["sessionRxBytes"], 0);
    assert_eq!(response["totalTxBytes"], 0);
}

/// The history aggregation must group by the bucket size, not by the bucket
/// count, otherwise a one-hour window produces 600 one-second groups instead
/// of six ten-minute groups.
#[test]
fn traffic_history_aggregates_into_fixed_ten_minute_buckets() {
    let (app, webview) = build_test_app();
    let pool = app.state::<DatabaseState>().pool.clone();

    // Two samples 20 minutes apart must land in different ten-minute buckets.
    // They are placed relative to "now" so both history windows can see them.
    let now = time::OffsetDateTime::now_utc();
    let stamps = [
        now - time::Duration::minutes(30),
        now - time::Duration::minutes(10),
    ];
    for (index, sampled_at) in stamps.iter().enumerate() {
        let sampled_at = sampled_at
            .format(&time::format_description::well_known::Rfc3339)
            .expect("timestamp");
        tauri::async_runtime::block_on(async {
            sqlx::query(
                "INSERT INTO traffic_samples \
                 (interface_id, interface_index, rx_bytes, tx_bytes, rx_rate_bps, tx_rate_bps, \
                  counter_reset, sampled_at, source) \
                 VALUES ('*', 0, $1, $1, 1000.0, 500.0, 0, $2, 'GetIfTable2')",
            )
            .bind(index as i64 * 1000)
            .bind(sampled_at)
            .execute(&pool)
            .await
            .expect("sample insert");
        });
    }

    let stored = tauri::async_runtime::block_on(async {
        sqlx::query("SELECT COUNT(*) AS n FROM traffic_samples")
            .fetch_one(&pool)
            .await
            .expect("count")
            .try_get::<i64, _>("n")
            .expect("n")
    });
    assert_eq!(stored, 2, "the samples must be persisted");

    // A 24-hour window is aggregated, so the two samples become two buckets.
    let response = invoke_ok(
        &webview,
        "get_traffic_history",
        json!({ "interfaceId": "*", "seconds": 86400 }),
    );
    let samples = response.as_array().expect("history must be an array");
    assert_eq!(samples.len(), 2, "expected two ten-minute buckets");

    // A one-hour window over the same data keeps both samples separately.
    let response = invoke_ok(
        &webview,
        "get_traffic_history",
        json!({ "interfaceId": "*", "seconds": 3600 }),
    );
    let samples = response.as_array().expect("history must be an array");
    assert_eq!(samples.len(), 2);
}

#[test]
fn traffic_history_command_rejects_out_of_range_windows() {
    let (_app, webview) = build_test_app();
    let error = invoke(&webview, "get_traffic_history", json!({ "seconds": 5 }))
        .expect_err("an out-of-range window must fail closed");

    assert_eq!(error["code"], "invalid_input");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("between 60 and 604800"));
}

#[test]
fn traffic_history_command_accepts_a_valid_window() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(
        &webview,
        "get_traffic_history",
        json!({ "interfaceId": null, "seconds": 300 }),
    );
    assert!(response.is_array());
}

#[test]
fn traffic_monitor_status_reports_the_running_sampler() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_traffic_monitor_status", json!({}));

    assert_eq!(response["running"], true);
    assert_eq!(response["source"], "GetIfTable2");
}

#[test]
fn traffic_monitor_can_be_stopped_and_resumed() {
    let (_app, webview) = build_test_app();

    let stopped = invoke_ok(
        &webview,
        "set_traffic_monitor_running",
        json!({ "running": false }),
    );
    assert_eq!(stopped["running"], false);

    let resumed = invoke_ok(
        &webview,
        "set_traffic_monitor_running",
        json!({ "running": true }),
    );
    assert_eq!(resumed["running"], true);
}

#[test]
fn speed_test_lifecycle_reports_an_unknown_run_as_not_found() {
    let (_app, webview) = build_test_app();
    let error = invoke(
        &webview,
        "get_speed_test_status",
        json!({ "id": "does-not-exist" }),
    )
    .expect_err("an unknown run id must fail closed");

    assert_eq!(error["code"], "not_found");
    assert!(error["message"].as_str().unwrap().contains("not found"));
}

#[test]
fn speed_test_history_is_empty_until_a_real_run_is_recorded() {
    let (_app, webview) = build_test_app();
    let response = invoke_ok(&webview, "get_speed_test_history", json!({}));

    assert!(
        response.as_array().unwrap().is_empty(),
        "no speed test may appear in history before a verified run happened"
    );
}

#[test]
fn speed_test_start_rejects_a_non_https_server() {
    let (_app, webview) = build_test_app();
    let error = invoke(
        &webview,
        "start_speed_test",
        json!({
            "request": {
                "serverUrl": "http://127.0.0.1:9443",
                "interfaceId": null,
                "interfaceLocalIp": null,
                "downloadBytes": 1024,
                "uploadBytes": 1024,
                "pingSamples": 2,
                "timeoutSecs": 5,
                "allowSelfSignedLoopback": false
            }
        }),
    )
    .expect_err("a plaintext speed test server must be rejected");

    assert_eq!(error["code"], "invalid_input");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("must use HTTPS"));
}

#[test]
fn speed_test_start_rejects_self_signed_certificates_for_remote_servers() {
    let (_app, webview) = build_test_app();
    let error = invoke(
        &webview,
        "start_speed_test",
        json!({
            "request": {
                "serverUrl": "https://speed.example.com",
                "interfaceId": null,
                "interfaceLocalIp": null,
                "downloadBytes": 1024,
                "uploadBytes": 1024,
                "pingSamples": 2,
                "timeoutSecs": 5,
                "allowSelfSignedLoopback": true
            }
        }),
    )
    .expect_err("a self-signed certificate must only be accepted for loopback");

    assert_eq!(error["code"], "invalid_input");
    assert!(error["message"].as_str().unwrap().contains("loopback"));
}

/// The traffic sampler and the adapter inventory must describe the same set of
/// adapters. `GetIfTable2` also reports NDIS filter and QoS pseudo-interfaces;
/// counting those would make the two screens disagree.
#[test]
fn traffic_snapshot_only_reports_real_adapters() {
    let (_app, webview) = build_test_app();
    let snapshot = invoke_ok(&webview, "get_traffic_snapshot", json!({}));
    let inventory = invoke_ok(&webview, "list_network_interfaces", json!({}));

    let sampled: Vec<&str> = snapshot["interfaces"]
        .as_array()
        .expect("interfaces must be an array")
        .iter()
        .map(|i| i["name"].as_str().unwrap_or_default())
        .collect();
    let discovered: Vec<&str> = inventory
        .as_array()
        .expect("inventory must be an array")
        .iter()
        .map(|i| i["name"].as_str().unwrap_or_default())
        .collect();

    for name in &sampled {
        assert!(
            discovered.contains(name),
            "the sampler reported `{name}`, which is not a discovered adapter"
        );
    }

    // The only adapter that may be absent from the sampler is the loopback
    // interface, which the sampler deliberately excludes.
    assert!(
        sampled.len() <= discovered.len(),
        "the sampler reported more interfaces than the inventory holds"
    );
}

#[test]
fn incident_and_security_event_queries_return_real_rows_only() {
    let (_app, webview) = build_test_app();

    let incidents = invoke_ok(&webview, "get_incidents", json!({}));
    assert!(incidents.is_array());

    let events = invoke_ok(&webview, "get_security_events", json!({}));
    assert!(events.is_array());
}

/// The diagnostics state must be reachable as managed state so the sampler
/// can be started against the same pool the commands read from.
#[test]
fn diagnostics_state_is_managed_by_the_application() {
    let (app, _webview) = build_test_app();
    let state = app.state::<Arc<DiagnosticsState>>();
    // A fresh store is empty; the sampler fills it at runtime.
    assert_eq!(state.tracked_speed_test_runs(), 0);
}

#[test]
fn database_state_is_managed_by_the_application() {
    let (app, _webview) = build_test_app();
    let state = app.state::<DatabaseState>();
    assert!(state.pool.is_closed() == false);
}
