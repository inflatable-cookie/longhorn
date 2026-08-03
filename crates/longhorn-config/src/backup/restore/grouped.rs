mod execution;
mod journal;
mod planning;
mod recovery;
mod types;

pub use types::{
    RestoreAdapterGroupError, RestoreAdapterGroupExecutionOptions,
    RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupExecutionStage,
    RestoreAdapterGroupPlan, RestoreAdapterGroupPlanEntry, RestoreAdapterGroupPlanError,
    RestoreAdapterGroupRecoveryError, RestoreAdapterGroupRecoveryOutcome,
    RestoreAdapterGroupRecoveryReceipt,
};

pub(crate) use execution::execute;
pub(crate) use planning::plan;
pub(crate) use recovery::recover;

pub(crate) fn operation_state(authority_root: &std::path::Path) -> super::RestoreOperationState {
    journal::operation_state(authority_root)
}

pub(crate) fn blocks_ordinary_recovery(authority_root: &std::path::Path) -> bool {
    journal::exists(authority_root)
}

pub(crate) fn journal_path(authority_root: &std::path::Path) -> std::path::PathBuf {
    journal::journal_path(authority_root)
}
