#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod deterministic_renderer;
mod evidence;
mod native_macos;
mod proof;
mod runtime;

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let root = evidence_root(app)?;
            let log = Arc::new(evidence::EvidenceLog::new(root)?);
            let controller = app
                .get_webview_window("controller")
                .ok_or_else(|| std::io::Error::other("controller window is missing"))?;
            let native_scale = controller.scale_factor()?;
            let scale = model_scale(native_scale).map_err(std::io::Error::other)?;
            log.record(
                "proof_started",
                json!({
                    "tauri_version": "2.10.3",
                    "platform": std::env::consts::OS,
                    "architecture": std::env::consts::ARCH,
                    "native_scale": native_scale,
                    "fixture": "controlled full-host NSView and deterministic consumer renderer",
                    "detach_policy": "reversible",
                }),
            )?;
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(180));
                if let Err(error) = proof::run(app_handle.clone(), scale, log.clone()) {
                    let _ = log.record("proof_failed", json!({"detail": error}));
                    app_handle.exit(1);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("could not run packaged Longhorn backing-surface proof");
}

fn evidence_root(app: &tauri::App<tauri::Wry>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("LONGHORN_BACKING_SURFACE_EVIDENCE_DIR") {
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

fn model_scale(value: f64) -> Result<longhorn_core::ScaleFactor, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid native scale {value}"));
    }
    longhorn_core::ScaleFactor::from_thousandths((value * 1_000.0).round() as u32)
        .map_err(|error| error.to_string())
}
