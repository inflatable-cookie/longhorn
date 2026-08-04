use std::collections::{BTreeMap, BTreeSet};

use crate::{ConfigStore, DomainLocation, RootKind, StorageClass};

use super::{
    LegacyStorageCandidate, LegacyStorageDiscovery, StorageFileEvidence, StorageTransitionAction,
    StorageTransitionDomain, StorageTransitionError, StorageTransitionExclusion,
    StorageTransitionLimits, inventory,
};

const LEGACY_ROOTS: [RootKind; 7] = [
    RootKind::Config,
    RootKind::Data,
    RootKind::State,
    RootKind::Workspace,
    RootKind::Cache,
    RootKind::Log,
    RootKind::Runtime,
];

/// Inspects only caller-declared candidates. It performs no writes or deletes.
pub fn discover_legacy_storage(
    registry: &ConfigStore,
    candidates: &[LegacyStorageCandidate],
    limits: StorageTransitionLimits,
) -> Result<Vec<LegacyStorageDiscovery>, StorageTransitionError> {
    let mut discoveries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut domains = Vec::new();
        let mut known = BTreeSet::new();
        let mut total = 0usize;
        for descriptor in registry.registered_descriptors() {
            let exclusion = match descriptor.storage_class() {
                StorageClass::Cache => Some(StorageTransitionExclusion::CacheRebuilt),
                StorageClass::Runtime => Some(StorageTransitionExclusion::RuntimeDiscarded),
                StorageClass::Secret => Some(StorageTransitionExclusion::SecretExternal),
                StorageClass::Defaults | StorageClass::Policy => {
                    Some(StorageTransitionExclusion::ExternalAuthority)
                }
                _ => None,
            };
            if let Some(exclusion) = exclusion {
                domains.push(StorageTransitionDomain {
                    domain: descriptor.id().clone(),
                    storage_class: descriptor.storage_class(),
                    source_path: None,
                    target_path: None,
                    source_evidence: None,
                    target_evidence: None,
                    action: StorageTransitionAction::Excluded(exclusion),
                });
                continue;
            }
            let DomainLocation::File(file) = candidate.roots().resolve(descriptor) else {
                continue;
            };
            known.insert(file.full_path().to_path_buf());
            let evidence = inventory::read_evidence(file.full_path(), limits, &mut total)?;
            let action = if evidence == StorageFileEvidence::Absent {
                StorageTransitionAction::Absent
            } else {
                StorageTransitionAction::CopyOrdinary
            };
            domains.push(StorageTransitionDomain {
                domain: descriptor.id().clone(),
                storage_class: descriptor.storage_class(),
                source_path: Some(file.full_path().to_path_buf()),
                target_path: None,
                source_evidence: Some(evidence),
                target_evidence: None,
                action,
            });
        }
        domains.sort_by(|left, right| left.domain.cmp(&right.domain));
        let mut roots = BTreeMap::new();
        for kind in LEGACY_ROOTS {
            for descriptor in registry.registered_descriptors() {
                if let DomainLocation::File(file) = candidate.roots().resolve(descriptor)
                    && file.root_kind() == kind
                {
                    roots.entry(file.root().to_path_buf()).or_insert(kind);
                }
            }
        }
        let mut unknown_files = Vec::new();
        for (root, kind) in roots {
            inventory::scan_directory(&root, kind, &known, limits, &mut total, &mut unknown_files)?;
        }
        unknown_files.sort_by(|left, right| left.path.cmp(&right.path));
        discoveries.push(LegacyStorageDiscovery {
            candidate_id: candidate.id().to_owned(),
            domains,
            unknown_files,
        });
    }
    Ok(discoveries)
}
