mod adapter;
mod execution;
mod inspection;
mod planning;
mod staging;

pub use execution::{
    MigrationRewriteError, MigrationRewriteOptions, MigrationRewriteReceipt, RestoreExecutionError,
    RestoreExecutionOptions, RestoreExecutionReceipt, RestoreExecutionStage,
    RestoreFailureTerminal, RestoreOperationState, RestoreRecoveryError, RestoreRecoveryOptions,
    RestoreRecoveryOutcome, RestoreRecoveryReceipt, RestoreSafetyBackupOptions,
};
pub use inspection::{
    RestoreDomainCompatibility, RestoreDomainInspection, RestoreExclusionInspection,
    RestoreIdentityInspection, RestoreIdentityStatus, RestoreInspection, RestoreInspectionReceipt,
};
pub use planning::{
    RestoreAction, RestoreChoiceError, RestoreChoices, RestoreConflictChoice,
    RestoreCurrentEvidence, RestorePlan, RestorePlanEntry, RestorePlanError, RestorePlanReceipt,
};
pub use staging::{
    RestorePrepareError, RestorePrepareOptions, RestoreStaging, RestoreStagingReceipt,
};

pub use adapter::{RestoreAdapterError, RestoreAdapterReceipt, RestoreAdapterRequirement};
pub(super) use inspection::{PreparedAdapterTarget, PreparedTarget};
pub(super) use planning::PlannedTarget;
pub(super) use staging::StagedDomain;
