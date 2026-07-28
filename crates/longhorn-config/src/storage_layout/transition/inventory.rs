mod digest;
mod roots;
mod scanning;

use std::{collections::BTreeSet, path::PathBuf};

use crate::{
    BackupAdapterRestoreParticipation, DomainDescriptor, DomainLocation, RootKind, StorageClass,
};

use super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionConflict,
    StorageTransitionConflictKind, StorageTransitionDomain, StorageTransitionError,
    StorageTransitionExclusion, StorageTransitionPlan, StorageTransitionPlanError,
    StorageTransitionPreview, StorageTransitionRequest, TransitionDecision,
};
use digest::{confirmation_digest, evidence_digest};
use roots::{equal_root_paths, root_conflicts};
use scanning::scan_layout;
pub(super) use scanning::{read_evidence, scan_directory};

const INVENTORY_ROOTS: [RootKind; 7] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Workspace,
    RootKind::Cache,
    RootKind::Log,
    RootKind::Runtime,
];
const MIGRATING_ROOTS: [RootKind; 5] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Workspace,
    RootKind::Log,
];

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

    let source_unknown = scan_layout(
        request.source_layout,
        &known_source,
        request.limits,
        &mut total_bytes,
        &INVENTORY_ROOTS,
        &BTreeSet::new(),
    )?;
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

fn inspect_domain(
    request: &StorageTransitionRequest<'_>,
    descriptor: &DomainDescriptor,
    known_source: &mut BTreeSet<PathBuf>,
    known_target: &mut BTreeSet<PathBuf>,
    total_bytes: &mut usize,
    conflicts: &mut Vec<StorageTransitionConflict>,
) -> Result<StorageTransitionDomain, StorageTransitionError> {
    let class = descriptor.storage_class();
    remember_resolved_paths(request, descriptor, known_source, known_target);
    if let Some(exclusion) = default_exclusion(class, request.include_logs) {
        return Ok(excluded_domain(descriptor, exclusion));
    }
    match request.catalog.decision(descriptor) {
        TransitionDecision::Missing => Err(StorageTransitionError::MissingPolicy {
            domain: descriptor.id().clone(),
        }),
        TransitionDecision::DescriptorChanged => Err(StorageTransitionError::DescriptorMismatch {
            domain: descriptor.id().clone(),
        }),
        TransitionDecision::Exclude(reason) => Ok(excluded_domain(
            descriptor,
            StorageTransitionExclusion::Catalog(reason.as_str().into()),
        )),
        TransitionDecision::Include => inspect_ordinary(
            request,
            descriptor,
            known_source,
            known_target,
            total_bytes,
            conflicts,
        ),
        TransitionDecision::Custom(source, target) => {
            if matches!(
                target.capabilities().restore(),
                BackupAdapterRestoreParticipation::Excluded(_)
            ) {
                return Err(StorageTransitionError::UnavailableDomain {
                    domain: descriptor.id().clone(),
                });
            }
            remember_adapter_paths(request.source_layout, descriptor, source, known_source)?;
            remember_adapter_paths(request.target_layout, descriptor, target, known_target)?;
            let source_evidence = source.current_evidence(descriptor).map_err(|error| {
                StorageTransitionError::Adapter {
                    domain: descriptor.id().clone(),
                    detail: error.to_string(),
                }
            })?;
            let target_evidence = target.current_evidence(descriptor).map_err(|error| {
                StorageTransitionError::Adapter {
                    domain: descriptor.id().clone(),
                    detail: error.to_string(),
                }
            })?;
            let action = match (&source_evidence, &target_evidence) {
                (None, None) => StorageTransitionAction::Absent,
                (Some(left), Some(right)) if left == right => StorageTransitionAction::Identical,
                (Some(_), None) => StorageTransitionAction::CustomAdapter {
                    source_adapter: source.id().clone(),
                    target_adapter: target.id().clone(),
                    participation: target.capabilities().restore().clone(),
                },
                _ => {
                    conflicts.push(StorageTransitionConflict {
                        kind: StorageTransitionConflictKind::TargetOccupied,
                        domain: Some(descriptor.id().clone()),
                        path: None,
                        detail: "custom target semantic evidence conflicts".into(),
                    });
                    StorageTransitionAction::CustomAdapter {
                        source_adapter: source.id().clone(),
                        target_adapter: target.id().clone(),
                        participation: target.capabilities().restore().clone(),
                    }
                }
            };
            Ok(StorageTransitionDomain {
                domain: descriptor.id().clone(),
                storage_class: class,
                source_path: None,
                target_path: None,
                source_evidence: source_evidence
                    .map(|sha256| StorageFileEvidence::Semantic { sha256 }),
                target_evidence: target_evidence
                    .map(|sha256| StorageFileEvidence::Semantic { sha256 }),
                action,
            })
        }
    }
}

fn remember_adapter_paths(
    layout: &crate::ResolvedStorageLayout,
    descriptor: &DomainDescriptor,
    adapter: &dyn super::StorageTransitionAdapter,
    known: &mut BTreeSet<PathBuf>,
) -> Result<(), StorageTransitionError> {
    for path in adapter.owned_paths(descriptor) {
        let confined = INVENTORY_ROOTS.iter().any(|kind| {
            layout
                .root(*kind)
                .is_some_and(|root| path.starts_with(root.path()))
        });
        if !path.is_absolute() || !confined {
            return Err(StorageTransitionError::Adapter {
                domain: descriptor.id().clone(),
                detail: "adapter-owned path is outside the resolved layout".into(),
            });
        }
        known.insert(path);
    }
    Ok(())
}

fn inspect_ordinary(
    request: &StorageTransitionRequest<'_>,
    descriptor: &DomainDescriptor,
    known_source: &mut BTreeSet<PathBuf>,
    known_target: &mut BTreeSet<PathBuf>,
    total_bytes: &mut usize,
    conflicts: &mut Vec<StorageTransitionConflict>,
) -> Result<StorageTransitionDomain, StorageTransitionError> {
    let DomainLocation::File(source) = request.source_store.roots.resolve(descriptor) else {
        return Err(StorageTransitionError::UnavailableDomain {
            domain: descriptor.id().clone(),
        });
    };
    let DomainLocation::File(target) = request.target_store.roots.resolve(descriptor) else {
        return Err(StorageTransitionError::UnavailableDomain {
            domain: descriptor.id().clone(),
        });
    };
    known_source.insert(source.full_path().to_path_buf());
    known_target.insert(target.full_path().to_path_buf());
    let source_evidence = read_evidence(source.full_path(), request.limits, total_bytes)?;
    let target_evidence = if source.full_path() == target.full_path() {
        source_evidence.clone()
    } else {
        read_evidence(target.full_path(), request.limits, total_bytes)?
    };
    let action = if source.full_path() == target.full_path() {
        StorageTransitionAction::SameAuthority
    } else {
        match (&source_evidence, &target_evidence) {
            (StorageFileEvidence::Absent, StorageFileEvidence::Absent) => {
                StorageTransitionAction::Absent
            }
            (StorageFileEvidence::Present { sha256: left, .. }, StorageFileEvidence::Absent) => {
                let _ = left;
                StorageTransitionAction::CopyOrdinary
            }
            (
                StorageFileEvidence::Present { sha256: left, .. },
                StorageFileEvidence::Present { sha256: right, .. },
            ) if left == right => StorageTransitionAction::Identical,
            _ => {
                conflicts.push(StorageTransitionConflict {
                    kind: StorageTransitionConflictKind::TargetOccupied,
                    domain: Some(descriptor.id().clone()),
                    path: Some(target.full_path().to_path_buf()),
                    detail: "ordinary target evidence conflicts".into(),
                });
                StorageTransitionAction::CopyOrdinary
            }
        }
    };
    Ok(StorageTransitionDomain {
        domain: descriptor.id().clone(),
        storage_class: descriptor.storage_class(),
        source_path: Some(source.full_path().to_path_buf()),
        target_path: Some(target.full_path().to_path_buf()),
        source_evidence: Some(source_evidence),
        target_evidence: Some(target_evidence),
        action,
    })
}

fn remember_resolved_paths(
    request: &StorageTransitionRequest<'_>,
    descriptor: &DomainDescriptor,
    source: &mut BTreeSet<PathBuf>,
    target: &mut BTreeSet<PathBuf>,
) {
    if let DomainLocation::File(file) = request.source_store.roots.resolve(descriptor) {
        source.insert(file.full_path().to_path_buf());
    }
    if let DomainLocation::File(file) = request.target_store.roots.resolve(descriptor) {
        target.insert(file.full_path().to_path_buf());
    }
}

fn excluded_domain(
    descriptor: &DomainDescriptor,
    exclusion: StorageTransitionExclusion,
) -> StorageTransitionDomain {
    StorageTransitionDomain {
        domain: descriptor.id().clone(),
        storage_class: descriptor.storage_class(),
        source_path: None,
        target_path: None,
        source_evidence: None,
        target_evidence: None,
        action: StorageTransitionAction::Excluded(exclusion),
    }
}

fn default_exclusion(
    class: StorageClass,
    include_logs: bool,
) -> Option<StorageTransitionExclusion> {
    match class {
        StorageClass::Cache => Some(StorageTransitionExclusion::CacheRebuilt),
        StorageClass::Runtime => Some(StorageTransitionExclusion::RuntimeDiscarded),
        StorageClass::Secret => Some(StorageTransitionExclusion::SecretExternal),
        StorageClass::Log if !include_logs => Some(StorageTransitionExclusion::LogsNotSelected),
        StorageClass::Defaults | StorageClass::Policy => {
            Some(StorageTransitionExclusion::ExternalAuthority)
        }
        _ => None,
    }
}
