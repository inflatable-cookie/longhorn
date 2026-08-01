use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryLabel, HistoryPolicy,
    LinearHistory,
};

/// Stable navigation intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationTarget {
    /// Move one entry toward the retained baseline.
    Undo,
    /// Move one entry toward the newest retained state.
    Redo,
    /// Make one stable entry the current applied entry.
    Checkout {
        /// Entry identity, never a presentation index.
        entry_id: HistoryEntryId,
    },
}

/// One injected, revision-bound navigation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationRequest {
    plan_id: HistoryPlanId,
    expected_revision: HistoryRevision,
    target: HistoryNavigationTarget,
}

impl HistoryNavigationRequest {
    /// Constructs a navigation request.
    #[must_use]
    pub const fn new(
        plan_id: HistoryPlanId,
        expected_revision: HistoryRevision,
        target: HistoryNavigationTarget,
    ) -> Self {
        Self {
            plan_id,
            expected_revision,
            target,
        }
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the exact source revision required for planning.
    #[must_use]
    pub const fn expected_revision(&self) -> HistoryRevision {
        self.expected_revision
    }

    /// Returns the requested stable target.
    #[must_use]
    pub const fn target(&self) -> &HistoryNavigationTarget {
        &self.target
    }
}

/// Direction of one planned linear navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryNavigationDirection {
    /// Move toward the retained baseline.
    Undo,
    /// Move toward the newest retained state.
    Redo,
    /// Remain at the same entry while committing an explicit checkout.
    Stationary,
}

/// One ordered typed payload application in a navigation batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationStep<P> {
    /// Apply the consumer-produced inverse of one current entry.
    Undo {
        /// Entry being unapplied.
        entry_id: HistoryEntryId,
        /// Typed inverse payload.
        payload: P,
    },
    /// Apply one retained forward payload.
    Redo {
        /// Entry being reapplied.
        entry_id: HistoryEntryId,
        /// Typed forward payload.
        payload: P,
    },
}

impl<P> HistoryNavigationStep<P> {
    /// Returns the entry affected by this step.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        match self {
            Self::Undo { entry_id, .. } | Self::Redo { entry_id, .. } => entry_id,
        }
    }

    /// Returns the typed consumer payload to apply.
    #[must_use]
    pub const fn payload(&self) -> &P {
        match self {
            Self::Undo { payload, .. } | Self::Redo { payload, .. } => payload,
        }
    }

    /// Returns this step's direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        match self {
            Self::Undo { .. } => HistoryNavigationDirection::Undo,
            Self::Redo { .. } => HistoryNavigationDirection::Redo,
        }
    }
}

/// Bounded authoritative position metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationPosition {
    applied_depth: usize,
    future_depth: usize,
    current_entry_id: Option<HistoryEntryId>,
    next_undo_label: Option<HistoryLabel>,
    next_redo_entry_id: Option<HistoryEntryId>,
    next_redo_label: Option<HistoryLabel>,
}

impl HistoryNavigationPosition {
    /// Returns the number of applied entries.
    #[must_use]
    pub const fn applied_depth(&self) -> usize {
        self.applied_depth
    }

    /// Returns the number of future entries.
    #[must_use]
    pub const fn future_depth(&self) -> usize {
        self.future_depth
    }

    /// Returns the current applied entry.
    #[must_use]
    pub const fn current_entry_id(&self) -> Option<&HistoryEntryId> {
        self.current_entry_id.as_ref()
    }

    /// Returns the next undo label.
    #[must_use]
    pub const fn next_undo_label(&self) -> Option<&HistoryLabel> {
        self.next_undo_label.as_ref()
    }

    /// Returns the next redo entry.
    #[must_use]
    pub const fn next_redo_entry_id(&self) -> Option<&HistoryEntryId> {
        self.next_redo_entry_id.as_ref()
    }

    /// Returns the next redo label.
    #[must_use]
    pub const fn next_redo_label(&self) -> Option<&HistoryLabel> {
        self.next_redo_label.as_ref()
    }
}

/// Immutable typed navigation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationPlan<P> {
    history_id: HistoryId,
    plan_id: HistoryPlanId,
    source_revision: HistoryRevision,
    target: HistoryNavigationTarget,
    direction: HistoryNavigationDirection,
    source_position: HistoryNavigationPosition,
    target_position: HistoryNavigationPosition,
    steps: Vec<HistoryNavigationStep<P>>,
}

impl<P> HistoryNavigationPlan<P> {
    /// Returns the owning history authority.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the exact source revision.
    #[must_use]
    pub const fn source_revision(&self) -> HistoryRevision {
        self.source_revision
    }

    /// Returns the stable requested target.
    #[must_use]
    pub const fn target(&self) -> &HistoryNavigationTarget {
        &self.target
    }

    /// Returns the planned direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        self.direction
    }

    /// Returns the exact source position.
    #[must_use]
    pub const fn source_position(&self) -> &HistoryNavigationPosition {
        &self.source_position
    }

    /// Returns the exact target position.
    #[must_use]
    pub const fn target_position(&self) -> &HistoryNavigationPosition {
        &self.target_position
    }

    /// Returns the complete ordered typed payload batch.
    #[must_use]
    pub fn steps(&self) -> &[HistoryNavigationStep<P>] {
        &self.steps
    }
}

impl<P: Clone> LinearHistory<P> {
    /// Plans undo, redo, or entry-id checkout without changing history.
    pub fn plan_navigation<T>(
        &self,
        request: HistoryNavigationRequest,
        policy: &T,
    ) -> Result<HistoryNavigationPlan<P>, HistoryNavigationPlanningError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        if request.expected_revision != self.state.revision {
            return Err(HistoryNavigationPlanningError::StaleRevision {
                expected: request.expected_revision,
                actual: self.state.revision,
            });
        }
        if self
            .recent_committed_plan_ids
            .iter()
            .any(|plan_id| plan_id == &request.plan_id)
        {
            return Err(HistoryNavigationPlanningError::DuplicatePlanId(
                request.plan_id,
            ));
        }
        if self.state.revision.checked_next().is_err() {
            return Err(HistoryNavigationPlanningError::RevisionOverflow);
        }

        let source_depth = self.state.applied.len();
        let total = source_depth + self.state.future.len();
        let target_depth = match &request.target {
            HistoryNavigationTarget::Undo => source_depth
                .checked_sub(1)
                .ok_or(HistoryNavigationPlanningError::NothingToUndo)?,
            HistoryNavigationTarget::Redo => {
                if self.state.future.is_empty() {
                    return Err(HistoryNavigationPlanningError::NothingToRedo);
                }
                source_depth + 1
            }
            HistoryNavigationTarget::Checkout { entry_id } => {
                if let Some(index) = self
                    .state
                    .applied
                    .iter()
                    .position(|entry| entry.entry_id() == entry_id)
                {
                    index + 1
                } else if let Some(index) = self
                    .state
                    .future
                    .iter()
                    .rev()
                    .position(|entry| entry.entry_id() == entry_id)
                {
                    source_depth + index + 1
                } else {
                    return Err(HistoryNavigationPlanningError::UnknownEntry(
                        entry_id.clone(),
                    ));
                }
            }
        };
        debug_assert!(target_depth <= total);

        let step_count = source_depth.abs_diff(target_depth);
        if step_count > self.navigation_limits.maximum_steps() {
            return Err(HistoryNavigationPlanningError::TooManySteps {
                maximum: self.navigation_limits.maximum_steps(),
                actual: step_count,
            });
        }

        let direction = direction(source_depth, target_depth);
        let mut steps = Vec::with_capacity(step_count);
        if target_depth < source_depth {
            for entry in self.state.applied[target_depth..source_depth].iter().rev() {
                let payload = policy.inverse(entry.payload()).map_err(|error| {
                    HistoryNavigationPlanningError::Policy {
                        entry_id: entry.entry_id().clone(),
                        error,
                    }
                })?;
                steps.push(HistoryNavigationStep::Undo {
                    entry_id: entry.entry_id().clone(),
                    payload,
                });
            }
        } else {
            for entry in self
                .state
                .future
                .iter()
                .rev()
                .take(target_depth - source_depth)
            {
                steps.push(HistoryNavigationStep::Redo {
                    entry_id: entry.entry_id().clone(),
                    payload: entry.payload().clone(),
                });
            }
        }

        Ok(HistoryNavigationPlan {
            history_id: self.state.history_id.clone(),
            plan_id: request.plan_id,
            source_revision: self.state.revision,
            target: request.target,
            direction,
            source_position: self.position_at(source_depth),
            target_position: self.position_at(target_depth),
            steps,
        })
    }
}

impl<P> LinearHistory<P> {
    /// Revalidates, applies through the consumer transaction, then commits.
    ///
    /// The exclusive history borrow prevents a history transition between
    /// product apply and structural commit. A rejected plan never reaches the
    /// consumer transaction.
    pub fn execute_navigation<T>(
        &mut self,
        plan: HistoryNavigationPlan<P>,
        transaction: &mut T,
    ) -> Result<HistoryNavigationReceipt, HistoryNavigationExecutionError<T::Error>>
    where
        T: HistoryNavigationTransaction<P>,
    {
        let committed_revision = self.validate_plan(&plan).map_err(|rejection| {
            HistoryNavigationExecutionError::Rejected {
                plan_id: plan.plan_id.clone(),
                rejection,
            }
        })?;
        if let Err(failure) = transaction.apply(&plan) {
            return Err(match failure {
                HistoryNavigationTransactionFailure::RolledBack { error } => {
                    HistoryNavigationExecutionError::RolledBack {
                        plan_id: plan.plan_id,
                        error,
                    }
                }
                HistoryNavigationTransactionFailure::RollbackFailed {
                    error,
                    rollback_error,
                } => HistoryNavigationExecutionError::RollbackFailed {
                    plan_id: plan.plan_id,
                    error,
                    rollback_error,
                },
            });
        }

        let moved_entry_ids: Vec<HistoryEntryId> = plan
            .steps
            .iter()
            .map(|step| step.entry_id().clone())
            .collect();
        let source_position = plan.source_position.clone();
        let target_depth = plan.target_position.applied_depth;
        let source_depth = plan.source_position.applied_depth;

        if target_depth < source_depth {
            for _ in target_depth..source_depth {
                let entry = self
                    .state
                    .applied
                    .pop()
                    .expect("validated undo plan has enough applied entries");
                self.state.future.push(entry);
            }
        } else {
            for _ in source_depth..target_depth {
                let entry = self
                    .state
                    .future
                    .pop()
                    .expect("validated redo plan has enough future entries");
                self.state.applied.push(entry);
            }
        }
        self.state.revision = committed_revision;
        self.remember_committed_plan(plan.plan_id.clone());
        self.close_transient_group(crate::HistoryGroupCloseReason::Navigation);
        let authoritative_position = self.position_at(target_depth);
        debug_assert_eq!(authoritative_position, plan.target_position);
        let transition = HistoryCommittedTransition::new(
            self.state.history_id.clone(),
            Some(plan.source_revision),
            committed_revision,
            HistoryCommittedTransitionKind::Navigation {
                plan_id: plan.plan_id.clone(),
                direction: plan.direction,
                moved_entry_ids: moved_entry_ids.clone(),
                source_position: source_position.clone(),
                authoritative_position: authoritative_position.clone(),
            },
        );

        Ok(HistoryNavigationReceipt {
            history_id: self.state.history_id.clone(),
            plan_id: plan.plan_id,
            previous_revision: plan.source_revision,
            committed_revision,
            direction: plan.direction,
            moved_entry_ids,
            source_position,
            authoritative_position,
            transition,
        })
    }

    fn validate_plan(
        &self,
        plan: &HistoryNavigationPlan<P>,
    ) -> Result<HistoryRevision, HistoryNavigationRejection> {
        if self
            .recent_committed_plan_ids
            .iter()
            .any(|plan_id| plan_id == &plan.plan_id)
        {
            return Err(HistoryNavigationRejection::DuplicatePlan);
        }
        if plan.history_id != self.state.history_id {
            return Err(HistoryNavigationRejection::ForeignHistory);
        }
        if plan.source_revision != self.state.revision {
            return Err(HistoryNavigationRejection::StaleRevision {
                expected: plan.source_revision,
                actual: self.state.revision,
            });
        }

        let source_depth = self.state.applied.len();
        let total = source_depth + self.state.future.len();
        if plan.source_position != self.position_at(source_depth) {
            return Err(HistoryNavigationRejection::SourcePositionChanged);
        }
        let target_depth = plan.target_position.applied_depth;
        if target_depth > total {
            return Err(HistoryNavigationRejection::InvalidPlan);
        }
        let step_count = source_depth.abs_diff(target_depth);
        if step_count > self.navigation_limits.maximum_steps() {
            return Err(HistoryNavigationRejection::TooManySteps {
                maximum: self.navigation_limits.maximum_steps(),
                actual: step_count,
            });
        }
        if plan.steps.len() != step_count
            || plan.direction != direction(source_depth, target_depth)
            || plan.target_position != self.position_at(target_depth)
            || !self.target_matches(plan, source_depth, target_depth)
            || !self.steps_match(plan, source_depth, target_depth)
        {
            return Err(HistoryNavigationRejection::InvalidPlan);
        }

        self.state
            .revision
            .checked_next()
            .map_err(|_| HistoryNavigationRejection::RevisionOverflow)
    }

    fn target_matches(
        &self,
        plan: &HistoryNavigationPlan<P>,
        source_depth: usize,
        target_depth: usize,
    ) -> bool {
        match &plan.target {
            HistoryNavigationTarget::Undo => source_depth.checked_sub(1) == Some(target_depth),
            HistoryNavigationTarget::Redo => target_depth == source_depth + 1,
            HistoryNavigationTarget::Checkout { entry_id } => {
                plan.target_position.current_entry_id.as_ref() == Some(entry_id)
            }
        }
    }

    fn steps_match(
        &self,
        plan: &HistoryNavigationPlan<P>,
        source_depth: usize,
        target_depth: usize,
    ) -> bool {
        if target_depth < source_depth {
            self.state.applied[target_depth..source_depth]
                .iter()
                .rev()
                .zip(&plan.steps)
                .all(|(entry, step)| {
                    step.direction() == HistoryNavigationDirection::Undo
                        && step.entry_id() == entry.entry_id()
                })
        } else {
            self.state
                .future
                .iter()
                .rev()
                .take(target_depth - source_depth)
                .zip(&plan.steps)
                .all(|(entry, step)| {
                    step.direction() == HistoryNavigationDirection::Redo
                        && step.entry_id() == entry.entry_id()
                })
        }
    }

    fn position_at(&self, applied_depth: usize) -> HistoryNavigationPosition {
        let total = self.state.applied.len() + self.state.future.len();
        let current = applied_depth
            .checked_sub(1)
            .and_then(|index| self.canonical_entry(index));
        let next_redo = self.canonical_entry(applied_depth);
        HistoryNavigationPosition {
            applied_depth,
            future_depth: total - applied_depth,
            current_entry_id: current.map(|entry| entry.entry_id().clone()),
            next_undo_label: current.map(|entry| entry.metadata().label().clone()),
            next_redo_entry_id: next_redo.map(|entry| entry.entry_id().clone()),
            next_redo_label: next_redo.map(|entry| entry.metadata().label().clone()),
        }
    }

    fn canonical_entry(&self, index: usize) -> Option<&crate::HistoryEntry<P>> {
        if index < self.state.applied.len() {
            self.state.applied.get(index)
        } else {
            self.state
                .future
                .iter()
                .rev()
                .nth(index - self.state.applied.len())
        }
    }

    fn remember_committed_plan(&mut self, plan_id: HistoryPlanId) {
        if self.recent_committed_plan_ids.len() == self.navigation_limits.maximum_recent_plans() {
            self.recent_committed_plan_ids.pop_front();
        }
        self.recent_committed_plan_ids.push_back(plan_id);
    }
}

const fn direction(source_depth: usize, target_depth: usize) -> HistoryNavigationDirection {
    if target_depth < source_depth {
        HistoryNavigationDirection::Undo
    } else if target_depth > source_depth {
        HistoryNavigationDirection::Redo
    } else {
        HistoryNavigationDirection::Stationary
    }
}

/// Consumer-owned atomic product transaction for one complete plan.
pub trait HistoryNavigationTransaction<P> {
    /// Product apply or rollback failure.
    type Error;

    /// Applies every step atomically or returns exact rollback evidence.
    fn apply(
        &mut self,
        plan: &HistoryNavigationPlan<P>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>>;
}

/// Consumer transaction failure evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationTransactionFailure<E> {
    /// Product apply failed and the exact source model was restored.
    RolledBack {
        /// Product apply failure.
        error: E,
    },
    /// Product apply failed and exact rollback also failed.
    RollbackFailed {
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
}

/// Successful committed linear navigation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationReceipt {
    history_id: HistoryId,
    plan_id: HistoryPlanId,
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    direction: HistoryNavigationDirection,
    moved_entry_ids: Vec<HistoryEntryId>,
    source_position: HistoryNavigationPosition,
    authoritative_position: HistoryNavigationPosition,
    transition: HistoryCommittedTransition,
}

impl HistoryNavigationReceipt {
    /// Returns the owning history authority.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the committed plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed successor revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the committed direction.
    #[must_use]
    pub const fn direction(&self) -> HistoryNavigationDirection {
        self.direction
    }

    /// Returns moved entry ids in product-apply order.
    #[must_use]
    pub fn moved_entry_ids(&self) -> &[HistoryEntryId] {
        &self.moved_entry_ids
    }

    /// Returns the admitted source position.
    #[must_use]
    pub const fn source_position(&self) -> &HistoryNavigationPosition {
        &self.source_position
    }

    /// Returns authoritative committed position metadata.
    #[must_use]
    pub const fn authoritative_position(&self) -> &HistoryNavigationPosition {
        &self.authoritative_position
    }

    /// Returns the committed structural transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}

/// Failed navigation planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationPlanningError<E> {
    /// The request did not target current history.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// A recently committed plan id was reused.
    DuplicatePlanId(HistoryPlanId),
    /// No applied entry can be undone.
    NothingToUndo,
    /// No future entry can be redone.
    NothingToRedo,
    /// Checkout named no retained entry.
    UnknownEntry(HistoryEntryId),
    /// The requested route exceeded the configured batch bound.
    TooManySteps {
        /// Configured maximum.
        maximum: usize,
        /// Required steps.
        actual: usize,
    },
    /// The history revision cannot advance.
    RevisionOverflow,
    /// Consumer inverse policy rejected one entry.
    Policy {
        /// Entry whose inverse failed.
        entry_id: HistoryEntryId,
        /// Consumer failure.
        error: E,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryNavigationPlanningError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "expected history revision {}; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicatePlanId(plan_id) => {
                write!(
                    formatter,
                    "history plan id {plan_id} was recently committed"
                )
            }
            Self::NothingToUndo => formatter.write_str("history has no applied entry to undo"),
            Self::NothingToRedo => formatter.write_str("history has no future entry to redo"),
            Self::UnknownEntry(entry_id) => {
                write!(formatter, "history entry {entry_id} is not retained")
            }
            Self::TooManySteps { maximum, actual } => write!(
                formatter,
                "history navigation requires {actual} steps; maximum is {maximum}"
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
            Self::Policy { entry_id, error } => {
                write!(formatter, "history inverse for {entry_id} failed: {error}")
            }
        }
    }
}

impl<E> Error for HistoryNavigationPlanningError<E> where E: Error + 'static {}

/// Structural rejection before consumer product apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationRejection {
    /// The plan identity was already committed recently.
    DuplicatePlan,
    /// The plan belongs to another history authority.
    ForeignHistory,
    /// The plan source revision is no longer current.
    StaleRevision {
        /// Planned source revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Current applied/future position differs from the plan source.
    SourcePositionChanged,
    /// Direction, target, steps, or entry identities are inconsistent.
    InvalidPlan,
    /// The route exceeds the current configured batch bound.
    TooManySteps {
        /// Current maximum.
        maximum: usize,
        /// Planned steps.
        actual: usize,
    },
    /// The history revision cannot advance.
    RevisionOverflow,
}

impl fmt::Display for HistoryNavigationRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePlan => formatter.write_str("history plan was already committed"),
            Self::ForeignHistory => formatter.write_str("history plan belongs to another history"),
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "history plan revision {} is stale; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::SourcePositionChanged => {
                formatter.write_str("history plan source position changed")
            }
            Self::InvalidPlan => formatter.write_str("history plan is structurally invalid"),
            Self::TooManySteps { maximum, actual } => write!(
                formatter,
                "history plan has {actual} steps; current maximum is {maximum}"
            ),
            Self::RevisionOverflow => formatter.write_str("history revision cannot advance"),
        }
    }
}

impl Error for HistoryNavigationRejection {}

/// Failed checked navigation execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationExecutionError<E> {
    /// Plan validation failed before product apply.
    Rejected {
        /// Rejected plan identity.
        plan_id: HistoryPlanId,
        /// Structural rejection.
        rejection: HistoryNavigationRejection,
    },
    /// Product apply failed and exact rollback succeeded.
    RolledBack {
        /// Plan identity.
        plan_id: HistoryPlanId,
        /// Product apply failure.
        error: E,
    },
    /// Product apply and exact rollback both failed.
    RollbackFailed {
        /// Plan identity.
        plan_id: HistoryPlanId,
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryNavigationExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { plan_id, rejection } => {
                write!(formatter, "history plan {plan_id} rejected: {rejection}")
            }
            Self::RolledBack { plan_id, error } => {
                write!(formatter, "history plan {plan_id} rolled back: {error}")
            }
            Self::RollbackFailed {
                plan_id,
                error,
                rollback_error,
            } => write!(
                formatter,
                "history plan {plan_id} failed: {error}; rollback failed: {rollback_error}"
            ),
        }
    }
}

impl<E> Error for HistoryNavigationExecutionError<E> where E: Error + 'static {}

#[cfg(test)]
mod tests {
    use longhorn_core::{HistoryId, HistoryPlanId, HistoryRevision};

    use crate::{HistoryLimits, HistoryNavigationTarget};

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Payload(u8);

    #[test]
    fn corrupted_private_plan_rejects_before_transaction() {
        let history = LinearHistory::<Payload>::new(
            HistoryId::new("history:test").unwrap(),
            HistoryLimits::default(),
        );
        let mut plan = HistoryNavigationPlan {
            history_id: history.history_id().clone(),
            plan_id: HistoryPlanId::new("plan:test").unwrap(),
            source_revision: HistoryRevision::INITIAL,
            target: HistoryNavigationTarget::Checkout {
                entry_id: HistoryEntryId::new("entry:missing").unwrap(),
            },
            direction: HistoryNavigationDirection::Stationary,
            source_position: history.position_at(0),
            target_position: history.position_at(0),
            steps: Vec::new(),
        };
        plan.target_position.applied_depth = 1;

        assert_eq!(
            history.validate_plan(&plan),
            Err(HistoryNavigationRejection::InvalidPlan)
        );
    }
}
