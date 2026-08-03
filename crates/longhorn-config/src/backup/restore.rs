mod adapter_restore;
mod execution;
mod grouped;
mod inspection;
mod journal;
mod live_io;
mod migration;
mod planning;
mod recovery;
mod staging;
mod transaction;
mod types;

pub use grouped::{
    RestoreAdapterGroupError, RestoreAdapterGroupExecutionOptions,
    RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupExecutionStage,
    RestoreAdapterGroupPlan, RestoreAdapterGroupPlanEntry, RestoreAdapterGroupPlanError,
    RestoreAdapterGroupRecoveryError, RestoreAdapterGroupRecoveryOutcome,
    RestoreAdapterGroupRecoveryReceipt,
};
pub use types::{
    MigrationRewriteError, MigrationRewriteOptions, MigrationRewriteReceipt, RestoreAction,
    RestoreAdapterError, RestoreAdapterReceipt, RestoreAdapterRequirement, RestoreChoiceError,
    RestoreChoices, RestoreConflictChoice, RestoreCurrentEvidence, RestoreDomainCompatibility,
    RestoreDomainInspection, RestoreExclusionInspection, RestoreExecutionError,
    RestoreExecutionOptions, RestoreExecutionReceipt, RestoreExecutionStage,
    RestoreFailureTerminal, RestoreIdentityInspection, RestoreIdentityStatus, RestoreInspection,
    RestoreInspectionReceipt, RestoreOperationState, RestorePlan, RestorePlanEntry,
    RestorePlanError, RestorePlanReceipt, RestorePrepareError, RestorePrepareOptions,
    RestoreRecoveryError, RestoreRecoveryOptions, RestoreRecoveryOutcome, RestoreRecoveryReceipt,
    RestoreSafetyBackupOptions, RestoreStaging, RestoreStagingReceipt,
};

pub(crate) use adapter_restore::execute as execute_adapter;
pub(crate) use execution::execute;
pub(crate) use grouped::recover as recover_grouped_adapters;
pub(crate) use grouped::{execute as execute_grouped_adapters, plan as plan_grouped_adapters};
pub(crate) use inspection::inspect;
pub(crate) use migration::rewrite as rewrite_migration;
pub(crate) use planning::plan;
pub(crate) use recovery::{recover, recover_guarded};
pub(crate) use staging::{
    PrepareSourceError, PreparedRestoreSource, prepare, prepare_typed_source,
};

pub(crate) fn operation_state(store: &crate::ConfigStore) -> RestoreOperationState {
    let authority = store.coordinator.authority_root();
    match (
        journal::operation_state(authority),
        grouped::operation_state(authority),
    ) {
        (RestoreOperationState::RecoveryRequired, _)
        | (_, RestoreOperationState::RecoveryRequired) => RestoreOperationState::RecoveryRequired,
        (RestoreOperationState::Active, _) | (_, RestoreOperationState::Active) => {
            RestoreOperationState::Active
        }
        (RestoreOperationState::Inactive, RestoreOperationState::Inactive) => {
            RestoreOperationState::Inactive
        }
    }
}

pub(crate) fn safety_pin(
    store: &crate::ConfigStore,
) -> Result<Option<crate::Sha256Digest>, RestoreRecoveryError> {
    let authority = store.coordinator.authority_root();
    journal::load(authority)
        .map(|journal| journal.map(|journal| journal.safety_sha256))
        .map_err(|error| RestoreRecoveryError {
            path: journal::journal_path(authority),
            domain: None,
            detail: error.to_string(),
        })
}
