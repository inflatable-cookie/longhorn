//! Plan validation and position helpers.

use longhorn_core::{HistoryPlanId, HistoryRevision};

use crate::{
    HistoryNavigationDirection, HistoryNavigationPosition, HistoryNavigationRejection,
    HistoryNavigationTarget, LinearHistory,
};

use super::HistoryNavigationPlan;

impl<P> LinearHistory<P> {
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
            // Depth zero and no current entry are the same statement made from
            // the two sides of the plan, so both are checked: a plan that
            // reached depth zero while still naming an entry is malformed.
            HistoryNavigationTarget::CheckoutRoot => {
                target_depth == 0 && plan.target_position.current_entry_id.is_none()
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

    pub(crate) fn remember_committed_plan(&mut self, plan_id: HistoryPlanId) {
        if self.recent_committed_plan_ids.len() == self.navigation_limits.maximum_recent_plans() {
            self.recent_committed_plan_ids.pop_front();
        }
        self.recent_committed_plan_ids.push_back(plan_id);
    }
}

pub(crate) const fn direction(
    source_depth: usize,
    target_depth: usize,
) -> HistoryNavigationDirection {
    if target_depth < source_depth {
        HistoryNavigationDirection::Undo
    } else if target_depth > source_depth {
        HistoryNavigationDirection::Redo
    } else {
        HistoryNavigationDirection::Stationary
    }
}
