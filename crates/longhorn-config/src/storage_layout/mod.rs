mod bootstrap;
mod identity;
mod resolver;
mod transition;
mod types;

pub use bootstrap::{
    StorageBootstrapOrigin, StorageBootstrapPathError, StorageBootstrapPaths,
    StorageBootstrapRecovery, StorageBootstrapRecoveryKind, StorageBootstrapSelection,
    StorageBootstrapState, StorageProfileLocator, StorageProfileSelection,
    StorageProfileSelectionError, inspect_storage_bootstrap, resolve_storage_bootstrap_paths,
};
pub use identity::{
    StorageIdentity, StorageIdentityError, StorageIdentityErrorKind, StorageIdentityField,
};
pub use resolver::resolve_storage_layout;
pub use transition::{
    LegacyStorageCandidate, LegacyStorageDiscovery, StorageFileEvidence, StorageTransitionAction,
    StorageTransitionAdapter, StorageTransitionCatalog, StorageTransitionCleanupPlan,
    StorageTransitionCleanupReceipt, StorageTransitionConflict, StorageTransitionConflictKind,
    StorageTransitionDomain, StorageTransitionError, StorageTransitionExclusion,
    StorageTransitionExecutionOptions, StorageTransitionGuard, StorageTransitionLimits,
    StorageTransitionOutcome, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionReceipt, StorageTransitionRecoveryReceipt,
    StorageTransitionRequest, StorageTransitionUnknownFile, apply_storage_transition_cleanup,
    discover_legacy_storage, execute_storage_transition, inspect_storage_transition,
    plan_storage_transition, recover_storage_transition,
};
pub use types::{
    PlatformDirectoryFact, PlatformDirectoryFacts, ResolvedStorageLayout, ResolvedStorageRoot,
    StorageLayoutDiagnostic, StorageLayoutError, StorageLayoutOverrides, StorageLayoutRequest,
    StorageLayoutWarning, StorageLeafProvenance, StorageProfile, StorageProfileIdError,
    StorageRootProvenance, TargetPlatform,
};
