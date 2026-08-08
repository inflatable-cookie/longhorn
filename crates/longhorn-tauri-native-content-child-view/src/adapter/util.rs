//! Generation helpers.

use longhorn_native_content::AttachGeneration;

use crate::ChildViewError;

use super::{AdapterState, Attachment};

pub(crate) fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, ChildViewError> {
    compare_generation(state.latest_generation, generation)?;
    if state.retired_generation == Some(generation) {
        return Err(ChildViewError::GenerationRetired(generation));
    }
    let attachment = state
        .attachment
        .as_mut()
        .ok_or(ChildViewError::NotAttached)?;
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
) -> Result<(), ChildViewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(ChildViewError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(ChildViewError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

pub(crate) fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), ChildViewError> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(ChildViewError::StaleGeneration { current, supplied });
    }
    if supplied == current || current.checked_next().ok() == Some(supplied) {
        Ok(())
    } else {
        Err(ChildViewError::FutureGeneration { current, supplied })
    }
}

pub(crate) fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> ChildViewError {
    if supplied < current {
        ChildViewError::StaleGeneration { current, supplied }
    } else {
        ChildViewError::FutureGeneration { current, supplied }
    }
}
