mod catalog;
mod cleanup;
mod execution;
mod inventory;
mod io;
mod journal;
mod legacy;
mod types;

pub use catalog::{StorageTransitionAdapter, StorageTransitionCatalog, StorageTransitionGuard};
pub use cleanup::apply_storage_transition_cleanup;
pub use execution::{execute_storage_transition, recover_storage_transition};
pub use inventory::{inspect_storage_transition, plan_storage_transition};
pub use legacy::discover_legacy_storage;
pub use types::{
    LegacyStorageCandidate, LegacyStorageDiscovery, StorageFileEvidence, StorageTransitionAction,
    StorageTransitionCleanupPlan, StorageTransitionCleanupReceipt, StorageTransitionConflict,
    StorageTransitionConflictKind, StorageTransitionDomain, StorageTransitionError,
    StorageTransitionExclusion, StorageTransitionExecutionOptions, StorageTransitionLimits,
    StorageTransitionOutcome, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionReceipt, StorageTransitionRecoveryReceipt,
    StorageTransitionRequest, StorageTransitionUnknownFile,
};

pub(crate) use catalog::TransitionDecision;
