use std::{collections::BTreeSet, path::PathBuf};

use crate::storage_layout::types::root_kind_id;

use super::{
    MIGRATING_ROOTS, StorageTransitionConflict, StorageTransitionConflictKind,
    StorageTransitionRequest,
};

pub(super) fn root_conflicts(
    request: &StorageTransitionRequest<'_>,
) -> Vec<StorageTransitionConflict> {
    let mut conflicts = Vec::new();
    for source_kind in MIGRATING_ROOTS {
        let Some(source) = request.source_layout.root(source_kind) else {
            continue;
        };
        for target_kind in MIGRATING_ROOTS {
            let Some(target) = request.target_layout.root(target_kind) else {
                continue;
            };
            let exact_same = source.path() == target.path();
            let overlaps = source.path().starts_with(target.path())
                || target.path().starts_with(source.path());
            if overlaps && !(exact_same && source_kind == target_kind) {
                conflicts.push(StorageTransitionConflict {
                    kind: StorageTransitionConflictKind::OverlappingRoots,
                    domain: None,
                    path: Some(target.path().to_path_buf()),
                    detail: format!(
                        "{} source overlaps {} target",
                        root_kind_id(source_kind),
                        root_kind_id(target_kind)
                    ),
                });
            }
        }
    }
    conflicts
}

pub(super) fn equal_root_paths(request: &StorageTransitionRequest<'_>) -> BTreeSet<PathBuf> {
    MIGRATING_ROOTS
        .iter()
        .filter_map(|kind| {
            let source = request.source_layout.root(*kind)?;
            let target = request.target_layout.root(*kind)?;
            (source.path() == target.path()).then(|| target.path().to_path_buf())
        })
        .collect()
}
