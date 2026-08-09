use longhorn_core::{LiveWindowMetrics, WindowId, WindowPlacement};
use serde::{Deserialize, Serialize};

use crate::{HostCapabilities, HostWindowHandle, ResolvedWindowPlacement};

use super::ApplyGeneration;

/// Desired host-facing state for one resolved logical window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DesiredWindow {
    window_id: WindowId,
    placement: WindowPlacement,
    maximized: bool,
    visible: bool,
}

impl DesiredWindow {
    /// Constructs desired state from explicit placement output.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        placement: WindowPlacement,
        maximized: bool,
        visible: bool,
    ) -> Self {
        Self {
            window_id,
            placement,
            maximized,
            visible,
        }
    }

    /// Projects a Card 015 placement result into desired host state.
    #[must_use]
    pub fn from_resolved(placement: &ResolvedWindowPlacement, visible: bool) -> Self {
        Self::new(
            placement.window_id().clone(),
            placement.normal_placement(),
            placement.is_maximized(),
            visible,
        )
    }

    /// Returns stable domain identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns desired outer-origin plus inner-content-size placement.
    #[must_use]
    pub const fn placement(&self) -> WindowPlacement {
        self.placement
    }

    /// Returns desired maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns desired visibility.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }
}

/// One currently live managed host window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveWindow {
    window_id: Option<WindowId>,
    transport_handle: HostWindowHandle,
    metrics: LiveWindowMetrics,
    maximized: bool,
    visible: bool,
    focused: bool,
}

impl LiveWindow {
    /// Constructs a live host observation without deriving identity from its handle.
    #[must_use]
    pub const fn new(
        window_id: Option<WindowId>,
        transport_handle: HostWindowHandle,
        metrics: LiveWindowMetrics,
        maximized: bool,
        visible: bool,
        focused: bool,
    ) -> Self {
        Self {
            window_id,
            transport_handle,
            metrics,
            maximized,
            visible,
            focused,
        }
    }

    /// Returns stable domain identity when host bookkeeping has one.
    #[must_use]
    pub const fn window_id(&self) -> Option<&WindowId> {
        self.window_id.as_ref()
    }

    /// Returns opaque transport identity.
    #[must_use]
    pub const fn transport_handle(&self) -> &HostWindowHandle {
        &self.transport_handle
    }

    /// Returns live inner size and distinct outer bounds.
    #[must_use]
    pub const fn metrics(&self) -> LiveWindowMetrics {
        self.metrics
    }

    /// Returns current maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns current visibility.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns current focus state.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }
}

/// Caller authority for focus changes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "window_id")]
pub enum FocusPolicy {
    /// Do not change native focus.
    Preserve,
    /// Focus one named desired window after geometry and visibility operations.
    Focus(WindowId),
}

/// Explicit policy for a host slot that cannot be closed by inference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ProtectedPrimaryPolicy {
    /// This host has no protected primary slot.
    None,
    /// Preserve the named slot without changing its logical identity.
    Preserve {
        /// Opaque protected host handle.
        transport_handle: HostWindowHandle,
    },
    /// Reuse the named slot for one desired logical window.
    Reuse {
        /// Opaque protected host handle.
        transport_handle: HostWindowHandle,
        /// Explicit logical identity to assign.
        window_id: WindowId,
    },
}

/// Complete immutable input for one pure desired/live diff.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowDiffInput {
    desired_windows: Vec<DesiredWindow>,
    live_windows: Vec<LiveWindow>,
    capabilities: HostCapabilities,
    focus_policy: FocusPolicy,
    protected_primary: ProtectedPrimaryPolicy,
    generation: ApplyGeneration,
}

impl WindowDiffInput {
    /// Constructs a diff with focus preserved and no protected slot.
    #[must_use]
    pub fn new(
        desired_windows: impl IntoIterator<Item = DesiredWindow>,
        live_windows: impl IntoIterator<Item = LiveWindow>,
        capabilities: HostCapabilities,
        generation: ApplyGeneration,
    ) -> Self {
        Self {
            desired_windows: desired_windows.into_iter().collect(),
            live_windows: live_windows.into_iter().collect(),
            capabilities,
            focus_policy: FocusPolicy::Preserve,
            protected_primary: ProtectedPrimaryPolicy::None,
            generation,
        }
    }

    /// Sets explicit focus policy.
    #[must_use]
    pub fn with_focus_policy(mut self, policy: FocusPolicy) -> Self {
        self.focus_policy = policy;
        self
    }

    /// Sets explicit protected-primary policy.
    #[must_use]
    pub fn with_protected_primary(mut self, policy: ProtectedPrimaryPolicy) -> Self {
        self.protected_primary = policy;
        self
    }

    /// Replaces live evidence while preserving desired state and caller policy.
    #[must_use]
    pub fn with_live_windows(mut self, live_windows: impl IntoIterator<Item = LiveWindow>) -> Self {
        self.live_windows = live_windows.into_iter().collect();
        self
    }

    /// Makes one restore pass keep every desired window hidden and unfocused.
    #[must_use]
    pub fn for_hidden_restore(mut self) -> Self {
        for desired in &mut self.desired_windows {
            desired.visible = false;
        }
        self.focus_policy = FocusPolicy::Preserve;
        self
    }

    /// Replaces declared host capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: HostCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Returns desired host-facing state for every resolved logical window.
    ///
    /// A host adapter reads this when it must know a window's final placement
    /// before the window exists. GPUI is such a host: bounds, maximized state
    /// and initial focus are creation-time options, and two of the three
    /// cannot be changed afterwards, so its adapter cannot execute the plan's
    /// neutral-slot-then-mutate order literally. A host that can mutate after
    /// creating never reads this and is unaffected by its being visible.
    #[must_use]
    pub fn desired_windows(&self) -> &[DesiredWindow] {
        &self.desired_windows
    }

    pub(crate) fn live_windows(&self) -> &[LiveWindow] {
        &self.live_windows
    }

    pub(crate) const fn capabilities(&self) -> &HostCapabilities {
        &self.capabilities
    }

    pub(crate) const fn focus_policy(&self) -> &FocusPolicy {
        &self.focus_policy
    }

    pub(crate) const fn protected_primary(&self) -> &ProtectedPrimaryPolicy {
        &self.protected_primary
    }

    pub(crate) const fn generation(&self) -> ApplyGeneration {
        self.generation
    }
}
