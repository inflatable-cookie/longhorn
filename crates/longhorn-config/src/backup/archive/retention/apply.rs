use std::{error::Error, fmt, fs, path::PathBuf};

use crate::{BackupArchiveLimits, Sha256Digest, backup::archive::publication::read_archive_bytes};

use super::BackupRetentionPlan;

/// Applied deletion receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionApplyReceipt {
    /// Exact paths removed.
    pub deleted: Vec<PathBuf>,
}

/// Failure while rechecking or applying one exact deletion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRetentionApplyError {
    /// Candidate path.
    pub path: PathBuf,
    /// Whether this candidate was already removed.
    pub deleted: bool,
    /// Failure detail.
    pub detail: String,
}

impl fmt::Display for BackupRetentionApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot apply backup retention to {}: {}",
            self.path.display(),
            self.detail
        )
    }
}

impl Error for BackupRetentionApplyError {}

/// Rechecks exact bytes and removes only unchanged planned candidates.
pub fn apply_backup_retention(
    plan: &BackupRetentionPlan,
    archive_limits: BackupArchiveLimits,
) -> Result<BackupRetentionApplyReceipt, BackupRetentionApplyError> {
    let mut deleted = Vec::new();
    for deletion in &plan.deletions {
        if deletion.path.parent() != Some(plan.root.as_path()) {
            return Err(apply_error(
                deletion.path.clone(),
                false,
                "planned path escaped the operational root",
            ));
        }
        let limit = if deletion.path.extension() == Some(std::ffi::OsStr::new("age")) {
            plan.max_ciphertext_bytes.ok_or_else(|| {
                apply_error(
                    deletion.path.clone(),
                    false,
                    "encrypted archive was not inspected",
                )
            })?
        } else {
            archive_limits.max_archive_bytes()
        };
        let bytes = read_archive_bytes(&deletion.path, limit)
            .map_err(|error| apply_error(deletion.path.clone(), false, error.to_string()))?;
        if Sha256Digest::from_bytes(&bytes) != deletion.archive_sha256 {
            return Err(apply_error(
                deletion.path.clone(),
                false,
                "archive changed after retention planning",
            ));
        }
        fs::remove_file(&deletion.path)
            .map_err(|error| apply_error(deletion.path.clone(), false, error.to_string()))?;
        deleted.push(deletion.path.clone());
    }
    Ok(BackupRetentionApplyReceipt { deleted })
}

fn apply_error(
    path: PathBuf,
    deleted: bool,
    detail: impl Into<String>,
) -> BackupRetentionApplyError {
    BackupRetentionApplyError {
        path,
        deleted,
        detail: detail.into(),
    }
}
