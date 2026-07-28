use std::{
    fs,
    io::{self, Read},
};

use crate::{
    BackupArchiveLimits, BackupLimits, BackupPublicationReceipt, ConfigStore, DomainLocation,
    RestoreExecutionError, RestoreExecutionStage, RestoreFailureTerminal, RestoreStaging,
    Sha256Digest,
};

use super::{
    execution::failure,
    journal::{JournalDescriptor, JournalEvidence, RollbackEntry},
    live_io,
};

pub(super) struct CapturedRollback {
    pub(super) entries: Vec<RollbackEntry>,
    pub(super) descriptors: Vec<JournalDescriptor>,
}

pub(super) fn capture_rollback(
    store: &ConfigStore,
    staging: &RestoreStaging,
    limits: BackupLimits,
) -> Result<CapturedRollback, RestoreExecutionError> {
    let mut rollback = Vec::with_capacity(staging.domains.len());
    let mut descriptors = Vec::with_capacity(staging.domains.len());
    let mut total = 0usize;
    for staged in &staging.domains {
        let descriptor = store
            .registered_descriptor(&staged.domain)
            .ok_or_else(|| stale_failure(&staged.domain, "domain is no longer registered"))?;
        let DomainLocation::File(file) = store.roots.resolve(descriptor) else {
            return Err(stale_failure(
                &staged.domain,
                "domain no longer resolves to a file",
            ));
        };
        if file.full_path() != staged.path {
            return Err(stale_failure(
                &staged.domain,
                "registered target path changed after staging",
            ));
        }
        let expected = JournalEvidence::from_current(&staged.current);
        if let JournalEvidence::Present { byte_length, .. } = &expected {
            let length = usize::try_from(*byte_length).map_err(|_| {
                rollback_limit_failure(&staged.domain, "source length is not addressable")
            })?;
            if length > limits.max_domain_bytes() {
                return Err(rollback_limit_failure(
                    &staged.domain,
                    format!(
                        "source length {length} exceeds rollback domain limit {}",
                        limits.max_domain_bytes()
                    ),
                ));
            }
            total = total.checked_add(length).ok_or_else(|| {
                rollback_limit_failure(&staged.domain, "rollback total length overflow")
            })?;
            if total > limits.max_total_bytes() {
                return Err(rollback_limit_failure(
                    &staged.domain,
                    format!(
                        "rollback total {total} exceeds limit {}",
                        limits.max_total_bytes()
                    ),
                ));
            }
        }
        let bytes = live_io::read_exact_state(&file, &expected).map_err(|error| {
            failure(
                RestoreExecutionStage::CaptureRollback,
                Some(staged.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                error,
            )
        })?;
        rollback.push(RollbackEntry {
            domain: staged.domain.clone(),
            bytes,
        });
        descriptors.push(JournalDescriptor {
            domain: staged.domain.clone(),
            storage_class: descriptor.storage_class(),
            relative_path: descriptor
                .file_path()
                .expect("file-backed descriptor has a path")
                .as_str()
                .to_owned(),
        });
    }
    Ok(CapturedRollback {
        entries: rollback,
        descriptors,
    })
}

pub(super) fn verify_published_safety(
    receipt: &BackupPublicationReceipt,
    limits: BackupArchiveLimits,
) -> io::Result<()> {
    let mut input = fs::File::open(&receipt.path)?;
    let observed = input.metadata()?.len();
    if observed > limits.max_archive_bytes() as u64 {
        return Err(io::Error::other(
            "published safety archive exceeds configured byte limit",
        ));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(observed).unwrap_or(0));
    Read::by_ref(&mut input)
        .take(limits.max_archive_bytes() as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limits.max_archive_bytes()
        || Sha256Digest::from_bytes(&bytes) != receipt.archive_sha256
    {
        return Err(io::Error::other(
            "published safety archive no longer matches verified receipt",
        ));
    }
    crate::inspect_backup_archive(&bytes, limits)
        .map(|_| ())
        .map_err(|error| io::Error::other(error.to_string()))
}

fn rollback_limit_failure(
    domain: &longhorn_core::DomainId,
    detail: impl Into<String>,
) -> RestoreExecutionError {
    failure(
        RestoreExecutionStage::CaptureRollback,
        Some(domain.clone()),
        RestoreFailureTerminal::NoLiveMutation,
        detail.into(),
    )
}

fn stale_failure(
    domain: &longhorn_core::DomainId,
    detail: impl Into<String>,
) -> RestoreExecutionError {
    failure(
        RestoreExecutionStage::RecheckCurrent,
        Some(domain.clone()),
        RestoreFailureTerminal::NoLiveMutation,
        detail.into(),
    )
}
