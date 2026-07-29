//! Packaged failure and geometry evidence.

mod scenarios;

use std::path::Path;

use longhorn_core::ScreenPoint;
use longhorn_tauri_transfer::{ManagedTransferRuntime, TauriTransferRuntime};
use longhorn_tauri_windowing::UniformScaleMapper;
use longhorn_windowing::HostWindowHandle;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager, Wry};

use crate::{domain::SOURCE_WINDOW_ID, host::ProofHost};

pub(crate) fn run(app: &AppHandle<Wry>, host: &ProofHost, root: &Path) -> Result<Value, String> {
    let failures = scenarios::run(&root.join("failure-matrix"))?;
    let geometry = geometry(app, host)?;
    let passed = failures
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && geometry
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    Ok(json!({
        "result": if passed { "passed" } else { "failed" },
        "failures": failures,
        "geometry": geometry,
    }))
}

fn geometry(app: &AppHandle<Wry>, host: &ProofHost) -> Result<Value, String> {
    let runtime = TauriTransferRuntime::new(host.window_host.clone(), UniformScaleMapper);
    let snapshot = runtime
        .snapshot(
            &HostWindowHandle::new(SOURCE_WINDOW_ID)
                .expect("proof Tauri label uses the opaque-id grammar"),
        )
        .map_err(|error| error.to_string())?;
    let mut windows = Vec::new();
    let mut all_contained = true;
    let mut all_half_open = true;
    for window in snapshot.windows() {
        let outer = window.outer_bounds();
        let content = window.content_bounds();
        let right = edge(outer.origin().x().get(), outer.size().width())?;
        let bottom = edge(outer.origin().y().get(), outer.size().height())?;
        let inner_corner = ScreenPoint::new(right - 1, bottom - 1);
        let right_edge = ScreenPoint::new(right, inner_corner.y().get());
        let bottom_edge = ScreenPoint::new(inner_corner.x().get(), bottom);
        let content_contained = outer.contains_rect(&content);
        let half_open = outer.contains_point(&inner_corner)
            && !outer.contains_point(&right_edge)
            && !outer.contains_point(&bottom_edge);
        all_contained &= content_contained;
        all_half_open &= half_open;
        let webview = app
            .get_webview_window(window.transport_handle().as_str())
            .ok_or_else(|| format!("managed webview {} disappeared", window.transport_handle()))?;
        windows.push(json!({
            "window_id": window.window_id(),
            "transport_handle": window.transport_handle(),
            "scale_factor": webview.scale_factor().map_err(|error| error.to_string())?,
            "outer_bounds": outer,
            "content_bounds": content,
            "content_within_outer_frame": content_contained,
            "half_open_boundary": {
                "inside_bottom_right": inner_corner,
                "right_edge_excluded": !outer.contains_point(&right_edge),
                "bottom_edge_excluded": !outer.contains_point(&bottom_edge),
                "passed": half_open,
            },
        }));
    }
    #[cfg(feature = "surface-mode")]
    let empty_display = {
        let point = host.screen_policy.drop_point();
        let outside_all = snapshot
            .windows()
            .iter()
            .all(|window| !window.outer_bounds().contains_point(&point));
        json!({
            "screen_point": point,
            "outside_all_managed_windows": outside_all,
            "explicit_policy_enabled": true,
        })
    };
    #[cfg(not(feature = "surface-mode"))]
    let empty_display = Value::Null;
    let empty_display_passed = empty_display
        .get("outside_all_managed_windows")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    Ok(json!({
        "passed": snapshot.windows().len() == 2
            && all_contained
            && all_half_open
            && empty_display_passed,
        "coordinate_space": "global_screen_dip",
        "managed_window_count": snapshot.windows().len(),
        "windows": windows,
        "empty_display": empty_display,
    }))
}

fn edge(origin: i32, extent: u32) -> Result<i32, String> {
    origin
        .checked_add(i32::try_from(extent).map_err(|_| "window extent exceeds i32".to_string())?)
        .ok_or_else(|| "window boundary overflows screen coordinates".to_string())
}
