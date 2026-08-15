//! Generation helpers.

use longhorn_native_content::{AttachGeneration, compare_generation, gate_attached};

use crate::IsolatedWindowError;

use super::{AdapterState, Attachment};

pub(crate) fn current_attachment_mut<H>(
    state: &mut AdapterState<H>,
    generation: AttachGeneration,
) -> Result<&mut Attachment<H>, IsolatedWindowError> {
    compare_generation(state.latest_generation, generation)?;
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
