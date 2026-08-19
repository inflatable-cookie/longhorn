//! Provider seam for native (non-webview) surfaces (contract 022).
//!
//! Non-webview content — GPUI, native-content islands — is visible in
//! screenshots only unless a native surface registers its own snapshot and
//! action handlers through this seam. No provider ships under contract 022:
//! a host composing nothing is not a gap (contract 020), and `None` at the
//! seam is a first-class composition.

use crate::{
    ClickRequest, DragRequest, PressRequest, ScrollRequest, SemanticNode, ToolError, TypeRequest,
};

/// One ref-addressed action routed to a native surface provider — the
/// shared vocabulary, so a host routes one shape to webview and provider
/// edges alike.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeSurfaceAction {
    /// Synthetic click on a resolved ref.
    Click(ClickRequest),
    /// Synthetic text entry into a resolved ref.
    Type(TypeRequest),
    /// Synthetic key press.
    Press(PressRequest),
    /// Synthetic scroll.
    Scroll(ScrollRequest),
    /// Synthetic in-page drag; untrusted events only, never OS-level.
    Drag(DragRequest),
}

/// The seam a native (non-webview) surface implements later to contribute
/// snapshot and action handling to the control surface.
///
/// Deliberately the smallest trait that admits a provider: one snapshot
/// contribution, one action dispatch, both failing through the shared
/// [`ToolError`] vocabulary. Refs a provider stamps are its own to resolve;
/// the core holds no ref table for providers either.
pub trait NativeSurfaceProvider: Send + Sync {
    /// Contributes this surface's semantic tree, refs stamped by the
    /// provider's own edge.
    fn snapshot(&self) -> Result<SemanticNode, ToolError>;

    /// Performs one ref-addressed action against this surface.
    fn perform(&self, action: NativeSurfaceAction) -> Result<(), ToolError>;
}
