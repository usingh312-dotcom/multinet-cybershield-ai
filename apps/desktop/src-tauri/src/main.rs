#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use serde::Serialize;
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
        phase: "Phase 1 — Foundation and Architecture",
        note: "No live network/security operations, no fake metrics, no system mutations.",
    }
}

#[tauri::command]
async fn get_database_status(state: tauri::State<'_, db::DatabaseState>) -> Result<db::DbStatus, String> {
    let state = state.inner();
    db::database_status(state)
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_foundation_status,
            get_database_status
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let database_state = tauri::async_runtime::block_on(db::initialize_database(&data_dir))?;
            app.manage(database_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MultiNet CyberShield AI");
}
