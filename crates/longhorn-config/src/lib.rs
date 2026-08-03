//! Versioned, root-confined configuration domains, atomic local mutation, and
//! bounded backup snapshots.
//!
//! This crate performs read, validation, in-memory migration, and coordinated
//! atomic patch mutation with optional bounded debounce and explicit flush. It
//! also captures immutable registry-driven backup snapshots and publishes
//! bounded, verified plaintext backup archives. Restore adds confirmation-bound
//! private staging, durable journaled publication, exact rollback, crash
//! recovery, coordinated load-sets, safety-backed migration rewrite, and
//! failure-atomic grouped custom-adapter restore with offline boot recovery.

mod backup;
mod coordination;
mod debounce;
mod domain;
mod location;
mod operations;
mod registry;
mod storage_layout;
mod store;

pub use backup::{
    BackupAdapter, BackupAdapterCapabilities, BackupAdapterCapture, BackupAdapterCaptureMode,
    BackupAdapterCaptureReceipt, BackupAdapterCaptureRequest, BackupAdapterConsistencyGroup,
    BackupAdapterDeclarationError, BackupAdapterError, BackupAdapterGroupedApplyKind,
    BackupAdapterGroupedApplyRequest, BackupAdapterGroupedRestore,
    BackupAdapterGroupedStageRequest, BackupAdapterGroupedVerifyRequest, BackupAdapterId,
    BackupAdapterInspectRequest, BackupAdapterPayload, BackupAdapterPayloadRef,
    BackupAdapterRelativePath, BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation,
    BackupAdapterRestorePreview, BackupAdapterRestoreRequest, BackupAdapterRestoreStage,
    BackupAdapterStateEvidence, BackupApplication, BackupArchiveError, BackupArchiveFileName,
    BackupArchiveFileNameError, BackupArchiveInspection, BackupArchiveLimits,
    BackupArchiveLimitsError, BackupAuthenticity, BackupCaptureError, BackupCaptureOptions,
    BackupCaptureReceipt, BackupCatalog, BackupCatalogError, BackupConsistencyGroup,
    BackupConsistencyMode, BackupDestinationKind, BackupExclusion, BackupExclusionReason,
    BackupExportTarget, BackupIntegrity, BackupKind, BackupLimits, BackupLimitsError,
    BackupManifest, BackupManifestDomain, BackupMetadata, BackupMetadataError,
    BackupOperationalCandidate, BackupOperationalListing, BackupOperationalRoot,
    BackupPayloadManifest, BackupPayloadPath, BackupPayloadPathError, BackupPolicy, BackupProducer,
    BackupPublicationError, BackupPublicationOptions, BackupPublicationReceipt,
    BackupPublicationStage, BackupRetentionApplyError, BackupRetentionApplyReceipt,
    BackupRetentionDeletion, BackupRetentionDiagnostic, BackupRetentionDiagnosticKind,
    BackupRetentionPlan, BackupRetentionPlanError, BackupRetentionPolicy,
    BackupRetentionPolicyError, BackupRetentionReason, BackupScope, BackupScopeError,
    BackupSnapshot, BackupSnapshotPayload, BackupSourceIssue, BackupSourceState,
    EncodedBackupArchive, ExportOverwrite, InspectedBackupPayload, MigrationRewriteError,
    MigrationRewriteOptions, MigrationRewriteReceipt, MilestoneRetention, RestoreAction,
    RestoreAdapterError, RestoreAdapterGroupError, RestoreAdapterGroupExecutionOptions,
    RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupExecutionStage,
    RestoreAdapterGroupPlan, RestoreAdapterGroupPlanEntry, RestoreAdapterGroupPlanError,
    RestoreAdapterGroupReceiptEntry, RestoreAdapterGroupRecoveryError,
    RestoreAdapterGroupRecoveryOutcome, RestoreAdapterGroupRecoveryReceipt, RestoreAdapterReceipt,
    RestoreAdapterRequirement, RestoreChoiceError, RestoreChoices, RestoreConflictChoice,
    RestoreCurrentEvidence, RestoreDomainCompatibility, RestoreDomainInspection,
    RestoreExclusionInspection, RestoreExecutionError, RestoreExecutionOptions,
    RestoreExecutionReceipt, RestoreExecutionStage, RestoreFailureTerminal,
    RestoreIdentityInspection, RestoreIdentityStatus, RestoreInspection, RestoreInspectionReceipt,
    RestoreOperationState, RestorePlan, RestorePlanEntry, RestorePlanError, RestorePlanReceipt,
    RestorePrepareError, RestorePrepareOptions, RestoreRecoveryError, RestoreRecoveryOptions,
    RestoreRecoveryOutcome, RestoreRecoveryReceipt, RestoreSafetyBackupOptions, RestoreStaging,
    RestoreStagingReceipt, Sha256Digest, Sha256DigestError, apply_backup_retention,
    encode_backup_archive, encode_backup_export_archive, export_backup, inspect_backup_archive,
    list_operational_backups, plan_backup_retention, publish_operational_backup,
};
pub use coordination::{
    CoordinationAuthority, CoordinationAuthorityError, CoordinationFailure, CoordinationFailureKind,
};
pub use debounce::{
    DebounceClock, DebounceFlushSet, DebouncePolicy, DebouncePolicyError, DebounceSnapshot,
    DebounceStrategy, DebounceTerminal, DebouncedMutation, FlushOutcome, FlushSetError,
    PendingSnapshot, RetryDisposition, StageDisposition, StageError, StageReceipt, SystemClock,
};
pub use domain::{
    ConfigDomain, DomainDescriptor, DomainDescriptorError, DomainFilePath, DomainFilePathError,
    DomainIssue, MigrationStep, StorageClass,
};
pub use location::{
    AccessMode, DomainLocation, ResolvedFile, RootKind, StorageRootError, StorageRoots,
};
pub use operations::{
    BackupArchiveProjection, BackupCaptureReceiptProjection, BackupCreateCommand,
    BackupCreateOutcome, BackupEncryptionState, BackupExportCommand, BackupExportOutcome,
    BackupInventoryEntry, BackupInventoryEntryState, BackupInventoryProjection,
    BackupOperationsProjection, BackupPendingState, BackupPublicationReceiptProjection,
    BackupRetentionApplyCommand, BackupRetentionApplyOutcome, BackupRetentionProjection,
    BackupRetentionReasonProjection, CONFIG_OPERATIONS_PROTOCOL_VERSION, ConfigGeneration,
    ConfigOperationCapability, ConfigOperationProjectionError, ConfigOperationRejection,
    ConfigOperationRejectionCode, ConfigOperationsSnapshot, ConfigProtocolVersion,
    ConfigSnapshotCommand, PendingBackupPolicy, RestoreAdapterExecuteCommand,
    RestoreAdapterExecuteOutcome, RestoreAdapterParticipationProjection,
    RestoreAdapterReceiptProjection, RestoreAdapterRequirementProjection, RestoreArchiveSelection,
    RestoreAuthenticityProjection, RestoreConflictChoiceProjection,
    RestoreConsistencyGroupProjection, RestoreCurrentEvidenceProjection, RestoreDomainChoice,
    RestoreDomainCompatibilityProjection, RestoreDomainInspectionProjection,
    RestoreExclusionProjection, RestoreExecuteCommand, RestoreExecuteOutcome,
    RestoreExecutionFailureProjection, RestoreExecutionReceiptProjection,
    RestoreIdentityProjection, RestoreIdentityStatusProjection, RestoreInspectCommand,
    RestoreInspectOutcome, RestoreInspectionProjection, RestoreInspectionReceiptProjection,
    RestoreIntegrityProjection, RestoreOperationStateProjection, RestoreOperationsProjection,
    RestorePlanCommand, RestorePlanEntryProjection, RestorePlanOutcome, RestorePlanProjection,
    RestorePlanReceiptProjection, RestoreRecoveryCommand, RestoreRecoveryOutcomeProjection,
    RestoreRecoveryReceiptProjection, RestoreStagingReceiptProjection, StorageBootstrapProjection,
    StorageCleanupCommand, StorageCleanupOutcome, StorageCleanupReceiptProjection,
    StorageLayoutProjection, StorageLeafProvenanceProjection, StorageOperationsProjection,
    StorageProfileId, StorageRecoveryCommand, StorageRecoveryOutcome,
    StorageRecoveryReceiptProjection, StorageRootProjection, StorageTransitionConflictProjection,
    StorageTransitionDomainProjection, StorageTransitionExecuteCommand,
    StorageTransitionExecuteOutcome, StorageTransitionInspectCommand,
    StorageTransitionInspectOutcome, StorageTransitionPreviewProjection,
    StorageTransitionReceiptProjection,
};
pub use registry::RegistrationError;
pub use storage_layout::{
    LegacyStorageCandidate, LegacyStorageDiscovery, PlatformDirectoryFact, PlatformDirectoryFacts,
    ResolvedStorageLayout, ResolvedStorageRoot, StorageBootstrapOrigin, StorageBootstrapPathError,
    StorageBootstrapPaths, StorageBootstrapRecovery, StorageBootstrapRecoveryKind,
    StorageBootstrapSelection, StorageBootstrapState, StorageFileEvidence, StorageIdentity,
    StorageIdentityError, StorageIdentityErrorKind, StorageIdentityField, StorageLayoutDiagnostic,
    StorageLayoutError, StorageLayoutOverrides, StorageLayoutRequest, StorageLayoutWarning,
    StorageLeafProvenance, StorageProfile, StorageProfileIdError, StorageProfileLocator,
    StorageProfileSelection, StorageProfileSelectionError, StorageRootProvenance,
    StorageTransitionAction, StorageTransitionAdapter, StorageTransitionCatalog,
    StorageTransitionCleanupPlan, StorageTransitionCleanupReceipt, StorageTransitionConflict,
    StorageTransitionConflictKind, StorageTransitionDomain, StorageTransitionError,
    StorageTransitionExclusion, StorageTransitionExecutionOptions, StorageTransitionGuard,
    StorageTransitionLimits, StorageTransitionOutcome, StorageTransitionPlan,
    StorageTransitionPlanError, StorageTransitionPreview, StorageTransitionReceipt,
    StorageTransitionRecoveryReceipt, StorageTransitionRequest, StorageTransitionUnknownFile,
    TargetPlatform, apply_storage_transition_cleanup, discover_legacy_storage,
    execute_storage_transition, inspect_storage_bootstrap, inspect_storage_transition,
    plan_storage_transition, recover_storage_transition, resolve_storage_bootstrap_paths,
    resolve_storage_layout,
};
pub use store::{
    CheckedMutationContext, CheckedMutationError, CheckedMutationOutcome, ConfigStore,
    CoordinatedLoadError, CoordinatedLoadSet, Durability, DurabilityRequirement, LoadDiagnostic,
    LoadDiagnosticCode, LoadOutcome, LoadedConfig, LoadedOrigin, MutationError, MutationOptions,
    MutationReceipt, MutationRefusal, PublicationFailure, PublicationStage, RecoveryKind,
    RecoveryState, SourceDocument, StoreError, UnavailableState,
};
