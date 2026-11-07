#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_foundation_status])
        .run(tauri::generate_context!())
        .expect("error while running MultiNet CyberShield AI");
}
