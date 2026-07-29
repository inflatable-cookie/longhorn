use std::{error::Error, fmt};

use longhorn_core::{
    DisplayId, ScreenPoint, SurfaceId, TransferHostBindingId, WindowId, WindowPlacement,
};
use longhorn_transfer::DragSessionId;

/// Consumer-neutral request for one hidden, placed, ready Surface host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowProvisionRequest {
    session_id: DragSessionId,
    surface_id: SurfaceId,
    window_id: WindowId,
    display_id: DisplayId,
    drop_point: ScreenPoint,
    placement: WindowPlacement,
}

impl SurfaceWindowProvisionRequest {
    pub(crate) const fn new(
        session_id: DragSessionId,
        surface_id: SurfaceId,
        window_id: WindowId,
        display_id: DisplayId,
        drop_point: ScreenPoint,
        placement: WindowPlacement,
    ) -> Self {
        Self {
            session_id,
            surface_id,
            window_id,
            display_id,
            drop_point,
            placement,
        }
    }

    /// Returns the consumed transfer session.
    #[must_use]
    pub const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    /// Returns the Surface awaiting transfer.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    /// Returns the predeclared logical target window.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the consumer-selected display.
    #[must_use]
    pub const fn display_id(&self) -> &DisplayId {
        &self.display_id
    }

    /// Returns the empty-display drop point.
    #[must_use]
    pub const fn drop_point(&self) -> ScreenPoint {
        self.drop_point
    }

    /// Returns consumer-resolved placement with no package default.
    #[must_use]
    pub const fn placement(&self) -> WindowPlacement {
        self.placement
    }
}

/// Provisioning stage used by stable failure evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceWindowProvisionStage {
    /// Create a neutral hidden unmaximized native slot.
    CreateHidden,
    /// Apply consumer-resolved placement.
    Place,
    /// Wait for renderer or host readiness while hidden.
    Ready,
    /// Commit the prepared host after Surface publication.
    Commit,
    /// Close and unregister an uncommitted host.
    Cleanup,
}

/// Typed host provisioning failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowProvisionFailure {
    stage: SurfaceWindowProvisionStage,
    detail: String,
}

impl SurfaceWindowProvisionFailure {
    /// Constructs host-supplied failure evidence.
    #[must_use]
    pub fn new(stage: SurfaceWindowProvisionStage, detail: impl Into<String>) -> Self {
        Self {
            stage,
            detail: detail.into(),
        }
    }

    /// Returns the failed lifecycle stage.
    #[must_use]
    pub const fn stage(&self) -> SurfaceWindowProvisionStage {
        self.stage
    }

    /// Returns diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SurfaceWindowProvisionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceWindowProvisionFailure {}

/// Completed hidden creation, placement, and readiness evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowProvisionReceipt {
    window_id: WindowId,
    host_binding_id: TransferHostBindingId,
    display_id: DisplayId,
    placement: WindowPlacement,
}

impl SurfaceWindowProvisionReceipt {
    /// Records a target returned only after hidden placement and readiness.
    #[must_use]
    pub const fn hidden_ready(
        window_id: WindowId,
        host_binding_id: TransferHostBindingId,
        display_id: DisplayId,
        placement: WindowPlacement,
    ) -> Self {
        Self {
            window_id,
            host_binding_id,
            display_id,
            placement,
        }
    }

    /// Returns the prepared logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the prepared target binding.
    #[must_use]
    pub const fn host_binding_id(&self) -> &TransferHostBindingId {
        &self.host_binding_id
    }

    /// Returns the prepared display.
    #[must_use]
    pub const fn display_id(&self) -> &DisplayId {
        &self.display_id
    }

    /// Returns applied placement.
    #[must_use]
    pub const fn placement(&self) -> WindowPlacement {
        self.placement
    }
}

/// Host acknowledgement that prepared authority was committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowCommitReceipt {
    window_id: WindowId,
}

impl SurfaceWindowCommitReceipt {
    /// Constructs commit evidence for one prepared logical window.
    #[must_use]
    pub const fn new(window_id: WindowId) -> Self {
        Self { window_id }
    }

    /// Returns the committed logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }
}

/// Host acknowledgement that an uncommitted provision was removed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowCleanupReceipt {
    window_id: WindowId,
}

impl SurfaceWindowCleanupReceipt {
    /// Constructs cleanup evidence for one prepared logical window.
    #[must_use]
    pub const fn new(window_id: WindowId) -> Self {
        Self { window_id }
    }

    /// Returns the removed logical target.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }
}

/// Prepared target plus caller-retained cleanup authority.
#[derive(Debug)]
pub struct ProvisionedSurfaceWindow<A> {
    authority: A,
    receipt: SurfaceWindowProvisionReceipt,
}

impl<A> ProvisionedSurfaceWindow<A> {
    /// Constructs a prepared target from explicit authority and lifecycle evidence.
    #[must_use]
    pub const fn new(authority: A, receipt: SurfaceWindowProvisionReceipt) -> Self {
        Self { authority, receipt }
    }

    pub(crate) fn parts(self) -> (A, SurfaceWindowProvisionReceipt) {
        (self.authority, self.receipt)
    }
}

/// Injected consumer authority for dynamic Surface window lifecycle.
pub trait SurfaceWindowProvisioner {
    /// Opaque authority retained until commit or successful cleanup.
    type Authority;

    /// Creates, places, and readies a neutral target without revealing it.
    fn provision(
        &mut self,
        request: &SurfaceWindowProvisionRequest,
    ) -> Result<ProvisionedSurfaceWindow<Self::Authority>, SurfaceWindowProvisionFailure>;

    /// Commits the prepared target after authoritative Surface publication.
    fn commit(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCommitReceipt, SurfaceWindowProvisionFailure>;

    /// Closes and unregisters a target whose Surface publication failed.
    fn cleanup(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCleanupReceipt, SurfaceWindowProvisionFailure>;
}
