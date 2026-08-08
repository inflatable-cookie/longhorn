//! Navigation plan construction.

use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
    MAXIMUM_HISTORY_NAVIGATION_STEPS,
};

use crate::{ForkBranchId, ForkHistory};

use super::{ForkNavigationError, ForkNavigationPlan, ForkNavigationTarget, ForkNavigationTransaction, shared_depth};

impl<P: Clone> ForkHistory<P> {
    /// Plans an undo, preferred redo, or branch checkout.
    pub fn plan_navigation<T>(
        &self,
        plan_id: HistoryPlanId,
        expected_revision: HistoryRevision,
        target: ForkNavigationTarget,
        policy: &T,
    ) -> Result<ForkNavigationPlan<P>, ForkNavigationError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        if expected_revision != self.revision {
            return Err(ForkNavigationError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if self.revision.checked_next().is_err() {
            return Err(ForkNavigationError::RevisionOverflow);
        }

        let (target_branch_id, target_node_id) = self.resolve_target(&target)?;
        if target_node_id == self.current_node_id {
            return Err(ForkNavigationError::AlreadyAtTarget);
        }
        let source_lineage = self
            .lineage(self.current_node_id.as_ref())
            .map_err(ForkNavigationError::UnknownEntry)?;
        let target_lineage = self
            .lineage(target_node_id.as_ref())
            .map_err(ForkNavigationError::UnknownEntry)?;
        let shared = shared_depth(&source_lineage, &target_lineage);
        let route_length = source_lineage.len() - shared + target_lineage.len() - shared;
        if route_length > MAXIMUM_HISTORY_NAVIGATION_STEPS {
            return Err(ForkNavigationError::RouteTooLong {
                maximum: MAXIMUM_HISTORY_NAVIGATION_STEPS,
                actual: route_length,
            });
        }
        let lowest_common_ancestor = shared
            .checked_sub(1)
            .and_then(|index| source_lineage.get(index))
            .cloned();

        let mut steps = Vec::with_capacity(route_length);
        for entry_id in source_lineage[shared..].iter().rev() {
            let node = self
                .nodes
                .get(entry_id)
                .expect("validated lineage contains retained nodes");
            let payload =
                policy
                    .inverse(node.payload())
                    .map_err(|error| ForkNavigationError::Policy {
                        entry_id: entry_id.clone(),
                        error,
                    })?;
            steps.push(HistoryNavigationStep::Undo {
                entry_id: entry_id.clone(),
                payload,
            });
        }
        for entry_id in &target_lineage[shared..] {
            let node = self
                .nodes
                .get(entry_id)
                .expect("validated lineage contains retained nodes");
            steps.push(HistoryNavigationStep::Redo {
                entry_id: entry_id.clone(),
                payload: node.payload().clone(),
            });
        }

        Ok(ForkNavigationPlan {
            history_id: self.history_id.clone(),
            history_revision: self.revision,
            plan_id,
            target,
            source_branch_id: self.current_branch_id.clone(),
            target_branch_id,
            source_node_id: self.current_node_id.clone(),
            target_node_id,
            lowest_common_ancestor,
            steps,
        })
    }
}
