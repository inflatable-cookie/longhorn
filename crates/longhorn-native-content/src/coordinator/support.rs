use longhorn_core::{NativeContentRevision, WindowId};
use serde::{Deserialize, Serialize};

use crate::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, CoordinationError, DesiredState,
    DesiredUpdate, EffectiveFocus, EffectiveVisibility, NativeContentMechanism, ObservationUpdate,
    ObservedGeometry, ObservedReadiness, ObservedState, ReceiptError, StepExecution,
};
use crate::{
    plan::plan_transition,
    proposal::{decide_content_size, validate_content_size_proposal},
};



pub(crate) fn require_revision(
    current: NativeContentRevision,
    supplied: NativeContentRevision,
) -> Result<(), CoordinationError> {
    if current == supplied {
        Ok(())
    } else {
        Err(CoordinationError::StaleRevision { current, supplied })
    }
}

pub(crate) fn validate_desired_generation(
    desired: &DesiredState,
    observed: &ObservedState,
    update: &DesiredUpdate,
) -> Result<(), CoordinationError> {
    let current = desired.generation();
    let supplied = update.generation();
    if supplied < current {
        return Err(CoordinationError::StaleGeneration { current, supplied });
    }
    if supplied == current {
        return Ok(());
    }
    let next = current
        .checked_next()
        .map_err(|_| CoordinationError::GenerationOverflow)?;
    if supplied != next {
        return Err(CoordinationError::GenerationGap { current, supplied });
    }
    if !matches!(
        observed.lifecycle(),
        AttachmentLifecycle::Absent | AttachmentLifecycle::Failed
    ) {
        return Err(CoordinationError::GenerationStillAttached(
            observed.lifecycle(),
        ));
    }
    Ok(())
}

pub(crate) fn compare_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> Result<(), CoordinationError> {
    if supplied < current {
        Err(CoordinationError::StaleGeneration { current, supplied })
    } else if supplied > current {
        Err(CoordinationError::FutureGeneration { current, supplied })
    } else {
        Ok(())
    }
}

pub(crate) fn validate_observation_capabilities(
    desired: &DesiredState,
    update: &ObservationUpdate,
) -> Result<(), CoordinationError> {
    let capabilities = desired.capabilities();
    if !capabilities.observes_visibility() && update.visibility() != EffectiveVisibility::Unknown {
        return Err(CoordinationError::UnsupportedVisibilityObservation);
    }
    if !capabilities.observes_focus() && update.focus() != EffectiveFocus::Unknown {
        return Err(CoordinationError::UnsupportedFocusObservation);
    }
    if let Some(input) = update.input_routing() {
        capabilities.validate_input(input)?;
    }
    if update.readiness() == ObservedReadiness::Ready
        && update.lifecycle() != AttachmentLifecycle::Attached
    {
        return Err(CoordinationError::ReadinessWithoutAttachment);
    }
    if update.lifecycle() == AttachmentLifecycle::Absent
        && (!matches!(update.geometry(), ObservedGeometry::Unknown)
            || update.input_routing().is_some())
    {
        return Err(CoordinationError::AbsentWithNativeEvidence);
    }
    match (capabilities.mechanism(), update.geometry()) {
        (_, ObservedGeometry::Unknown)
        | (NativeContentMechanism::ChildView, ObservedGeometry::ChildBounds { .. })
        | (NativeContentMechanism::IsolatedWindow, ObservedGeometry::IsolatedContent { .. })
        | (NativeContentMechanism::BackingSurface, ObservedGeometry::BackingSurface { .. }) => {
            Ok(())
        }
        (mechanism, _) => Err(CoordinationError::GeometryMechanismMismatch { mechanism }),
    }
}

pub(crate) fn legal_transition(current: AttachmentLifecycle, proposed: AttachmentLifecycle) -> bool {
    current == proposed
        || matches!(
            (current, proposed),
            (AttachmentLifecycle::Absent, AttachmentLifecycle::Attaching)
                | (AttachmentLifecycle::Absent, AttachmentLifecycle::Attached)
                | (AttachmentLifecycle::Absent, AttachmentLifecycle::Failed)
                | (
                    AttachmentLifecycle::Attaching,
                    AttachmentLifecycle::Attached
                )
                | (
                    AttachmentLifecycle::Attaching,
                    AttachmentLifecycle::Detaching
                )
                | (AttachmentLifecycle::Attaching, AttachmentLifecycle::Failed)
                | (
                    AttachmentLifecycle::Attached,
                    AttachmentLifecycle::Detaching
                )
                | (AttachmentLifecycle::Attached, AttachmentLifecycle::Failed)
                | (AttachmentLifecycle::Detaching, AttachmentLifecycle::Absent)
                | (AttachmentLifecycle::Detaching, AttachmentLifecycle::Failed)
        )
}
