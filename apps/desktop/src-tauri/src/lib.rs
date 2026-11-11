//! MultiNet CyberShield AI — desktop shell library.
//!
//! The Tauri application is split into a library and a thin binary so the
//! native command surface can be exercised by integration tests through the
//! real Tauri IPC dispatch path (`tauri::test::get_ipc_response`) instead of
//! being re-implemented in a mock.

pub mod db;
pub mod diagnostics;

use db::{DatabaseState, SqliteAuditSink};
use security_policy::HumanApprovalPolicy;
use serde::Serialize;
use sqlx::Row;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

#[derive(Serialize)]
struct FoundationStatus {
    product: &'static str,
    version: &'static str,
    phase: &'static str,
    note: &'static str,
}

#[tauri::command]
fn get_foundation_status() -> FoundationStatus {
    FoundationStatus {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1 - Foundation and Architecture",
        note: "No live network/security operations, no fake metrics, no system mutations.",
    }
}

#[tauri::command]
async fn get_database_status(
    state: tauri::State<'_, DatabaseState>,
) -> Result<db::DbStatus, String> {
    // Legacy string-error contract (pre-existing command): preserved to not
    // destabilize existing frontend behavior.
    let state = state.inner();
    db::database_status(state).await.map_err(|e| e.to_string())
}

/// Structured error contract — applied to NEW commands only (Day 10 onward).
/// Existing commands keep their previous error shape.
#[derive(Debug, Serialize)]
pub struct IpcError {
    code: String,
    message: String,
    retryable: bool,
}

impl IpcError {
    pub(crate) fn database_unavailable(message: String) -> Self {
        Self {
            code: "database_unavailable".into(),
            message,
            retryable: true,
        }
    }

    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input".into(),
            message: message.into(),
            retryable: false,
        }
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found".into(),
            message: message.into(),
            retryable: false,
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error".into(),
            message: message.into(),
            retryable: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IncidentRecordDto {
    id: String,
    title: String,
    severity: String,
    category: Option<String>,
    source: Option<String>,
    created_at: String,
    assigned_to: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SecurityEventDto {
    id: String,
    source: String,
    severity: String,
    category: Option<String>,
    summary: String,
    occurred_at: String,
    ingested_at: String,
}

/// Read-only incident inventory. Returns real rows only; an empty result is a
/// valid result — no record is ever synthesized on the backend.
#[tauri::command]
async fn get_incidents(
    state: tauri::State<'_, DatabaseState>,
) -> Result<Vec<IncidentRecordDto>, IpcError> {
    let rows = sqlx::query(
        "SELECT id, title, severity, category, source, opened_at, assigned_to, status \
         FROM incidents ORDER BY opened_at DESC, id ASC LIMIT 200",
    )
    .fetch_all(&state.inner().pool)
    .await
    .map_err(|e| IpcError::database_unavailable(e.to_string()))?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        records.push(IncidentRecordDto {
            id: row.try_get("id").map_err(db_read_error)?,
            title: row.try_get("title").map_err(db_read_error)?,
            severity: row.try_get("severity").map_err(db_read_error)?,
            category: row.try_get("category").map_err(db_read_error)?,
            source: row.try_get("source").map_err(db_read_error)?,
            created_at: row.try_get("opened_at").map_err(db_read_error)?,
            assigned_to: row.try_get("assigned_to").map_err(db_read_error)?,
            status: row.try_get("status").map_err(db_read_error)?,
        });
    }
    Ok(records)
}

/// Read-only security-event inventory. Returns real rows only; an empty result
/// is a valid result — no event is ever synthesized on the backend.
#[tauri::command]
async fn get_security_events(
    state: tauri::State<'_, DatabaseState>,
) -> Result<Vec<SecurityEventDto>, IpcError> {
    let rows = sqlx::query(
        "SELECT id, source, severity, category, summary, occurred_at, ingested_at \
         FROM security_events ORDER BY occurred_at DESC, id ASC LIMIT 200",
    )
    .fetch_all(&state.inner().pool)
    .await
    .map_err(|e| IpcError::database_unavailable(e.to_string()))?;

    let mut events = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        events.push(SecurityEventDto {
            id: row.try_get("id").map_err(db_read_error)?,
            source: row.try_get("source").map_err(db_read_error)?,
            severity: row.try_get("severity").map_err(db_read_error)?,
            category: row.try_get("category").map_err(db_read_error)?,
            summary: row.try_get("summary").map_err(db_read_error)?,
            occurred_at: row.try_get("occurred_at").map_err(db_read_error)?,
            ingested_at: row.try_get("ingested_at").map_err(db_read_error)?,
        });
    }
    Ok(events)
}

fn db_read_error(e: sqlx::Error) -> IpcError {
    IpcError {
        code: "database_read_failed".into(),
        message: e.to_string(),
        retryable: true,
    }
}

/// Security engine state: the Phase 1 `HumanApprovalPolicy` is now backed by the
/// durable `SqliteAuditSink`, so every denial, approval request and refusal is
/// persisted before the policy returns its result. No command may bypass this
/// policy to perform a system mutation.
#[allow(dead_code)] // wired into managed state; first consumer command lands Day 11+
struct SecurityState {
    policy: HumanApprovalPolicy<SqliteAuditSink>,
}

/// Builds the application with the full native command surface registered.
///
/// Shared by the production binary and the integration tests so both exercise
/// exactly the same command list.
pub fn build_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        get_foundation_status,
        get_database_status,
        get_incidents,
        get_security_events,
        diagnostics::get_traffic_monitor_status,
        diagnostics::set_traffic_monitor_running,
        diagnostics::get_traffic_snapshot,
        diagnostics::get_traffic_history,
        diagnostics::list_network_interfaces,
        diagnostics::start_speed_test,
        diagnostics::get_speed_test_status,
        diagnostics::cancel_speed_test,
        diagnostics::get_speed_test_history
    ])
}

/// Resolves the directory that holds the application database.
pub fn resolve_data_dir<R: tauri::Runtime>(handle: &tauri::AppHandle<R>) -> PathBuf {
    std::env::var_os("MULTINET_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
        })
}

/// Starts the application: database, security policy and the always-on
/// traffic sampler are installed as managed state.
pub fn run() {
    build_app(tauri::Builder::default())
        .setup(|app| {
            let data_dir = resolve_data_dir(app.handle());
            let database_state =
                tauri::async_runtime::block_on(db::initialize_database(&data_dir))?;
            let policy =
                HumanApprovalPolicy::new(SqliteAuditSink::new(database_state.pool.clone()));
            let diagnostics = Arc::new(diagnostics::DiagnosticsState::new());
            diagnostics::spawn_traffic_monitor(
                Arc::clone(&diagnostics),
                database_state.pool.clone(),
            );
            app.manage(SecurityState { policy });
            app.manage(database_state);
            app.manage(diagnostics);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MultiNet CyberShield AI");
}
