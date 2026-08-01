use longhorn_core::WindowId;
use longhorn_native_content::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentIslandId,
    NativeContentMechanism,
};

/// Honest capabilities of the backing-surface coordination layer.
pub const BACKING_SURFACE_CAPABILITIES: MechanismCapabilities = MechanismCapabilities::new(
    NativeContentMechanism::BackingSurface,
    InputRoutingMode::RendererForwarded,
    false,
    DetachPolicy::Reversible,
    false,
    false,
);

/// Immutable island and host mapping for one backing surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackingSurfaceSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
}

impl BackingSurfaceSpec {
    /// Creates one mapping without storage, renderer, or input authority.
    #[must_use]
    pub const fn new(island_id: NativeContentIslandId, host_window_id: WindowId) -> Self {
        Self {
            island_id,
            host_window_id,
        }
    }

    /// Returns shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the logical host-window binding.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }
}
