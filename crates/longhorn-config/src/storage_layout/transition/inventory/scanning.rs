use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{RootKind, Sha256Digest};

use super::super::{StorageFileEvidence, StorageTransitionError, StorageTransitionUnknownFile};

pub(crate) fn read_evidence(
    path: &Path,
    limits: super::super::StorageTransitionLimits,
    total: &mut usize,
) -> Result<StorageFileEvidence, StorageTransitionError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StorageFileEvidence::Absent);
        }
        Err(error) => return Err(fs_error(path, error)),
    };
    if !metadata.file_type().is_file() {
        return Err(StorageTransitionError::Filesystem {
            path: path.to_path_buf(),
            detail: "transition evidence requires a regular file".into(),
        });
    }
    let length =
        usize::try_from(metadata.len()).map_err(|_| StorageTransitionError::BoundExceeded {
            path: path.to_path_buf(),
        })?;
    if length > limits.max_file_bytes() {
        return Err(StorageTransitionError::BoundExceeded {
            path: path.to_path_buf(),
        });
    }
    *total = total
        .checked_add(length)
        .filter(|value| *value <= limits.max_total_bytes())
        .ok_or_else(|| StorageTransitionError::BoundExceeded {
            path: path.to_path_buf(),
        })?;
    let bytes = fs::read(path).map_err(|error| fs_error(path, error))?;
    if bytes.len() != length {
        return Err(StorageTransitionError::Filesystem {
            path: path.to_path_buf(),
            detail: "file changed while inventory was read".into(),
        });
    }
    Ok(StorageFileEvidence::Present {
        byte_length: bytes.len(),
        sha256: Sha256Digest::from_bytes(&bytes),
    })
}

pub(super) fn scan_layout(
    layout: &crate::ResolvedStorageLayout,
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
                    evidence: read_evidence(&path, limits, total)?,
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
