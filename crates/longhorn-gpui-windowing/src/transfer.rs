//! Cross-window transfer readback for the GPUI host.
//!
//! Contract 020's last unproven claim. The host's whole contribution to a
//! cross-window drag is one thing: *where every managed window currently is*.
//! [`TransferCoordinator::attempt_target_resolution`] takes
//! `&[LiveTransferWindow]` and decides the rest, so a backend that can observe
//! its windows can participate.
//!
//! That is smaller than the Tauri adapter suggests.
//! `longhorn-tauri-transfer` is 2,600 lines, and nearly all of it is the
//! webview command surface — invoke handlers, projections, caller authority.
//! None of that is the transfer decision, and none of it is needed here: a
//! GPUI application calls Longhorn directly and has no IPC boundary to police.
//!
//! # Freshness
//!
//! Every window is observed at the moment of the call. A snapshot taken when
//! the drag started would resolve a drop against where windows *were*, and a
//! window moved mid-drag is exactly when a stale answer is wrong.

use longhorn_core::WindowId;
use longhorn_transfer::LiveTransferWindow;

use crate::{GpuiWindowBackend, GpuiWindowError, GpuiWindowKey};

/// Observes every supplied window and reports its current outer bounds.
///
/// The pairs are `(window id, key)` — the same association the lifecycle host
/// and the apply path already hold, passed in rather than looked up so this
/// borrows nothing and can be called from either.
///
/// A window that cannot be observed fails the whole call rather than being
/// dropped from the list. A silently short list is a drop resolved against a
/// desktop missing one window, which reads as "no target" and loses the
/// transfer with no diagnostic.
pub fn live_transfer_windows<'a>(
    backend: &mut impl GpuiWindowBackend,
    windows: impl IntoIterator<Item = (&'a WindowId, GpuiWindowKey)>,
) -> Result<Vec<LiveTransferWindow>, GpuiWindowError> {
    windows
        .into_iter()
        .map(|(window_id, key)| {
            let facts = backend.observe(key)?;
            let bounds = facts.bounds().to_screen_rect().map_err(|error| {
                GpuiWindowError::new(format!("window {window_id} has unusable bounds: {error}"))
            })?;
            Ok(LiveTransferWindow::new(window_id.clone(), bounds))
        })
        .collect()
}
