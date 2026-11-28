use longhorn_core::{ScreenSize, WindowId};
use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedNormalPlacement, CapturedWindowPlacement,
};

/// Builds a capture from GPUI facts, with no display association.
///
/// GPUI cannot supply the physical bounds, work area or scale that
/// [`longhorn_windowing::CapturedDisplayEvidence`] requires, so a capture
/// taken from GPUI alone associates with no display. A product that needs the
/// association supplies the missing facts itself and constructs the evidence
/// directly. While maximized, pass a previously observed or stored normal
/// content size when available; the capture records it as unknown otherwise.
pub fn capture_from_gpui_facts(
    window_id: &WindowId,
    facts: &crate::GpuiWindowFacts,
    retained_normal_size: Option<ScreenSize>,
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

    let inner_size = if state.is_maximized() {
        retained_normal_size
    } else {
        Some(
            facts
                .content_size()
                .to_screen_size()
                .map_err(|e| e.to_string())?,
        )
    };
    let normal_placement = CapturedNormalPlacement::new(
        restore.to_screen_origin().map_err(|e| e.to_string())?,
        inner_size,
    );
    Ok(CapturedWindowPlacement::from_normal(
        window_id.clone(),
        normal_placement,
        state.is_maximized(),
        CapturedDisplayAssociation::Unresolved,
    ))
}
