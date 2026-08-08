//! Navigation plan execution.

use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
    MAXIMUM_HISTORY_NAVIGATION_STEPS,
};

use crate::{ForkBranchId, ForkHistory};

use super::{
    ForkNavigationError, ForkNavigationPlan, ForkNavigationReceipt, ForkNavigationTarget,
    ForkNavigationTransaction,
};

impl<P> ForkHistory<P> {
    /// Revalidates, atomically applies, then commits one route.
    pub fn execute_navigation<T>(
        &mut self,
        plan: ForkNavigationPlan<P>,
        transaction: &mut T,
    ) -> Result<ForkNavigationReceipt, ForkNavigationError<T::Error>>
    where
        T: ForkNavigationTransaction<P>,
    {
        self.validate_plan(&plan)?;
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkNavigationError::RevisionOverflow)?;
        if let Err(failure) = transaction.apply(&plan) {
            return Err(match failure {
                HistoryNavigationTransactionFailure::RolledBack { error } => {
                    ForkNavigationError::RolledBack { error }
                }
                HistoryNavigationTransactionFailure::RollbackFailed {
                    error,
                    rollback_error,
                } => ForkNavigationError::RollbackFailed {
                    error,
                    rollback_error,
                },
            });
        }

        let moved_entry_ids = plan
            .steps
            .iter()
            .map(|step| step.entry_id().clone())
            .collect();
        let target_lineage = self
            .lineage(plan.target_node_id.as_ref())
            .expect("validated plan target has a finite lineage");
        let shared = plan
            .lowest_common_ancestor
            .as_ref()
            .and_then(|ancestor| target_lineage.iter().position(|entry| entry == ancestor))
            .map_or(0, |index| index + 1);
        let mut parent = plan.lowest_common_ancestor.clone();
        for child in &target_lineage[shared..] {
            self.preferred_children
                .insert(parent.clone(), child.clone());
            parent = Some(child.clone());
        }

        self.current_node_id = plan.target_node_id.clone();
        self.current_branch_id = plan.target_branch_id.clone();
        self.revision = committed_revision;
        Ok(ForkNavigationReceipt {
            history_id: self.history_id.clone(),
            previous_revision: plan.history_revision,
            committed_revision,
            plan_id: plan.plan_id,
            source_node_id: plan.source_node_id,
            target_node_id: plan.target_node_id,
            target_branch_id: plan.target_branch_id,
            moved_entry_ids,
        })
    }

    pub(crate) fn lineage(
        &self,
        target: Option<&HistoryEntryId>,
    ) -> Result<Vec<HistoryEntryId>, HistoryEntryId> {
        let mut reverse = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cursor = target.cloned();
        while let Some(entry_id) = cursor {
            if !seen.insert(entry_id.clone()) {
                return Err(entry_id);
            }
            let node = self.nodes.get(&entry_id).ok_or_else(|| entry_id.clone())?;
            reverse.push(entry_id);
            cursor = node.parent_entry_id().cloned();
        }
        reverse.reverse();
        Ok(reverse)
    }

    pub(crate) fn branch_contains(
        &self,
        branch_id: &ForkBranchId,
        target: Option<&HistoryEntryId>,
    ) -> bool {
        let Some(branch) = self.branches.get(branch_id) else {
            return false;
        };
        let mut cursor = branch.head_entry_id().cloned();
        loop {
            if cursor.as_ref() == target {
                return true;
            }
            let Some(entry_id) = cursor else {
                return false;
            };
            cursor = self
                .nodes
                .get(&entry_id)
                .and_then(|node| node.parent_entry_id().cloned());
        }
    }

    pub(crate) fn resolve_target<E>(
        &self,
        target: &ForkNavigationTarget,
    ) -> Result<(ForkBranchId, Option<HistoryEntryId>), ForkNavigationError<E>> {
        match target {
            ForkNavigationTarget::Undo => {
                let current = self
                    .current_node_id
                    .as_ref()
                    .ok_or(ForkNavigationError::NothingToUndo)?;
                let parent = self
                    .nodes
                    .get(current)
                    .expect("current node is retained")
                    .parent_entry_id()
                    .cloned();
                Ok((self.current_branch_id.clone(), parent))
            }
            ForkNavigationTarget::Redo => {
                let child = self
                    .preferred_children
                    .get(&self.current_node_id)
                    .cloned()
                    .ok_or(ForkNavigationError::NothingToRedo)?;
                let branch_id = self
                    .preferred_branch_for(&child)
                    .ok_or_else(|| ForkNavigationError::UnreferencedTarget(child.clone()))?;
                Ok((branch_id, Some(child)))
            }
            ForkNavigationTarget::Checkout {
                branch_id,
                entry_id,
            } => {
                if !self.nodes.contains_key(entry_id) {
                    return Err(ForkNavigationError::UnknownEntry(entry_id.clone()));
                }
                if !self.branch_contains(branch_id, Some(entry_id)) {
                    return Err(ForkNavigationError::EntryOutsideBranch {
                        branch_id: branch_id.clone(),
                        entry_id: entry_id.clone(),
                    });
                }
                Ok((branch_id.clone(), Some(entry_id.clone())))
            }
        }
    }

    pub(crate) fn validate_plan<E>(&self, plan: &ForkNavigationPlan<P>) -> Result<(), ForkNavigationError<E>> {
        if plan.history_id != self.history_id {
            return Err(ForkNavigationError::WrongHistory {
                expected: self.history_id.clone(),
                actual: plan.history_id.clone(),
            });
        }
        if plan.history_revision != self.revision {
            return Err(ForkNavigationError::StaleRevision {
                expected: plan.history_revision,
                actual: self.revision,
            });
        }
        if plan.source_node_id != self.current_node_id
            || plan.source_branch_id != self.current_branch_id
            || !self.branch_contains(&plan.target_branch_id, plan.target_node_id.as_ref())
        {
            return Err(ForkNavigationError::InvalidPlan);
        }
        let route_ids: Vec<_> = plan
            .steps
            .iter()
            .map(|step| step.entry_id().clone())
            .collect();
        let expected_ids =
            self.route_ids(plan.source_node_id.as_ref(), plan.target_node_id.as_ref())?;
        if route_ids != expected_ids {
            return Err(ForkNavigationError::InvalidPlan);
        }
        Ok(())
    }

    pub(crate) fn route_ids<E>(
        &self,
        source: Option<&HistoryEntryId>,
        target: Option<&HistoryEntryId>,
    ) -> Result<Vec<HistoryEntryId>, ForkNavigationError<E>> {
        let source_lineage = self
            .lineage(source)
            .map_err(ForkNavigationError::UnknownEntry)?;
        let target_lineage = self
            .lineage(target)
            .map_err(ForkNavigationError::UnknownEntry)?;
        let shared = shared_depth(&source_lineage, &target_lineage);
        Ok(source_lineage[shared..]
            .iter()
            .rev()
            .chain(&target_lineage[shared..])
            .cloned()
            .collect())
    }

    pub(crate) fn preferred_branch_for(&self, child: &HistoryEntryId) -> Option<ForkBranchId> {
        if self.branch_contains(&self.current_branch_id, Some(child)) {
            return Some(self.current_branch_id.clone());
        }
        self.branches
            .values()
            .filter(|branch| self.branch_contains(branch.branch_id(), Some(child)))
            .map(|branch| branch.branch_id().clone())
            .min()
    }
}

pub(crate) fn shared_depth(left: &[HistoryEntryId], right: &[HistoryEntryId]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

