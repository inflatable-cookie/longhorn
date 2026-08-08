use std::collections::BTreeSet;

use crate::RootKind;

use super::{
    INVENTORY_ROOTS, MIGRATING_ROOTS, StorageTransitionConflict, StorageTransitionConflictKind,
    StorageTransitionError, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionRequest, confirmation_digest, equal_root_paths,
    evidence_digest, inspect_domain, root_conflicts, scan_layout,
};

/// Inventories both layouts and current evidence without mutation.
pub fn inspect_storage_transition(
    request: &StorageTransitionRequest<'_>,
) -> Result<StorageTransitionPreview, StorageTransitionError> {
    validate_request(request)?;
    let mut conflicts = root_conflicts(request);
    let mut domains = Vec::new();
    let mut known_source = BTreeSet::new();
    let mut known_target = BTreeSet::new();
    let mut total_bytes = 0usize;

    for descriptor in request.source_store.registered_descriptors() {
        let target_descriptor = request
            .target_store
            .registered_descriptor(descriptor.id())
            .ok_or_else(|| StorageTransitionError::DescriptorMismatch {
                domain: descriptor.id().clone(),
            })?;
        if target_descriptor != descriptor {
            return Err(StorageTransitionError::DescriptorMismatch {
                domain: descriptor.id().clone(),
            });
        }
        let domain = inspect_domain(
            request,
            descriptor,
            &mut known_source,
            &mut known_target,
            &mut total_bytes,
            &mut conflicts,
        )?;
        domains.push(domain);
    }

    let same_layout = request.source_layout.digest() == request.target_layout.digest();
    let source_unknown = if same_layout {
        // Profile adoption changes no file authority. Walking unrelated retained
        // product data here can be unbounded and cannot affect the plan.
        Vec::new()
    } else {
        scan_layout(
            request.source_layout,
            &known_source,
            request.limits,
            &mut total_bytes,
            &INVENTORY_ROOTS,
            &BTreeSet::new(),
        )?
    };
    let equal_roots = equal_root_paths(request);
    let target_unknown = scan_layout(
        request.target_layout,
        &known_target,
        request.limits,
        &mut total_bytes,
        &MIGRATING_ROOTS,
        &equal_roots,
    )?;
    for unknown in &target_unknown {
        conflicts.push(StorageTransitionConflict {
            kind: StorageTransitionConflictKind::UnknownTargetFile,
            domain: None,
            path: Some(unknown.path.clone()),
            detail: "target contains an unregistered file".into(),
        });
    }
    domains.sort_by(|left, right| left.domain.cmp(&right.domain));
    conflicts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.domain.cmp(&right.domain))
    });
    let evidence_digest = evidence_digest(&domains, &source_unknown, &target_unknown, &conflicts);
    let confirmation_digest = confirmation_digest(request, &evidence_digest);
    Ok(StorageTransitionPreview {
        source_layout_digest: request.source_layout.digest().clone(),
        target_layout_digest: request.target_layout.digest().clone(),
        target_selection: request.target_selection.clone(),
        domains,
        source_unknown,
        target_unknown,
        conflicts,
        evidence_digest,
        confirmation_digest,
    })
}

/// Refuses conflicted inventory and freezes the exact preview.
pub fn plan_storage_transition(
    preview: &StorageTransitionPreview,
) -> Result<StorageTransitionPlan, StorageTransitionPlanError> {
    if !preview.conflicts.is_empty() {
        return Err(StorageTransitionPlanError::Conflicts {
            count: preview.conflicts.len(),
        });
    }
    Ok(StorageTransitionPlan {
        preview: preview.clone(),
    })
}

fn validate_request(request: &StorageTransitionRequest<'_>) -> Result<(), StorageTransitionError> {
    let source_identity = request.source_layout.identity();
    let target_identity = request.target_layout.identity();
    if source_identity.canonical_application_id() != target_identity.canonical_application_id() {
        return Err(StorageTransitionError::LayoutIdentityMismatch);
    }
    if request.source_store.roots != *request.source_layout.storage_roots()
        || request.target_store.roots != *request.target_layout.storage_roots()
    {
        return Err(StorageTransitionError::LayoutStoreMismatch);
    }
    if request.target_selection.profile() != request.target_layout.profile() {
        return Err(StorageTransitionError::TargetSelectionMismatch);
    }
    if let Some(root) = request.target_selection.explicit_root() {
        let expected = request
            .target_layout
            .root(RootKind::Config)
            .map(|entry| entry.path());
        if expected != Some(&root.join("config")) {
            return Err(StorageTransitionError::TargetSelectionMismatch);
        }
    }
    Ok(())
}
