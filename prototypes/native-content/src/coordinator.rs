use serde::{Deserialize, Serialize};

use crate::{
    ApplyPlan, AttachGeneration, AttachmentLifecycle, CoordinationError, DesiredState,
    DesiredUpdate, EffectiveFocus, EffectiveVisibility, NativeContentMechanism,
    NativeContentRevision, ObservationUpdate, ObservedGeometry, ObservedReadiness, ObservedState,
    plan_transition,
};

/// Successful desired-state replacement evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DesiredUpdateReceipt {
    previous_revision: NativeContentRevision,
    current_revision: NativeContentRevision,
    generation: AttachGeneration,
}

impl DesiredUpdateReceipt {
    /// Returns the revision checked by the caller.
    #[must_use]
    pub const fn previous_revision(self) -> NativeContentRevision {
        self.previous_revision
    }

    /// Returns the committed desired revision.
    #[must_use]
    pub const fn current_revision(self) -> NativeContentRevision {
        self.current_revision
    }

    /// Returns the current desired attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
}

/// Successful fresh-observation admission evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservationReceipt {
    previous_revision: NativeContentRevision,
    current_revision: NativeContentRevision,
    generation: AttachGeneration,
    lifecycle: AttachmentLifecycle,
}

impl ObservationReceipt {
    /// Returns the previously observed revision.
    #[must_use]
    pub const fn previous_revision(self) -> NativeContentRevision {
        self.previous_revision
    }

    /// Returns the committed observed revision.
    #[must_use]
    pub const fn current_revision(self) -> NativeContentRevision {
        self.current_revision
    }

    /// Returns admitted attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }

    /// Returns admitted lifecycle.
    #[must_use]
    pub const fn lifecycle(self) -> AttachmentLifecycle {
        self.lifecycle
    }
}

/// Pure desired/observed authority for one native-content island.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeContentCoordinator {
    desired: DesiredState,
    observed: ObservedState,
}

impl NativeContentCoordinator {
    /// Creates one coordinator with an absent observation at desired generation.
    #[must_use]
    pub fn new(desired: DesiredState) -> Self {
        let observed = ObservedState::absent(desired.generation());
        Self { desired, observed }
    }

    /// Returns current desired state.
    #[must_use]
    pub const fn desired(&self) -> &DesiredState {
        &self.desired
    }

    /// Returns current observed state.
    #[must_use]
    pub const fn observed(&self) -> &ObservedState {
        &self.observed
    }

    /// Replaces the mutable desired state under expected-revision authority.
    pub fn update_desired(
        &mut self,
        expected_revision: NativeContentRevision,
        update: DesiredUpdate,
    ) -> Result<DesiredUpdateReceipt, CoordinationError> {
        require_revision(self.desired.revision(), expected_revision)?;
        validate_desired_generation(&self.desired, &self.observed, &update)?;

        let current_revision = self
            .desired
            .revision()
            .checked_next()
            .map_err(|_| CoordinationError::CounterOverflow)?;
        let generation = update.generation();
        self.desired.replace(current_revision, update);

        Ok(DesiredUpdateReceipt {
            previous_revision: expected_revision,
            current_revision,
            generation,
        })
    }

    /// Admits one complete fresh adapter observation under revision and generation checks.
    pub fn admit_observation(
        &mut self,
        expected_revision: NativeContentRevision,
        update: ObservationUpdate,
    ) -> Result<ObservationReceipt, CoordinationError> {
        require_revision(self.observed.revision(), expected_revision)?;
        compare_generation(self.desired.generation(), update.generation())?;
        validate_observation_capabilities(&self.desired, &update)?;

        if self.observed.generation() == update.generation() {
            if !legal_transition(self.observed.lifecycle(), update.lifecycle()) {
                return Err(CoordinationError::IllegalLifecycleTransition {
                    current: self.observed.lifecycle(),
                    proposed: update.lifecycle(),
                });
            }
        } else {
            if !matches!(
                self.observed.lifecycle(),
                AttachmentLifecycle::Absent | AttachmentLifecycle::Failed
            ) {
                return Err(CoordinationError::GenerationStillAttached(
                    self.observed.lifecycle(),
                ));
            }
            if !matches!(
                update.lifecycle(),
                AttachmentLifecycle::Absent
                    | AttachmentLifecycle::Attaching
                    | AttachmentLifecycle::Attached
                    | AttachmentLifecycle::Failed
            ) {
                return Err(CoordinationError::IllegalLifecycleTransition {
                    current: AttachmentLifecycle::Absent,
                    proposed: update.lifecycle(),
                });
            }
        }

        let current_revision = self
            .observed
            .revision()
            .checked_next()
            .map_err(|_| CoordinationError::CounterOverflow)?;
        let generation = update.generation();
        let lifecycle = update.lifecycle();
        self.observed.replace(current_revision, update);

        Ok(ObservationReceipt {
            previous_revision: expected_revision,
            current_revision,
            generation,
            lifecycle,
        })
    }

    /// Plans the current desired state against fresh observed evidence.
    pub fn plan(&self) -> Result<ApplyPlan, CoordinationError> {
        plan_transition(&self.desired, &self.observed)
    }
}

fn require_revision(
    current: NativeContentRevision,
    supplied: NativeContentRevision,
) -> Result<(), CoordinationError> {
    if current == supplied {
        Ok(())
    } else {
        Err(CoordinationError::StaleRevision { current, supplied })
    }
}

fn validate_desired_generation(
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
        .map_err(|_| CoordinationError::CounterOverflow)?;
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

fn compare_generation(
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

fn validate_observation_capabilities(
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

fn legal_transition(current: AttachmentLifecycle, proposed: AttachmentLifecycle) -> bool {
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
