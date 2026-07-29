use longhorn_core::{DropZoneId, ScreenPoint, ScreenRect, WindowId};

use crate::{ClientEpoch, DropZone, LeaseGeneration, TransferClientId, TransferDuration};

/// Complete replacement lease publication for one current client window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeasePublication {
    window_id: WindowId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    generation: LeaseGeneration,
    lifetime: TransferDuration,
    window_outer_bounds: ScreenRect,
    zones: Vec<DropZone>,
}

impl LeasePublication {
    /// Constructs a publication. The coordinator validates it before replace.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        window_id: WindowId,
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        generation: LeaseGeneration,
        lifetime: TransferDuration,
        window_outer_bounds: ScreenRect,
        zones: Vec<DropZone>,
    ) -> Self {
        Self {
            window_id,
            client_id,
            client_epoch,
            generation,
            lifetime,
            window_outer_bounds,
            zones,
        }
    }

    pub(crate) const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    pub(crate) const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    pub(crate) const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    pub(crate) const fn generation(&self) -> LeaseGeneration {
        self.generation
    }

    pub(crate) const fn lifetime(&self) -> TransferDuration {
        self.lifetime
    }

    pub(crate) const fn window_outer_bounds(&self) -> ScreenRect {
        self.window_outer_bounds
    }

    pub(crate) fn zones(&self) -> &[DropZone] {
        self.zones.as_slice()
    }
}

/// Fresh current managed-window bounds used for target resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveTransferWindow {
    window_id: WindowId,
    outer_bounds: ScreenRect,
}

impl LiveTransferWindow {
    /// Constructs one fresh managed-window observation.
    #[must_use]
    pub const fn new(window_id: WindowId, outer_bounds: ScreenRect) -> Self {
        Self {
            window_id,
            outer_bounds,
        }
    }

    /// Returns the stable managed-window identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns current outer bounds.
    #[must_use]
    pub const fn outer_bounds(&self) -> ScreenRect {
        self.outer_bounds
    }
}

/// Caller-selected target-resolution path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetSelector {
    /// Resolve one current leased zone identity.
    ExplicitZone(DropZoneId),
    /// Host-hit-test one current global screen-DIP point.
    ScreenPoint(ScreenPoint),
}

/// Stable target-resolution evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetResolutionPath {
    /// Selected by explicit zone identity.
    ExplicitZone,
    /// Selected by fresh window and zone hit-testing.
    ScreenPoint,
}

/// One current deterministic leased target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTransferTarget {
    path: TargetResolutionPath,
    window_id: WindowId,
    zone: DropZone,
}

impl ResolvedTransferTarget {
    pub(crate) const fn new(
        path: TargetResolutionPath,
        window_id: WindowId,
        zone: DropZone,
    ) -> Self {
        Self {
            path,
            window_id,
            zone,
        }
    }

    /// Returns the resolution path.
    #[must_use]
    pub const fn path(&self) -> TargetResolutionPath {
        self.path
    }

    /// Returns the current target window.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the complete leased target evidence.
    #[must_use]
    pub const fn zone(&self) -> &DropZone {
        &self.zone
    }
}
