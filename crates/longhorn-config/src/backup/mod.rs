mod adapter;
mod archive;
pub(crate) mod capture;
mod catalog;
pub(crate) mod restore;
mod types;

pub use adapter::{
    BackupAdapter, BackupAdapterCapabilities, BackupAdapterCapture, BackupAdapterCaptureMode,
    BackupAdapterCaptureRequest, BackupAdapterConsistencyGroup, BackupAdapterDeclarationError,
    BackupAdapterError, BackupAdapterGroupedApplyKind, BackupAdapterGroupedApplyRequest,
    BackupAdapterGroupedRestore, BackupAdapterGroupedStageRequest,
    BackupAdapterGroupedVerifyRequest, BackupAdapterInspectRequest, BackupAdapterPayload,
    BackupAdapterPayloadRef, BackupAdapterRelativePath, BackupAdapterRestoreOutcome,
    BackupAdapterRestoreParticipation, BackupAdapterRestorePreview, BackupAdapterRestoreRequest,
    BackupAdapterRestoreStage, BackupAdapterStateEvidence,
};
pub use capture::BackupCaptureError;
pub use catalog::{
    BackupAdapterId, BackupCatalog, BackupCatalogError, BackupExclusionReason, BackupPolicy,
};
pub use restore::{
    MigrationRewriteError, MigrationRewriteOptions, MigrationRewriteReceipt, RestoreAction,
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
    RestoreStagingReceipt,
};
pub use types::{
    BackupAdapterCaptureReceipt, BackupApplication, BackupCaptureOptions, BackupCaptureReceipt,
    BackupConsistencyGroup, BackupConsistencyMode, BackupExclusion, BackupKind, BackupLimits,
    BackupLimitsError, BackupManifest, BackupManifestDomain, BackupMetadata, BackupMetadataError,
    BackupPayloadManifest, BackupPayloadPath, BackupPayloadPathError, BackupProducer, BackupScope,
    BackupScopeError, BackupSnapshot, BackupSnapshotPayload, BackupSourceIssue, BackupSourceState,
    Sha256Digest, Sha256DigestError,
};

pub use archive::{
    BackupArchiveError, BackupArchiveFileName, BackupArchiveFileNameError, BackupArchiveInspection,
    BackupArchiveLimits, BackupArchiveLimitsError, BackupAuthenticity, BackupDestinationKind,
    BackupExportTarget, BackupIntegrity, BackupOperationalCandidate, BackupOperationalListing,
    BackupOperationalRoot, BackupPublicationError, BackupPublicationOptions,
    BackupPublicationReceipt, BackupPublicationStage, BackupRetentionApplyError,
    BackupRetentionApplyReceipt, BackupRetentionDeletion, BackupRetentionDiagnostic,
    BackupRetentionDiagnosticKind, BackupRetentionPlan, BackupRetentionPlanError,
    BackupRetentionPolicy, BackupRetentionPolicyError, BackupRetentionReason, EncodedBackupArchive,
    ExportOverwrite, InspectedBackupPayload, MilestoneRetention, apply_backup_retention,
    encode_backup_archive, encode_backup_export_archive, export_backup, inspect_backup_archive,
    list_operational_backups, plan_backup_retention, publish_operational_backup,
};
pub(crate) use catalog::CatalogDecision;
