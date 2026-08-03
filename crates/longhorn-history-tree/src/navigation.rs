use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
    MAXIMUM_HISTORY_NAVIGATION_STEPS,
};

use crate::{ForkBranchId, ForkHistory};

/// Stable graph navigation intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkNavigationTarget {
    /// Move to the parent of the current node.
    Undo,
    /// Move to the deterministic preferred child.
    Redo,
    /// Move to one stable entry on one first-class branch.
    Checkout {
        /// Stable branch reference.
        branch_id: ForkBranchId,
        /// Stable target entry.
        entry_id: HistoryEntryId,
    },
}

/// Immutable mixed undo/redo route bound to exact graph authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkNavigationPlan<P> {
    history_id: HistoryId,
    history_revision: HistoryRevision,
    plan_id: HistoryPlanId,
    target: ForkNavigationTarget,
    source_branch_id: ForkBranchId,
    target_branch_id: ForkBranchId,
    source_node_id: Option<HistoryEntryId>,
    target_node_id: Option<HistoryEntryId>,
    lowest_common_ancestor: Option<HistoryEntryId>,
    steps: Vec<HistoryNavigationStep<P>>,
}

impl<P> ForkNavigationPlan<P> {
    /// Returns the bound graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the exact source revision.
    #[must_use]
    pub const fn history_revision(&self) -> HistoryRevision {
        self.history_revision
    }

    /// Returns the injected plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the requested navigation intent.
    #[must_use]
    pub const fn target(&self) -> &ForkNavigationTarget {
        &self.target
    }

    /// Returns the source branch.
    #[must_use]
    pub const fn source_branch_id(&self) -> &ForkBranchId {
        &self.source_branch_id
    }

    /// Returns the target branch.
    #[must_use]
    pub const fn target_branch_id(&self) -> &ForkBranchId {
        &self.target_branch_id
    }

    /// Returns the source node, or root.
    #[must_use]
    pub const fn source_node_id(&self) -> Option<&HistoryEntryId> {
        self.source_node_id.as_ref()
    }

    /// Returns the target node, or root.
    #[must_use]
    pub const fn target_node_id(&self) -> Option<&HistoryEntryId> {
        self.target_node_id.as_ref()
    }

    /// Returns the route's lowest common ancestor, or root.
    #[must_use]
    pub const fn lowest_common_ancestor(&self) -> Option<&HistoryEntryId> {
        self.lowest_common_ancestor.as_ref()
    }

    /// Returns the complete ordered product payload batch.
    #[must_use]
    pub fn steps(&self) -> &[HistoryNavigationStep<P>] {
        &self.steps
    }
}

/// Consumer-owned atomic product transaction for one complete graph route.
pub trait ForkNavigationTransaction<P> {
    /// Product apply or rollback failure.
    type Error;

    /// Applies the complete mixed route atomically.
    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<P>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>>;
}

/// Successful graph navigation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkNavigationReceipt {
    history_id: HistoryId,
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    plan_id: HistoryPlanId,
    source_node_id: Option<HistoryEntryId>,
    target_node_id: Option<HistoryEntryId>,
    target_branch_id: ForkBranchId,
    moved_entry_ids: Vec<HistoryEntryId>,
}

impl ForkNavigationReceipt {
    /// Returns the graph authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the source graph revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed graph revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the committed plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &HistoryPlanId {
        &self.plan_id
    }

    /// Returns the source node, or root.
    #[must_use]
    pub const fn source_node_id(&self) -> Option<&HistoryEntryId> {
        self.source_node_id.as_ref()
    }

    /// Returns the target node, or root.
    #[must_use]
    pub const fn target_node_id(&self) -> Option<&HistoryEntryId> {
        self.target_node_id.as_ref()
    }

    /// Returns the selected first-class branch.
    #[must_use]
    pub const fn target_branch_id(&self) -> &ForkBranchId {
        &self.target_branch_id
    }

    /// Returns entry identities in apply order.
    #[must_use]
    pub fn moved_entry_ids(&self) -> &[HistoryEntryId] {
        &self.moved_entry_ids
    }
}

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

    fn resolve_target<E>(
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

    fn validate_plan<E>(&self, plan: &ForkNavigationPlan<P>) -> Result<(), ForkNavigationError<E>> {
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

    fn route_ids<E>(
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

    fn preferred_branch_for(&self, child: &HistoryEntryId) -> Option<ForkBranchId> {
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

fn shared_depth(left: &[HistoryEntryId], right: &[HistoryEntryId]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

/// Rejected graph navigation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkNavigationError<E> {
    /// Plan belongs to another graph authority.
    WrongHistory {
        /// Expected graph identity.
        expected: HistoryId,
        /// Supplied graph identity.
        actual: HistoryId,
    },
    /// Request or plan revision is stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Current node is the root.
    NothingToUndo,
    /// Current node has no preferred child.
    NothingToRedo,
    /// Requested checkout is already current.
    AlreadyAtTarget,
    /// Entry identity does not exist.
    UnknownEntry(HistoryEntryId),
    /// Target entry is not on the selected branch.
    EntryOutsideBranch {
        /// Selected branch.
        branch_id: ForkBranchId,
        /// Requested target.
        entry_id: HistoryEntryId,
    },
    /// A preferred child has no first-class branch reference.
    UnreferencedTarget(HistoryEntryId),
    /// Planned route exceeded its hard bound.
    RouteTooLong {
        /// Maximum navigation steps.
        maximum: usize,
        /// Planned navigation steps.
        actual: usize,
    },
    /// Consumer inverse policy rejected a node.
    Policy {
        /// Rejected entry.
        entry_id: HistoryEntryId,
        /// Consumer policy failure.
        error: E,
    },
    /// Plan no longer matches graph state.
    InvalidPlan,
    /// Product apply failed and exact rollback was verified.
    RolledBack {
        /// Product apply failure.
        error: E,
    },
    /// Product apply and rollback both failed.
    RollbackFailed {
        /// Product apply failure.
        error: E,
        /// Rollback failure.
        rollback_error: E,
    },
    /// Revision could not advance.
    RevisionOverflow,
}

impl<E> fmt::Display for ForkNavigationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongHistory { .. } => "fork navigation plan belongs to another history",
            Self::StaleRevision { .. } => "fork navigation revision is stale",
            Self::NothingToUndo => "fork history has nothing to undo",
            Self::NothingToRedo => "fork history has nothing to redo",
            Self::AlreadyAtTarget => "fork history is already at the requested target",
            Self::UnknownEntry(_) => "fork navigation entry does not exist",
            Self::EntryOutsideBranch { .. } => "fork navigation entry is outside its branch",
            Self::UnreferencedTarget(_) => "fork preferred child has no branch reference",
            Self::RouteTooLong { .. } => "fork navigation route exceeds its hard limit",
            Self::Policy { .. } => "fork navigation inverse policy failed",
            Self::InvalidPlan => "fork navigation plan is invalid",
            Self::RolledBack { .. } => "fork navigation apply failed and rolled back",
            Self::RollbackFailed { .. } => "fork navigation apply and rollback failed",
            Self::RevisionOverflow => "fork history revision cannot advance",
        })
    }
}

impl<E: Error + fmt::Debug> Error for ForkNavigationError<E> {}
