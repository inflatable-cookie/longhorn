//! Packaged native proof for Longhorn's production isolated-window adapter.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod evidence;
mod helper;
mod native_macos;
mod proof;
mod runtime_bridge;

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use evidence::EvidenceLog;
use serde_json::json;
use tauri::Manager;

fn main() {
    let context = tauri::generate_context!();
    match helper::options_from_args() {
        Ok(Some(options)) => {
            helper::run(options, context);
            return;
        }
        Ok(None) => {}
        Err(error) => panic!("invalid isolated-window helper launch: {error}"),
    }
    tauri::Builder::default()
        .setup(|app| {
            let root = evidence_root(app)?;
            let log = Arc::new(EvidenceLog::new(root)?);
            let controller = app
                .get_webview_window("controller")
                .ok_or_else(|| std::io::Error::other("controller window is missing"))?;
            let native_scale = controller.scale_factor()?;
            let scale = scale_from_native(native_scale).map_err(std::io::Error::other)?;
            log.record(
                "proof_started",
                json!({
                    "tauri_version": "2.10.3",
                    "platform": std::env::consts::OS,
                    "architecture": std::env::consts::ARCH,
                    "native_scale": native_scale,
                    "helper": "same packaged executable",
                    "fixture": "controlled NSView child",
                }),
            )?;
            let app_handle = app.handle().clone();
            let executable = std::env::current_exe()?;
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                if let Err(error) = proof::run(app_handle.clone(), executable, scale, log.clone()) {
                    let _ = log.record("proof_failed", json!({"detail": error}));
                    app_handle.exit(1);
                }
            });
            Ok(())
        })
        .run(context)
        .expect("could not run packaged Longhorn isolated-window proof");
}

fn evidence_root(app: &tauri::App<tauri::Wry>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("LONGHORN_ISOLATED_WINDOW_EVIDENCE_DIR") {
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

fn scale_from_native(value: f64) -> Result<longhorn_core::ScaleFactor, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid native scale factor {value}"));
    }
    let thousandths = (value * 1000.0).round();
    if thousandths > f64::from(u32::MAX) {
        return Err(format!("native scale factor {value} exceeds model range"));
    }
    longhorn_core::ScaleFactor::from_thousandths(thousandths as u32)
        .map_err(|error| error.to_string())
}
