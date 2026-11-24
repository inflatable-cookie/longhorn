use std::path::{Path, PathBuf};

use crate::store::load::max_live_domain_bytes;
use crate::{ConfigStore, DomainLocation, ResolvedFile, Sha256Digest, StorageRootsResolve};
use longhorn_core::DomainId;

use super::super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError, StorageTransitionPlan,
    StorageTransitionRequest, TransitionDecision, io, journal::TransitionJournal,
};

pub(crate) fn stage_ordinary(
    store: &ConfigStore,
    plan: &StorageTransitionPlan,
    staging: &Path,
) -> Result<(), StorageTransitionError> {
    for entry in plan.domains() {
        if entry.action != StorageTransitionAction::CopyOrdinary {
            continue;
        }
        let source = resolve_ordinary(store, &entry.domain)?;
        if entry.source_path.as_deref() != Some(source.full_path()) {
            return Err(StorageTransitionError::StalePlan);
        }
        let bytes = read_ordinary(&source)?.ok_or(StorageTransitionError::StalePlan)?;
        verify_bytes(&bytes, entry.source_evidence.as_ref())?;
        io::atomic_write(&staging.join(stage_path(&entry.domain)), &bytes)?;
    }
    Ok(())
}

pub(crate) fn stage_path(domain: &DomainId) -> PathBuf {
    Path::new("ordinary").join(format!("{domain}.bin"))
}

pub(crate) fn read_staged(
    bootstrap: &Path,
    staging_relative: &Path,
    domain: &DomainId,
    expected: Option<&StorageFileEvidence>,
) -> Result<Vec<u8>, StorageTransitionError> {
    let relative = staging_relative.join(stage_path(domain));
    let bytes = io::read(bootstrap, &relative, max_live_domain_bytes())?
        .ok_or(StorageTransitionError::StalePlan)?;
    verify_bytes(&bytes, expected)?;
    Ok(bytes)
}

fn resolve_ordinary(
    store: &ConfigStore,
    id: &DomainId,
) -> Result<ResolvedFile, StorageTransitionError> {
    let descriptor = store
        .registered_descriptor(id)
        .ok_or_else(|| StorageTransitionError::DescriptorMismatch { domain: id.clone() })?;
    match store.roots.resolve(descriptor) {
        DomainLocation::File(file) => Ok(file),
        _ => Err(StorageTransitionError::UnavailableDomain { domain: id.clone() }),
    }
}

pub(crate) fn read_ordinary(
    file: &ResolvedFile,
) -> Result<Option<Vec<u8>>, StorageTransitionError> {
    io::read(
        file.root(),
        file.relative_path().as_path(),
        max_live_domain_bytes(),
    )
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
        let id = DomainId::new(&entry.domain)
            .map_err(|_| StorageTransitionError::Journal("invalid journal domain".into()))?;
        if entry.custom || (entry.source_path.is_none() && entry.target_path.is_none()) {
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
        } else {
            let (store, journal_path) = if target {
                (request.target_store, &entry.target_path)
            } else {
                (request.source_store, &entry.source_path)
            };
            let file = resolve_ordinary(store, &id)?;
            if journal_path.as_deref() != Some(file.full_path()) {
                return Err(StorageTransitionError::RecoveryRequired(format!(
                    "resolved path for {id} no longer matches transition journal"
                )));
            }
            verify_path(&file, expected)?;
        }
    }
    Ok(())
}

pub(crate) fn verify_path(
    file: &ResolvedFile,
    expected: Option<&StorageFileEvidence>,
) -> Result<(), StorageTransitionError> {
    let observed = match read_ordinary(file)? {
        Some(bytes) => StorageFileEvidence::Present {
            byte_length: bytes.len(),
            sha256: Sha256Digest::from_bytes(&bytes),
        },
        None => StorageFileEvidence::Absent,
    };
    if Some(&observed) == expected {
        Ok(())
    } else {
        Err(StorageTransitionError::RecoveryRequired(format!(
            "transition evidence changed at {}",
            file.full_path().display()
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
