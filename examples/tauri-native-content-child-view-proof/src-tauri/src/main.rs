//! Packaged macOS proof for the production Tauri child-view adapter.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod evidence;
mod proof;
mod server;

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use evidence::EvidenceLog;
use serde_json::json;
use tauri::{Manager, Wry, window::WindowBuilder};

fn setup(app: &mut tauri::App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let evidence_root = evidence_root(app)?;
    let log = Arc::new(EvidenceLog::new(evidence_root)?);
    log.record(
        "proof_started",
        json!({
            "tauri_version": "2.10.3",
            "platform": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "unstable_api": ["WebviewBuilder", "Window::add_child"],
            "adapter": "longhorn-tauri-native-content-child-view",
        }),
    )?;
    let host = WindowBuilder::new(app, "host")
        .title("Longhorn production child-view proof")
        .inner_size(760.0, 520.0)
        .visible(true)
        .focused(false)
        .build()?;
    let handle = app.handle().clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        if let Err(error) = proof::run(handle.clone(), host, log.clone()) {
            let _ = log.record("proof_failed", json!({"detail": error}));
            handle.exit(1);
        }
    });
    Ok(())
}

fn evidence_root(app: &tauri::App<Wry>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("LONGHORN_CHILD_VIEW_EVIDENCE_DIR") {
        let root = PathBuf::from(root);
        fs::create_dir_all(&root)?;
        return Ok(root);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(app
        .path()
        .app_data_dir()?
        .join("packaged-runs")
        .join(timestamp.to_string()))
}

fn main() {
    tauri::Builder::default()
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("could not run packaged Longhorn production child-view proof");
}
