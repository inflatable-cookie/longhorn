use longhorn_display::{
    DisplayIdAllocator, KnownDisplayRegistry, ReconcileError, Reconciliation, reconcile_displays,
};
use longhorn_windowing::{
    PlacementPolicy, PlacementResolutionError, SavedWindowPlacement, WindowPlacementResolution,
    WindowRole, restore_window_placement,
};
use tauri::{AppHandle, Runtime};

use crate::{
    DesktopObservation, DisplayMetadataProvider, TauriObservationError, observe_tauri_desktop,
};

/// Canonical display reconciliation plus one pure restore result.
pub struct TauriWindowRestore {
    reconciliation: Reconciliation,
    placement: WindowPlacementResolution,
}

impl TauriWindowRestore {
    /// Returns the updated persistent display registry and current inventory.
    #[must_use]
    pub const fn reconciliation(&self) -> &Reconciliation {
        &self.reconciliation
    }

    /// Returns the saved, fallback, or unavailable placement result.
    #[must_use]
    pub const fn placement(&self) -> &WindowPlacementResolution {
        &self.placement
    }

    /// Consumes the result.
    #[must_use]
    pub fn into_parts(self) -> (Reconciliation, WindowPlacementResolution) {
        (self.reconciliation, self.placement)
    }
}

/// Restore planning failure across observation, correlation, or placement.
#[derive(Debug)]
pub enum TauriWindowRestoreError<AllocatorError> {
    /// Tauri observation or global geometry mapping failed.
    Observation(TauriObservationError),
    /// Canonical display correlation failed.
    Reconciliation(ReconcileError<AllocatorError>),
    /// Pure placement geometry failed.
    Placement(PlacementResolutionError),
}

/// Reconciles an existing observation and restores one saved placement.
pub fn plan_window_restore_from_observation<A>(
    saved: &SavedWindowPlacement,
    registry: &KnownDisplayRegistry,
    observation: &DesktopObservation,
    allocator: &mut A,
    role: WindowRole,
    policy: PlacementPolicy,
) -> Result<TauriWindowRestore, TauriWindowRestoreError<A::Error>>
where
    A: DisplayIdAllocator,
{
    let reconciliation =
        reconcile_displays(registry, observation.displays().iter().cloned(), allocator)
            .map_err(TauriWindowRestoreError::Reconciliation)?;
    let placement = restore_window_placement(saved, reconciliation.inventory(), role, policy)
        .map_err(TauriWindowRestoreError::Placement)?;
    Ok(TauriWindowRestore {
        reconciliation,
        placement,
    })
}

/// Observes current Tauri displays, reconciles identity, and restores placement.
#[allow(clippy::too_many_arguments)]
pub fn plan_tauri_window_restore<R, A>(
    app: &AppHandle<R>,
    saved: &SavedWindowPlacement,
    registry: &KnownDisplayRegistry,
    metadata_provider: &mut impl DisplayMetadataProvider,
    allocator: &mut A,
    role: WindowRole,
    policy: PlacementPolicy,
) -> Result<TauriWindowRestore, TauriWindowRestoreError<A::Error>>
where
    R: Runtime,
    A: DisplayIdAllocator,
{
    let observation = observe_tauri_desktop(app, &[], metadata_provider)
        .map_err(TauriWindowRestoreError::Observation)?;
    plan_window_restore_from_observation(saved, registry, &observation, allocator, role, policy)
}
