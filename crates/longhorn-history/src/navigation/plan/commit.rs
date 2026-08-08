//! Navigation plan apply and structural commit.

use longhorn_core::HistoryEntryId;

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryNavigationExecutionError,
    HistoryNavigationReceipt, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
    LinearHistory,
};

use super::HistoryNavigationPlan;

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
}
