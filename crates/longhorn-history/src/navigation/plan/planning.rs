//! Navigation plan construction.

use crate::{
    HistoryNavigationPlanningError, HistoryNavigationRequest, HistoryNavigationStep,
    HistoryNavigationTarget, HistoryPolicy, LinearHistory,
};

use super::helpers::direction;
use super::HistoryNavigationPlan;

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
