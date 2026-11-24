use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::Sha256Digest;
use longhorn_storage_layout::RootKind;

use super::super::{StorageFileEvidence, StorageTransitionError, StorageTransitionUnknownFile};

pub(crate) fn read_domain_evidence(
    file: &crate::ResolvedFile,
    limits: super::super::StorageTransitionLimits,
    total: &mut usize,
) -> Result<StorageFileEvidence, StorageTransitionError> {
    read_evidence(
        file.root(),
        file.relative_path().as_path(),
        limits
            .max_file_bytes()
            .min(crate::store::load::max_live_domain_bytes()),
        limits.max_total_bytes(),
        total,
    )
}

fn read_evidence(
    root: &Path,
    relative: &Path,
    max_file_bytes: usize,
    max_total_bytes: usize,
    total: &mut usize,
) -> Result<StorageFileEvidence, StorageTransitionError> {
    let bound = max_file_bytes.min(max_total_bytes - *total);
    let Some(bytes) = super::super::io::read(root, relative, bound)? else {
        return Ok(StorageFileEvidence::Absent);
    };
    *total += bytes.len();
    Ok(StorageFileEvidence::Present {
        byte_length: bytes.len(),
        sha256: Sha256Digest::from_bytes(&bytes),
    })
}

pub(crate) fn scan_layout(
    layout: &longhorn_storage_layout::ResolvedStorageLayout,
    known: &BTreeSet<PathBuf>,
    limits: super::super::StorageTransitionLimits,
    total: &mut usize,
    kinds: &[RootKind],
    skipped_roots: &BTreeSet<PathBuf>,
) -> Result<Vec<StorageTransitionUnknownFile>, StorageTransitionError> {
    let mut roots = BTreeMap::new();
    for kind in kinds {
        if let Some(root) = layout.root(*kind)
            && !skipped_roots.contains(root.path())
        {
            roots.entry(root.path().to_path_buf()).or_insert(*kind);
        }
    }
    let mut unknown = Vec::new();
    for (root, kind) in roots {
        scan_directory(&root, kind, known, limits, total, &mut unknown)?;
    }
    unknown.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(unknown)
}

pub(crate) fn scan_directory(
    root: &Path,
    kind: RootKind,
    known: &BTreeSet<PathBuf>,
    limits: super::super::StorageTransitionLimits,
    total: &mut usize,
    unknown: &mut Vec<StorageTransitionUnknownFile>,
) -> Result<(), StorageTransitionError> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(fs_error(&directory, error)),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| fs_error(&directory, error))
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        for path in paths.into_iter().rev() {
            if path
                .components()
                .any(|component| component.as_os_str() == ".longhorn")
            {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).map_err(|error| fs_error(&path, error))?;
            if metadata.file_type().is_dir() {
                stack.push(path);
            } else if metadata.file_type().is_file() && !known.contains(&path) {
                if unknown.len() >= limits.max_unknown_files() {
                    return Err(StorageTransitionError::BoundExceeded { path });
                }
                unknown.push(StorageTransitionUnknownFile {
                    root: kind,
                    evidence: read_evidence(
                        root,
                        path.strip_prefix(root)
                            .expect("inventory entries descend from root"),
                        limits.max_file_bytes(),
                        limits.max_total_bytes(),
                        total,
                    )?,
                    path,
                });
            } else if !metadata.file_type().is_file() {
                return Err(StorageTransitionError::Filesystem {
                    path,
                    detail: "links and special files are not transition inventory".into(),
                });
            }
        }
    }
    Ok(())
}

fn fs_error(path: &Path, error: std::io::Error) -> StorageTransitionError {
    StorageTransitionError::Filesystem {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}
