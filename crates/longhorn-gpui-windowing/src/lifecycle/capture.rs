use longhorn_core::{WindowId, WindowPlacement};
use longhorn_windowing::{CapturedDisplayAssociation, CapturedWindowPlacement};

/// Builds a capture from GPUI facts, with no display association.
///
/// GPUI cannot supply the physical bounds, work area or scale that
/// [`longhorn_windowing::CapturedDisplayEvidence`] requires, so a capture
/// taken from GPUI alone associates with no display. A product that needs the
/// association supplies the missing facts itself and constructs the evidence
/// directly.
pub fn capture_from_gpui_facts(
    window_id: &WindowId,
    facts: &crate::GpuiWindowFacts,
) -> Result<CapturedWindowPlacement, String> {
    let state = facts.bounds_state();
    // Longhorn's placement vocabulary has no fullscreen state. The bounds
    // GPUI reports for a fullscreen window are fullscreen geometry, and
    // persisting them as a windowed placement would restore the window as an
    // ordinary one at that size. Refuse here and let the caller decide.
    if state.is_fullscreen() {
        return Err("a fullscreen window has no windowed placement to persist".to_owned());
    }
    let restore = state.restore_bounds();

    // Longhorn's normal placement wants an *inner* size, and gpui's bounds are
    // outer. For a window that is not maximized the two are both available and
    // `content_size` is the right one; taking the outer extent instead
    // recorded a 560pt window as 592 on macOS, and applying that back grew it
    // by the titlebar every save-and-restore cycle. Measured, not reasoned:
    // the composition example persisted two real windows and the numbers
    // disagreed by exactly the frame.
    //
    // Maximized is the case that has no clean answer. `content_size` then
    // describes the *maximized* window while the restore bounds describe where
    // it will return to, so the restore extent stays the closest honest value
    // and the frame difference is accepted rather than hidden.
    let inner_size = if state.is_maximized() {
        restore.to_screen_size().map_err(|e| e.to_string())?
    } else {
        facts
            .content_size()
            .to_screen_size()
            .map_err(|e| e.to_string())?
    };
    let normal_placement = WindowPlacement::new(
        restore.to_screen_origin().map_err(|e| e.to_string())?,
        inner_size,
    );
    Ok(CapturedWindowPlacement::new(
        window_id.clone(),
        normal_placement,
        state.is_maximized(),
        CapturedDisplayAssociation::Unresolved,
    ))
}
