mod bounded_cursor;
mod central_directory;
mod codec;
mod encoding;
mod error;
mod manifest;
mod publication;
mod publication_types;
mod retention;
mod types;

pub use codec::inspect_backup_archive;
pub use encoding::encode_backup_archive;
pub use error::BackupArchiveError;
pub use publication::{export_backup, publish_operational_backup};
pub use publication_types::{
    BackupArchiveFileName, BackupArchiveFileNameError, BackupDestinationKind, BackupExportTarget,
    BackupOperationalRoot, BackupPublicationError, BackupPublicationOptions,
    BackupPublicationReceipt, BackupPublicationStage, ExportOverwrite,
};
pub use retention::{
    BackupOperationalCandidate, BackupOperationalListing, BackupRetentionApplyError,
    BackupRetentionApplyReceipt, BackupRetentionDeletion, BackupRetentionDiagnostic,
    BackupRetentionDiagnosticKind, BackupRetentionPlan, BackupRetentionPlanError,
    BackupRetentionPolicy, BackupRetentionPolicyError, BackupRetentionReason, MilestoneRetention,
    apply_backup_retention, list_operational_backups, plan_backup_retention,
};
pub use types::{
    BackupArchiveInspection, BackupArchiveLimits, BackupArchiveLimitsError, BackupAuthenticity,
    BackupIntegrity, EncodedBackupArchive, InspectedBackupPayload,
};

const MANIFEST_PATH: &str = "longhorn/manifest.json";
const DEFLATE_LEVEL: i64 = 6;
