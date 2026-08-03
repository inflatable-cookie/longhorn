mod orchestrator;
mod receipt;

use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    BackupAdapterCapture, BackupAdapterCaptureRequest, BackupAdapterInspectRequest,
    BackupAdapterPayloadRef, BackupAdapterRestoreOutcome, BackupAdapterRestoreRequest,
    BackupLimits, BackupPayloadPath, Sha256Digest, backup::BackupAdapterCaptureMode,
};
use longhorn_core::DomainId;

use super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError, StorageTransitionPlan,
    StorageTransitionRequest, TransitionDecision, io, journal::TransitionJournal,
};
#[cfg(test)]
pub(crate) use orchestrator::{InjectedFailure, execute_inner};
pub use orchestrator::{execute_storage_transition, recover_storage_transition};
use receipt::receipt_digest;

fn acquire_adapter_guards<'request>(
    request: &'request StorageTransitionRequest<'request>,
    timeout: std::time::Duration,
) -> Result<Vec<Box<dyn super::StorageTransitionGuard + 'request>>, StorageTransitionError> {
    let mut authorities = BTreeMap::new();
    for descriptor in request.source_store.registered_descriptors() {
        if let TransitionDecision::Custom(source, target) = request.catalog.decision(descriptor) {
            authorities
                .entry(source.transition_authority().to_owned())
                .or_insert((source, descriptor));
            authorities
                .entry(target.transition_authority().to_owned())
                .or_insert((target, descriptor));
        }
    }
    authorities
        .into_values()
        .map(|(adapter, descriptor)| {
            adapter
                .acquire_transition_guard(descriptor, timeout)
                .map_err(|error| StorageTransitionError::Adapter {
                    domain: descriptor.id().clone(),
                    detail: error.to_string(),
                })
        })
        .collect()
}

pub(super) fn acquire_store_guards<'request>(
    request: &'request StorageTransitionRequest<'request>,
    timeout: std::time::Duration,
) -> Result<
    (
        crate::coordination::CoordinationGuard<'request>,
        Option<crate::coordination::CoordinationGuard<'request>>,
    ),
    StorageTransitionError,
> {
    let source_root = request.source_store.coordinator.authority_root();
    let target_root = request.target_store.coordinator.authority_root();
    if source_root == target_root {
        return Ok((
            request
                .source_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            None,
        ));
    }
    if source_root < target_root {
        Ok((
            request
                .source_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            Some(
                request
                    .target_store
                    .coordinator
                    .acquire(timeout)
                    .map_err(StorageTransitionError::Coordination)?,
            ),
        ))
    } else {
        Ok((
            request
                .target_store
                .coordinator
                .acquire(timeout)
                .map_err(StorageTransitionError::Coordination)?,
            Some(
                request
                    .source_store
                    .coordinator
                    .acquire(timeout)
                    .map_err(StorageTransitionError::Coordination)?,
            ),
        ))
    }
}

struct CapturedCustom {
    source_schema_version: Option<longhorn_core::SchemaVersion>,
    paths: Vec<BackupPayloadPath>,
    bytes: Vec<Vec<u8>>,
    preview: crate::BackupAdapterRestorePreview,
}

fn capture_custom(
    request: &StorageTransitionRequest<'_>,
    plan: &StorageTransitionPlan,
) -> Result<BTreeMap<DomainId, CapturedCustom>, StorageTransitionError> {
    let limits = BackupLimits::new(
        request.limits.max_file_bytes(),
        request.limits.max_total_bytes(),
    )
    .map_err(|_| StorageTransitionError::InvalidLimits)?;
    let mut captures = BTreeMap::new();
    for entry in plan.domains() {
        if !matches!(entry.action, StorageTransitionAction::CustomAdapter { .. }) {
            continue;
        }
        let descriptor = request
            .source_store
            .registered_descriptor(&entry.domain)
            .ok_or_else(|| StorageTransitionError::DescriptorMismatch {
                domain: entry.domain.clone(),
            })?;
        let TransitionDecision::Custom(source, target) = request.catalog.decision(descriptor)
        else {
            return Err(StorageTransitionError::StalePlan);
        };
        if matches!(
            source.capabilities().capture(),
            BackupAdapterCaptureMode::Excluded(_)
        ) {
            return Err(StorageTransitionError::UnavailableDomain {
                domain: entry.domain.clone(),
            });
        }
        let captured = source
            .capture(BackupAdapterCaptureRequest::new(descriptor, limits))
            .map_err(|error| StorageTransitionError::Adapter {
                domain: entry.domain.clone(),
                detail: error.to_string(),
            })?;
        let BackupAdapterCapture::Present {
            source_schema_version,
            mut payloads,
        } = captured
        else {
            return Err(StorageTransitionError::StalePlan);
        };
        payloads.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        if payloads.is_empty() {
            return Err(StorageTransitionError::Adapter {
                domain: entry.domain.clone(),
                detail: "adapter returned no transition payloads".into(),
            });
        }
        let mut paths = Vec::with_capacity(payloads.len());
        let mut bytes = Vec::with_capacity(payloads.len());
        let mut total = 0usize;
        for payload in payloads {
            total = total
                .checked_add(payload.bytes().len())
                .filter(|value| *value <= request.limits.max_total_bytes())
                .ok_or_else(|| StorageTransitionError::BoundExceeded {
                    path: Path::new(source.id().as_str()).to_path_buf(),
                })?;
            paths.push(BackupPayloadPath::adapter(
                descriptor.id(),
                payload.relative_path(),
            ));
            bytes.push(payload.bytes);
        }
        let refs = payload_refs(&paths, &bytes);
        let preview = target
            .inspect(BackupAdapterInspectRequest::new(
                descriptor,
                crate::BackupSourceState::Present,
                Some(source_schema_version),
                refs,
            ))
            .map_err(|error| StorageTransitionError::Adapter {
                domain: entry.domain.clone(),
                detail: error.to_string(),
            })?;
        let Some(StorageFileEvidence::Semantic { sha256: expected }) =
            entry.source_evidence.as_ref()
        else {
            return Err(StorageTransitionError::StalePlan);
        };
        if preview.target_evidence().sha256() != Some(expected)
            || preview.current_evidence().sha256()
                != entry.target_evidence.as_ref().and_then(semantic_digest)
        {
            return Err(StorageTransitionError::StalePlan);
        }
        captures.insert(
            entry.domain.clone(),
            CapturedCustom {
                source_schema_version: Some(source_schema_version),
                paths,
                bytes,
                preview,
            },
        );
    }
    Ok(captures)
}

fn restore_custom(
    request: &StorageTransitionRequest<'_>,
    entry: &super::StorageTransitionDomain,
    captures: &BTreeMap<DomainId, CapturedCustom>,
) -> Result<(), StorageTransitionError> {
    let descriptor = request
        .target_store
        .registered_descriptor(&entry.domain)
        .ok_or_else(|| StorageTransitionError::DescriptorMismatch {
            domain: entry.domain.clone(),
        })?;
    let TransitionDecision::Custom(_, target) = request.catalog.decision(descriptor) else {
        return Err(StorageTransitionError::StalePlan);
    };
    let captured = captures
        .get(&entry.domain)
        .ok_or(StorageTransitionError::StalePlan)?;
    let inspect = BackupAdapterInspectRequest::new(
        descriptor,
        crate::BackupSourceState::Present,
        captured.source_schema_version,
        payload_refs(&captured.paths, &captured.bytes),
    );
    let outcome = target
        .restore(BackupAdapterRestoreRequest::new(inspect, &captured.preview))
        .map_err(|error| StorageTransitionError::Adapter {
            domain: entry.domain.clone(),
            detail: error.to_string(),
        })?;
    match outcome {
        BackupAdapterRestoreOutcome::Verified { evidence }
            if Some(&evidence) == captured.preview.target_evidence().sha256() =>
        {
            Ok(())
        }
        BackupAdapterRestoreOutcome::Verified { .. }
        | BackupAdapterRestoreOutcome::RolledBack { .. }
        | BackupAdapterRestoreOutcome::RecoveryRequired => {
            Err(StorageTransitionError::RecoveryRequired(format!(
                "custom domain {} did not verify target",
                entry.domain
            )))
        }
    }
}

fn stage_ordinary(
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

fn stage_path(staging: &Path, domain: &DomainId) -> std::path::PathBuf {
    staging.join("ordinary").join(format!("{domain}.bin"))
}

fn verify_journal_authority(
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

fn verify_path(
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

fn semantic_digest(evidence: &StorageFileEvidence) -> Option<&Sha256Digest> {
    match evidence {
        StorageFileEvidence::Semantic { sha256 } => Some(sha256),
        _ => None,
    }
}

fn payload_refs<'a>(
    paths: &'a [BackupPayloadPath],
    bytes: &'a [Vec<u8>],
) -> Vec<BackupAdapterPayloadRef<'a>> {
    paths
        .iter()
        .zip(bytes)
        .map(|(path, bytes)| BackupAdapterPayloadRef::new(path, bytes))
        .collect()
}

fn fs_error(path: &Path, error: std::io::Error) -> StorageTransitionError {
    StorageTransitionError::Filesystem {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
