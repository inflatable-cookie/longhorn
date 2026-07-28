use std::io::{Seek, Write};

use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{BackupSnapshot, Sha256Digest};

use super::{
    BackupArchiveError, BackupArchiveLimits, DEFLATE_LEVEL, EncodedBackupArchive, MANIFEST_PATH,
    bounded_cursor::BoundedCursor, manifest::validate_manifest,
};

/// Encodes an immutable captured snapshot into the canonical ZIP bundle.
pub fn encode_backup_archive(
    snapshot: &BackupSnapshot,
    limits: BackupArchiveLimits,
) -> Result<EncodedBackupArchive, BackupArchiveError> {
    let manifest_bytes =
        serde_json::to_vec(snapshot.manifest()).map_err(|error| BackupArchiveError::Encoding {
            detail: error.to_string(),
        })?;
    let payloads = validate_snapshot(snapshot)?;
    validate_encoding_bounds(&manifest_bytes, &payloads, limits)?;

    let cursor = BoundedCursor::new(limits.max_archive_bytes());
    let mut writer = ZipWriter::new(cursor);
    write_entry(&mut writer, MANIFEST_PATH, &manifest_bytes)?;
    for payload in payloads {
        write_entry(&mut writer, payload.path().as_str(), payload.bytes())?;
    }
    let cursor = writer.finish().map_err(zip_encoding_error)?;
    Ok(EncodedBackupArchive::new(cursor.into_inner()))
}

fn write_entry<W: Write + Seek>(
    writer: &mut ZipWriter<W>,
    path: &str,
    bytes: &[u8],
) -> Result<(), BackupArchiveError> {
    let compression = if bytes.is_empty() {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };
    let level = (compression == CompressionMethod::Deflated).then_some(DEFLATE_LEVEL);
    let options = SimpleFileOptions::default()
        .compression_method(compression)
        .compression_level(level)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o600)
        .large_file(false);
    writer
        .start_file(path, options)
        .map_err(zip_encoding_error)?;
    writer
        .write_all(bytes)
        .map_err(|error| BackupArchiveError::Encoding {
            detail: error.to_string(),
        })
}

fn validate_snapshot(
    snapshot: &BackupSnapshot,
) -> Result<Vec<&crate::BackupSnapshotPayload>, BackupArchiveError> {
    let declared = validate_manifest(snapshot.manifest())?;
    if snapshot.payloads().len() != declared.len() {
        return Err(snapshot_invariant(
            "payload count does not match manifest declaration",
        ));
    }
    let mut payloads = snapshot.payloads().iter().collect::<Vec<_>>();
    payloads.sort_by(|left, right| left.path().cmp(right.path()));
    for payload in &payloads {
        let Some(expected) = declared.get(payload.path().as_str()) else {
            return Err(snapshot_invariant(format!(
                "payload {} is undeclared",
                payload.path().as_str()
            )));
        };
        if payload.bytes().len() as u64 != expected.byte_length {
            return Err(snapshot_invariant(format!(
                "payload {} length differs from manifest",
                payload.path().as_str()
            )));
        }
        if Sha256Digest::from_bytes(payload.bytes()) != expected.sha256 {
            return Err(snapshot_invariant(format!(
                "payload {} digest differs from manifest",
                payload.path().as_str()
            )));
        }
    }
    Ok(payloads)
}

fn validate_encoding_bounds(
    manifest: &[u8],
    payloads: &[&crate::BackupSnapshotPayload],
    limits: BackupArchiveLimits,
) -> Result<(), BackupArchiveError> {
    let entry_count = payloads.len() + 1;
    if entry_count > limits.max_entries() {
        return Err(BackupArchiveError::TooManyEntries {
            limit: limits.max_entries(),
            observed: entry_count,
        });
    }
    let mut total = checked_encoding_entry(MANIFEST_PATH, manifest.len(), 0, limits)?;
    for payload in payloads {
        total = checked_encoding_entry(
            payload.path().as_str(),
            payload.bytes().len(),
            total,
            limits,
        )?;
    }
    Ok(())
}

fn checked_encoding_entry(
    path: &str,
    bytes: usize,
    current_total: usize,
    limits: BackupArchiveLimits,
) -> Result<usize, BackupArchiveError> {
    if path.len() > limits.max_path_bytes() {
        return Err(BackupArchiveError::PathTooLong {
            path: path.into(),
            limit: limits.max_path_bytes(),
        });
    }
    if bytes > limits.max_entry_bytes() {
        return Err(BackupArchiveError::EntryTooLarge {
            path: path.into(),
            limit: limits.max_entry_bytes(),
            observed: bytes as u64,
        });
    }
    let observed = current_total
        .checked_add(bytes)
        .ok_or(BackupArchiveError::TotalTooLarge {
            limit: limits.max_total_bytes(),
            observed: u64::MAX,
        })?;
    if observed > limits.max_total_bytes() {
        Err(BackupArchiveError::TotalTooLarge {
            limit: limits.max_total_bytes(),
            observed: observed as u64,
        })
    } else {
        Ok(observed)
    }
}

fn snapshot_invariant(detail: impl Into<String>) -> BackupArchiveError {
    BackupArchiveError::SnapshotInvariant {
        detail: detail.into(),
    }
}

fn zip_encoding_error(error: zip::result::ZipError) -> BackupArchiveError {
    BackupArchiveError::Encoding {
        detail: error.to_string(),
    }
}
