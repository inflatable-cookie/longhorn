use longhorn_core::{NativeContentRevision, WindowId};

use crate::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, CoordinationError, DesiredState,
    DesiredUpdate, ObservationUpdate, ObservedState, ReceiptError, StepExecution,
};
use crate::{
    plan::plan_transition,
    proposal::{decide_content_size, validate_content_size_proposal},
};

use super::{
    DesiredUpdateReceipt, HostDestroyOutcome, HostDestroyReceipt, ObservationReceipt,
    compare_generation, legal_transition, require_revision, validate_desired_generation,
    validate_observation_capabilities,
};

/// Pure desired/observed authority for one native-content island.
/// Pure desired/observed authority for one native-content island.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeContentCoordinator {
    pub(crate) desired: DesiredState,
    pub(crate) observed: ObservedState,
    pub(crate) invalidated_generation: Option<AttachGeneration>,
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

    pub(crate) fn validate_plan_cursor(&self, plan: &ApplyPlan) -> Result<(), ReceiptError> {
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
