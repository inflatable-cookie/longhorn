use std::time::Duration;

use crate::StorageRootsResolve;
use crate::{DomainLocation, ResolvedFile, StorageFileEvidence};

use super::{
    StorageTransitionCleanupPlan, StorageTransitionCleanupReceipt, StorageTransitionError,
    StorageTransitionOutcome, StorageTransitionReceipt, StorageTransitionRequest, execution,
    inventory, io,
};
use longhorn_storage_layout::locator_matches;

/// Applies an exact, receipt-bound cleanup after target authority is reverified.
pub fn apply_storage_transition_cleanup(
    request: &StorageTransitionRequest<'_>,
    receipt: &StorageTransitionReceipt,
    plan: &StorageTransitionCleanupPlan,
    lock_timeout: Duration,
) -> Result<StorageTransitionCleanupReceipt, StorageTransitionError> {
    validate_binding(receipt, plan)?;
    if !crate::storage_layout::layout_digest_matches(
        &receipt.target_layout_digest,
        request.target_layout.digest(),
    ) || request.source_layout.identity().canonical_application_id()
        != request.target_layout.identity().canonical_application_id()
    {
        return Err(StorageTransitionError::CleanupRefused(
            "cleanup request does not match the receipt target layout".into(),
        ));
    }
    let (_first, _second) = execution::acquire_store_guards(request, lock_timeout)?;
    inventory::ensure_restore_inactive(request)?;
    let target_committed = locator_matches(
        &request.bootstrap,
        request.source_layout.identity().canonical_application_id(),
        receipt.transition_id(),
        &crate::storage_layout::layout_evidence(receipt.target_layout_digest()),
    )
    .map_err(|error| StorageTransitionError::Locator(error.to_string()))?;
    if !target_committed {
        return Err(StorageTransitionError::CleanupRefused(
            "fixed locator does not select the receipt target".into(),
        ));
    }

    let files = resolve_files(request, plan)?;
    let mut deleted_paths = Vec::new();
    let mut already_absent_paths = Vec::new();
    for ((source, target), evidence) in files.iter().zip(&plan.evidence) {
        let bytes = execution::read_ordinary(target)?.ok_or_else(|| changed(target, "target"))?;
        verify_evidence(target, &bytes, evidence, "target")?;
        if let Some(bytes) = execution::read_ordinary(source)? {
            verify_evidence(source, &bytes, evidence, "source")?;
            deleted_paths.push(source.full_path().to_path_buf());
        } else {
            already_absent_paths.push(source.full_path().to_path_buf());
        }
    }

    for path in &deleted_paths {
        io::remove_file(path)?;
    }
    Ok(StorageTransitionCleanupReceipt {
        transition_id: receipt.transition_id.clone(),
        deleted_paths,
        already_absent_paths,
    })
}

fn validate_binding(
    receipt: &StorageTransitionReceipt,
    plan: &StorageTransitionCleanupPlan,
) -> Result<(), StorageTransitionError> {
    if receipt.outcome != StorageTransitionOutcome::TargetCommitted
        || plan.transition_id != receipt.transition_id
        || plan.receipt_digest != receipt.receipt_digest
        || plan.paths != receipt.retained_source_paths
        || plan.evidence != receipt.retained_source_evidence
        || plan.paths.len() != plan.evidence.len()
    {
        return Err(StorageTransitionError::CleanupRefused(
            "cleanup plan does not match its committed receipt".into(),
        ));
    }
    Ok(())
}

fn resolve_files(
    request: &StorageTransitionRequest<'_>,
    plan: &StorageTransitionCleanupPlan,
) -> Result<Vec<(ResolvedFile, ResolvedFile)>, StorageTransitionError> {
    plan.paths
        .iter()
        .map(|source_path| {
            request
                .source_store
                .registered_descriptors()
                .find_map(|descriptor| {
                    let DomainLocation::File(source) =
                        request.source_store.roots.resolve(descriptor)
                    else {
                        return None;
                    };
                    if source.full_path() != source_path {
                        return None;
                    }
                    let target_descriptor = request
                        .target_store
                        .registered_descriptor(descriptor.id())?;
                    if target_descriptor != descriptor {
                        return None;
                    }
                    let DomainLocation::File(target) =
                        request.target_store.roots.resolve(target_descriptor)
                    else {
                        return None;
                    };
                    (target.full_path() != source.full_path()).then_some((source, target))
                })
                .ok_or_else(|| {
                    StorageTransitionError::CleanupRefused(format!(
                        "{} is not an exact retained registered source",
                        source_path.display()
                    ))
                })
        })
        .collect()
}

fn verify_evidence(
    file: &ResolvedFile,
    bytes: &[u8],
    expected: &StorageFileEvidence,
    authority: &str,
) -> Result<(), StorageTransitionError> {
    let observed = StorageFileEvidence::Present {
        byte_length: bytes.len(),
        sha256: crate::Sha256Digest::from_bytes(bytes),
    };
    if &observed != expected {
        return Err(changed(file, authority));
    }
    Ok(())
}

fn changed(file: &ResolvedFile, authority: &str) -> StorageTransitionError {
    StorageTransitionError::CleanupRefused(format!(
        "{authority} evidence changed at {}",
        file.full_path().display()
    ))
}
