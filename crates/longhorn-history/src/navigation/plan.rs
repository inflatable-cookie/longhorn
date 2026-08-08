//! Navigation plan construction and commit helpers.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryPolicy, LinearHistory,
};

use super::{
    HistoryNavigationDirection, HistoryNavigationExecutionError, HistoryNavigationPlanningError,
    HistoryNavigationPosition, HistoryNavigationReceipt, HistoryNavigationRejection,
    HistoryNavigationRequest, HistoryNavigationStep, HistoryNavigationTarget,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
};

/// Immutable typed navigation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryNavigationPlan<P> {
    pub(crate) history_id: HistoryId,
    pub(crate) plan_id: HistoryPlanId,
    pub(crate) source_revision: HistoryRevision,
    pub(crate) target: HistoryNavigationTarget,
    pub(crate) direction: HistoryNavigationDirection,
    pub(crate) source_position: HistoryNavigationPosition,
    pub(crate) target_position: HistoryNavigationPosition,
    pub(crate) steps: Vec<HistoryNavigationStep<P>>,
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

    pub(crate) fn validate_plan(
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

    pub(crate) fn position_at(&self, applied_depth: usize) -> HistoryNavigationPosition {
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
