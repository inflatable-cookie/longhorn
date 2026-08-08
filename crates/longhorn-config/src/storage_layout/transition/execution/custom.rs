use std::{collections::BTreeMap, path::Path};

use crate::{
    BackupAdapterCapture, BackupAdapterCaptureRequest, BackupAdapterInspectRequest,
    BackupAdapterPayloadRef, BackupAdapterRestoreOutcome, BackupAdapterRestoreRequest,
    BackupLimits, BackupPayloadPath, Sha256Digest, backup::BackupAdapterCaptureMode,
};
use longhorn_core::DomainId;

use super::super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError, StorageTransitionPlan,
    StorageTransitionRequest, TransitionDecision,
};

pub(crate) struct CapturedCustom {
    source_schema_version: Option<longhorn_core::SchemaVersion>,
    paths: Vec<BackupPayloadPath>,
    bytes: Vec<Vec<u8>>,
    preview: crate::BackupAdapterRestorePreview,
}

pub(crate) fn capture_custom(
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

pub(crate) fn restore_custom(
    request: &StorageTransitionRequest<'_>,
    entry: &super::super::StorageTransitionDomain,
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
