//! Generation and observation helpers.

use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, EffectiveFocus, EffectiveVisibility, InputRoutingMode,
    ObservationUpdate, ObservedGeometry, ObservedReadiness, compare_generation, gate_attached,
};

use crate::{BackingSurfaceError, BackingSurfaceSnapshot};

use super::{AdapterState, Attachment};

pub(crate) fn validate_snapshot(
    snapshot: &BackingSurfaceSnapshot,
) -> Result<(), BackingSurfaceError> {
    if matches!(
        snapshot.input_routing,
        InputRoutingMode::RendererForwarded | InputRoutingMode::Disabled
    ) {
        Ok(())
    } else {
        Err(BackingSurfaceError::UnsupportedInputMode)
    }
}

pub(crate) fn observation(
    generation: AttachGeneration,
    snapshot: &BackingSurfaceSnapshot,
    detaching: bool,
) -> ObservationUpdate {
    ObservationUpdate::new(
        generation,
        if !snapshot.native_storage_attached {
            AttachmentLifecycle::Failed
        } else if detaching {
            AttachmentLifecycle::Detaching
        } else {
            AttachmentLifecycle::Attached
        },
        if snapshot.native_storage_attached {
            ObservedReadiness::Ready
        } else {
            ObservedReadiness::NotReady
        },
        EffectiveVisibility::Unknown,
        EffectiveFocus::Unknown,
        ObservedGeometry::BackingSurface {
            storage_bounds: snapshot.storage_bounds,
            clip: snapshot.clip,
        },
        Some(snapshot.input_routing),
    )
}

/// Mechanism-specific extension (contract 017): native storage can outlive
/// host invalidation inside the invalidate-then-detach window, so a retained
/// attachment rejects further work for its invalidated generation.
pub(crate) fn reject_invalidated(
    invalidated: Option<AttachGeneration>,
    generation: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    if invalidated == Some(generation) {
        return Err(BackingSurfaceError::GenerationInvalidated(generation));
    }
    Ok(())
}

pub(crate) fn current_attachment<H>(
    state: &AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    reject_invalidated(state.invalidated_generation, generation)?;
    gate_attached(
        state.retired_generation,
        state
            .attachment
            .as_ref()
            .map(|attachment| attachment.generation),
        generation,
    )?;
    Ok(state
        .attachment
        .as_ref()
        .expect("validated attachment is current"))
}

pub(crate) fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    reject_invalidated(state.invalidated_generation, generation)?;
    gate_attached(
        state.retired_generation,
        state
            .attachment
            .as_ref()
            .map(|attachment| attachment.generation),
        generation,
    )?;
    Ok(state
        .attachment
        .as_mut()
        .expect("validated attachment is current"))
}
