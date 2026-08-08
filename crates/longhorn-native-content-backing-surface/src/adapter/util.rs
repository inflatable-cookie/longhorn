//! Generation and observation helpers.

use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, EffectiveFocus, EffectiveVisibility, InputRoutingMode,
    ObservationUpdate, ObservedGeometry, ObservedReadiness,
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

pub(crate) fn current_attachment<H>(
    state: &AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    if state.invalidated_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationInvalidated(generation));
    }
    if state.retired_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationRetired(generation));
    }
    let attachment = state
        .attachment
        .as_ref()
        .ok_or(BackingSurfaceError::NotAttached)?;
    if attachment.generation != generation {
        return Err(compare_attached_generation(
            attachment.generation,
            generation,
        ));
    }
    Ok(attachment)
}

pub(crate) fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, BackingSurfaceError> {
    compare_generation(state.latest_generation, generation)?;
    if state.invalidated_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationInvalidated(generation));
    }
    if state.retired_generation == Some(generation) {
        return Err(BackingSurfaceError::GenerationRetired(generation));
    }
    let attachment = state
        .attachment
        .as_mut()
        .ok_or(BackingSurfaceError::NotAttached)?;
    if attachment.generation != generation {
        return Err(compare_attached_generation(
            attachment.generation,
            generation,
        ));
    }
    Ok(attachment)
}

pub(crate) fn compare_generation(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(BackingSurfaceError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(BackingSurfaceError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

pub(crate) fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), BackingSurfaceError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(BackingSurfaceError::StaleGeneration { current, supplied });
    }
    if supplied == current || current.checked_next().ok() == Some(supplied) {
        Ok(())
    } else {
        Err(BackingSurfaceError::FutureGeneration { current, supplied })
    }
}

pub(crate) fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> BackingSurfaceError {
    if supplied < current {
        BackingSurfaceError::StaleGeneration { current, supplied }
    } else {
        BackingSurfaceError::FutureGeneration { current, supplied }
    }
}
