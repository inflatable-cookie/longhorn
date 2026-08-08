use std::{fs, path::Path};

use crate::Sha256Digest;
use longhorn_core::DomainId;

use super::super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError, StorageTransitionPlan,
    StorageTransitionRequest, TransitionDecision, io, journal::TransitionJournal,
};

pub(crate) fn stage_ordinary(
    plan: &StorageTransitionPlan,
    staging: &Path,
) -> Result<(), StorageTransitionError> {
    for entry in plan.domains() {
        if entry.action != StorageTransitionAction::CopyOrdinary {
            continue;
        }
        let source = entry
            .source_path
            .as_deref()
            .ok_or(StorageTransitionError::StalePlan)?;
        let bytes = fs::read(source).map_err(|error| fs_error(source, error))?;
        verify_bytes(&bytes, entry.source_evidence.as_ref())?;
        io::atomic_write(&stage_path(staging, &entry.domain), &bytes)?;
    }
    Ok(())
}

pub(crate) fn stage_path(staging: &Path, domain: &DomainId) -> std::path::PathBuf {
    staging.join("ordinary").join(format!("{domain}.bin"))
}

pub(crate) fn verify_journal_authority(
    request: &StorageTransitionRequest<'_>,
    active: &TransitionJournal,
    target: bool,
) -> Result<(), StorageTransitionError> {
    for entry in &active.domains {
        let expected = if target {
            entry.target_expected.as_ref()
        } else {
            entry.source.as_ref()
        };
        if entry.custom {
            let id = DomainId::new(&entry.domain)
                .map_err(|_| StorageTransitionError::Journal("invalid journal domain".into()))?;
            let descriptor = request
                .source_store
                .registered_descriptor(&id)
                .ok_or_else(|| StorageTransitionError::DescriptorMismatch { domain: id.clone() })?;
            let TransitionDecision::Custom(source, target_adapter) =
                request.catalog.decision(descriptor)
            else {
                return Err(StorageTransitionError::StalePlan);
            };
            let observed = if target {
                target_adapter.current_evidence(descriptor)
            } else {
                source.current_evidence(descriptor)
            }
            .map_err(|error| StorageTransitionError::Adapter {
                domain: id,
                detail: error.to_string(),
            })?
            .map(|sha256| StorageFileEvidence::Semantic { sha256 });
            if observed.as_ref() != expected {
                return Err(StorageTransitionError::RecoveryRequired(format!(
                    "custom authority evidence changed for {}",
                    entry.domain
                )));
            }
        } else if let Some(path) = if target {
            entry.target_path.as_deref()
        } else {
            entry.source_path.as_deref()
        } {
            verify_path(path, expected)?;
        }
    }
    Ok(())
}

pub(crate) fn verify_path(
    path: &Path,
    expected: Option<&StorageFileEvidence>,
) -> Result<(), StorageTransitionError> {
    let observed = match fs::read(path) {
        Ok(bytes) => StorageFileEvidence::Present {
            byte_length: bytes.len(),
            sha256: Sha256Digest::from_bytes(&bytes),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => StorageFileEvidence::Absent,
        Err(error) => return Err(fs_error(path, error)),
    };
    if Some(&observed) == expected {
        Ok(())
    } else {
        Err(StorageTransitionError::RecoveryRequired(format!(
            "transition evidence changed at {}",
            path.display()
        )))
    }
}

fn verify_bytes(
    bytes: &[u8],
    expected: Option<&StorageFileEvidence>,
) -> Result<(), StorageTransitionError> {
    let observed = StorageFileEvidence::Present {
        byte_length: bytes.len(),
        sha256: Sha256Digest::from_bytes(bytes),
    };
    if Some(&observed) == expected {
        Ok(())
    } else {
        Err(StorageTransitionError::StalePlan)
    }
}

pub(crate) fn fs_error(path: &Path, error: std::io::Error) -> StorageTransitionError {
    StorageTransitionError::Filesystem {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}
