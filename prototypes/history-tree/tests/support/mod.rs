use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryPlanId};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryEntryMetadata, HistoryLabel,
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
};
use longhorn_history_tree_prototype::{
    ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkHistory, ForkNavigationPlan,
    ForkNavigationTransaction, ForkRecord,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Delta(pub(crate) i64);

pub(crate) struct DeltaPolicy;

impl HistoryPolicy<Delta> for DeltaPolicy {
    type Error = &'static str;

    fn inverse(&self, payload: &Delta) -> Result<Delta, Self::Error> {
        Ok(Delta(-payload.0))
    }

    fn is_noop(&self, payload: &Delta) -> bool {
        payload.0 == 0
    }

    fn encoded_weight(&self, _payload: &Delta) -> Result<u64, Self::Error> {
        Ok(8)
    }

    fn coalesce(
        &self,
        _previous: &Delta,
        _incoming: &Delta,
        _context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<Delta>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum TransactionMode {
    Commit,
    RollBack,
    RollbackFails,
}

pub(crate) struct ModelTransaction<'model> {
    pub(crate) model: &'model mut i64,
    pub(crate) mode: TransactionMode,
    pub(crate) calls: usize,
}

impl ForkNavigationTransaction<Delta> for ModelTransaction<'_> {
    type Error = &'static str;

    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<Delta>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.calls += 1;
        let source = *self.model;
        for step in plan.steps() {
            let delta = match step {
                HistoryNavigationStep::Undo { payload, .. }
                | HistoryNavigationStep::Redo { payload, .. } => payload.0,
            };
            *self.model += delta;
            if self.mode != TransactionMode::Commit {
                break;
            }
        }
        match self.mode {
            TransactionMode::Commit => Ok(()),
            TransactionMode::RollBack => {
                *self.model = source;
                Err(HistoryNavigationTransactionFailure::RolledBack {
                    error: "apply failed",
                })
            }
            TransactionMode::RollbackFails => {
                Err(HistoryNavigationTransactionFailure::RollbackFailed {
                    error: "apply failed",
                    rollback_error: "rollback failed",
                })
            }
        }
    }
}

pub(crate) fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("fixture branch id")
}

pub(crate) fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("fixture entry id")
}

pub(crate) fn plan_id(value: &str) -> HistoryPlanId {
    HistoryPlanId::new(value).expect("fixture plan id")
}

pub(crate) fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("fixture label"),
        Some(HistoryKindId::new("fixture:delta").expect("fixture kind")),
        None,
    )
}

pub(crate) fn branch_metadata(name: &str, pinned: bool) -> ForkBranchMetadata {
    ForkBranchMetadata::new(Some(name.to_owned()), None, pinned).expect("fixture branch metadata")
}

pub(crate) fn history() -> ForkHistory<Delta> {
    ForkHistory::new(
        HistoryId::new("history:fork-proof").expect("fixture history id"),
        branch_id("branch:main"),
        branch_metadata("Main", true),
    )
}

pub(crate) fn record(
    history: &mut ForkHistory<Delta>,
    model: &mut i64,
    id: &str,
    delta: i64,
    divergent_branch: Option<ForkBranchSeed>,
) {
    *model += delta;
    history
        .record_applied(ForkRecord::new(
            history.revision(),
            entry_id(id),
            metadata(id),
            8,
            Delta(delta),
            divergent_branch,
        ))
        .expect("fixture record");
}
