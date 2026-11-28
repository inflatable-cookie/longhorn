mod apply;
mod listing;
mod plan;
mod policy;
mod types;

pub use apply::{BackupRetentionApplyError, BackupRetentionApplyReceipt, apply_backup_retention};
pub use listing::{list_operational_backups, list_operational_backups_with_envelope};
pub use plan::{
    BackupRetentionDeletion, BackupRetentionPlan, BackupRetentionPlanError, BackupRetentionReason,
    plan_backup_retention,
};
pub use policy::{BackupRetentionPolicy, BackupRetentionPolicyError, MilestoneRetention};
pub use types::{
    BackupOperationalCandidate, BackupOperationalListing, BackupRetentionDiagnostic,
    BackupRetentionDiagnosticKind,
};

pub(crate) use policy::HARD_MAX_SCAN_ENTRIES;
pub(crate) use types::diagnostic;
