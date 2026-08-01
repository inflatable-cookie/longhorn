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

/// Successful desired-state replacement evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
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

/// Result of applying one host-destruction event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HostDestroyOutcome {
    /// The current generation was invalidated and observation became absent.
    Invalidated,
    /// This exact generation was already invalidated.
    AlreadyInvalidated,
}

/// Exact evidence for host-destruction invalidation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct HostDestroyReceipt {
    previous_observed_revision: NativeContentRevision,
    current_observed_revision: NativeContentRevision,
    generation: AttachGeneration,
    outcome: HostDestroyOutcome,
}

impl HostDestroyReceipt {
    /// Returns the observed revision checked by the caller.
    #[must_use]
    pub const fn previous_observed_revision(self) -> NativeContentRevision {
        self.previous_observed_revision
    }
    /// Returns the observed revision after invalidation.
    #[must_use]
    pub const fn current_observed_revision(self) -> NativeContentRevision {
        self.current_observed_revision
    }
    /// Returns the invalidated attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
    /// Returns whether this call performed or confirmed invalidation.
    #[must_use]
    pub const fn outcome(self) -> HostDestroyOutcome {
        self.outcome
    }
}

/// Pure desired/observed authority for one native-content island.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeContentCoordinator {
    desired: DesiredState,
    observed: ObservedState,
    invalidated_generation: Option<AttachGeneration>,
}

impl NativeContentCoordinator {
    /// Creates one coordinator with an absent observation at desired generation.
    #[must_use]
    pub fn new(desired: DesiredState) -> Self {
        let observed = ObservedState::absent(desired.generation());
        Self {
            desired,
            observed,
            invalidated_generation: None,
        }
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
    /// Returns the generation invalidated by host destruction, when any.
    #[must_use]
    pub const fn invalidated_generation(&self) -> Option<AttachGeneration> {
        self.invalidated_generation
    }

    /// Replaces mutable desired state under expected-revision authority.
    pub fn update_desired(
        &mut self,
        expected_revision: NativeContentRevision,
        update: DesiredUpdate,
    ) -> Result<DesiredUpdateReceipt, CoordinationError> {
        require_revision(self.desired.revision(), expected_revision)?;
        self.desired
            .capabilities()
            .validate_input(update.input_routing())?;
        validate_desired_generation(&self.desired, &self.observed, &update)?;
        if update.generation() == self.desired.generation()
            && update.host_window_id() != self.desired.host_window_id()
        {
            return Err(CoordinationError::HostChangeRequiresGeneration);
        }

        let current_revision = self
            .desired
            .revision()
            .checked_next()
            .map_err(|_| CoordinationError::RevisionOverflow)?;
        let generation = update.generation();
        let advanced_generation = generation != self.desired.generation();
        self.desired.replace(current_revision, update);
        if advanced_generation {
            self.invalidated_generation = None;
        }

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
        if self.invalidated_generation == Some(update.generation()) {
            return Err(CoordinationError::InvalidatedGeneration(
                update.generation(),
            ));
        }
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
            .map_err(|_| CoordinationError::RevisionOverflow)?;
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

    /// Invalidates the current generation before later events from a destroyed host arrive.
    pub fn host_destroyed(
        &mut self,
        host_window_id: &WindowId,
        expected_observed_revision: NativeContentRevision,
    ) -> Result<HostDestroyReceipt, CoordinationError> {
        require_revision(self.observed.revision(), expected_observed_revision)?;
        if host_window_id != self.desired.host_window_id() {
            return Err(CoordinationError::HostBindingMismatch {
                current: self.desired.host_window_id().clone(),
                supplied: host_window_id.clone(),
            });
        }
        let generation = self.desired.generation();
        if self.invalidated_generation == Some(generation) {
            return Ok(HostDestroyReceipt {
                previous_observed_revision: expected_observed_revision,
                current_observed_revision: expected_observed_revision,
                generation,
                outcome: HostDestroyOutcome::AlreadyInvalidated,
            });
        }

        let current_observed_revision = self
            .observed
            .revision()
            .checked_next()
            .map_err(|_| CoordinationError::RevisionOverflow)?;
        self.observed.replace(
            current_observed_revision,
            ObservationUpdate::absent(generation),
        );
        self.invalidated_generation = Some(generation);
        Ok(HostDestroyReceipt {
            previous_observed_revision: expected_observed_revision,
            current_observed_revision,
            generation,
            outcome: HostDestroyOutcome::Invalidated,
        })
    }

    /// Plans current desired state against fresh observed evidence.
    pub fn plan(&self) -> Result<ApplyPlan, CoordinationError> {
        if self.invalidated_generation == Some(self.desired.generation())
            && self.desired.presence() == crate::DesiredPresence::Present
        {
            return Err(CoordinationError::InvalidatedGeneration(
                self.desired.generation(),
            ));
        }
        plan_transition(&self.desired, &self.observed)
    }

    /// Reconciles sparse adapter reports only while their complete plan cursor is current.
    pub fn receipt(
        &self,
        plan: &ApplyPlan,
        executions: impl IntoIterator<Item = StepExecution>,
    ) -> Result<ApplyReceipt, ReceiptError> {
        self.validate_plan_cursor(plan)?;
        ApplyReceipt::build(plan, executions)
    }

    /// Validates and records a content-size decision without mutating desired state.
    pub fn decide_content_size(
        &self,
        proposal: ContentSizeProposal,
        decision: ContentSizeDecision,
    ) -> Result<ContentSizeProposalReceipt, CoordinationError> {
        if self.invalidated_generation == Some(proposal.generation()) {
            return Err(CoordinationError::InvalidatedGeneration(
                proposal.generation(),
            ));
        }
        decide_content_size(&self.desired, proposal, decision)
    }

    /// Validates a mechanism proposal without recording a consumer decision.
    pub fn validate_content_size_proposal(
        &self,
        proposal: ContentSizeProposal,
    ) -> Result<(), CoordinationError> {
        if self.invalidated_generation == Some(proposal.generation()) {
            return Err(CoordinationError::InvalidatedGeneration(
                proposal.generation(),
            ));
        }
        validate_content_size_proposal(&self.desired, proposal)
    }

    fn validate_plan_cursor(&self, plan: &ApplyPlan) -> Result<(), ReceiptError> {
        if plan.island_id() != self.desired.island_id() {
            return Err(ReceiptError::IslandMismatch {
                current: self.desired.island_id().clone(),
                supplied: plan.island_id().clone(),
            });
        }
        if plan.desired_revision() != self.desired.revision() {
            return Err(ReceiptError::StaleDesiredPlan {
                current: self.desired.revision(),
                supplied: plan.desired_revision(),
            });
        }
        if plan.observed_revision() != self.observed.revision() {
            return Err(ReceiptError::StaleObservedPlan {
                current: self.observed.revision(),
                supplied: plan.observed_revision(),
            });
        }
        if plan.generation() != self.desired.generation()
            || self.invalidated_generation == Some(plan.generation())
        {
            return Err(ReceiptError::InvalidGeneration {
                current: self.desired.generation(),
                supplied: plan.generation(),
            });
        }
        Ok(())
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
