use std::io::{Seek, Write};

use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{
    BackupArchiveInspection, BackupKind, BackupManifest, BackupPayloadPath, BackupSnapshot,
    Sha256Digest,
};

use super::{
    BackupArchiveError, BackupArchiveLimits, DEFLATE_LEVEL, EncodedBackupArchive, MANIFEST_PATH,
    bounded_cursor::BoundedCursor, manifest::validate_manifest,
};

/// Encodes an immutable captured snapshot into the canonical ZIP bundle.
pub fn encode_backup_archive(
    snapshot: &BackupSnapshot,
    limits: BackupArchiveLimits,
) -> Result<EncodedBackupArchive, BackupArchiveError> {
    let payloads = validate_snapshot(snapshot)?;
    encode_archive(snapshot.manifest(), &payloads, limits)
}

/// Re-encodes one verified archive as a canonical user export.
///
/// The immutable snapshot identity and payload evidence are preserved. Only
/// the manifest purpose changes to [`BackupKind::UserExport`].
pub fn encode_backup_export_archive(
    inspection: &BackupArchiveInspection,
    limits: BackupArchiveLimits,
) -> Result<EncodedBackupArchive, BackupArchiveError> {
    let manifest = inspection.manifest().with_kind(BackupKind::UserExport);
    let payloads = validate_inspection(inspection)?;
    encode_archive(&manifest, &payloads, limits)
}

#[derive(Clone, Copy)]
struct ArchivePayload<'a> {
    path: &'a BackupPayloadPath,
    bytes: &'a [u8],
}

fn encode_archive(
    manifest: &BackupManifest,
    payloads: &[ArchivePayload<'_>],
    limits: BackupArchiveLimits,
) -> Result<EncodedBackupArchive, BackupArchiveError> {
    let manifest_bytes =
        serde_json::to_vec(manifest).map_err(|error| BackupArchiveError::Encoding {
            detail: error.to_string(),
        })?;
    validate_encoding_bounds(&manifest_bytes, payloads, limits)?;

    let cursor = BoundedCursor::new(limits.max_archive_bytes());
    let mut writer = ZipWriter::new(cursor);
    write_entry(&mut writer, MANIFEST_PATH, &manifest_bytes)?;
    for payload in payloads {
        write_entry(&mut writer, payload.path.as_str(), payload.bytes)?;
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
) -> Result<Vec<ArchivePayload<'_>>, BackupArchiveError> {
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
    Ok(payloads
        .into_iter()
        .map(|payload| ArchivePayload {
            path: payload.path(),
            bytes: payload.bytes(),
        })
        .collect())
}

fn validate_inspection(
    inspection: &BackupArchiveInspection,
) -> Result<Vec<ArchivePayload<'_>>, BackupArchiveError> {
    let declared = validate_manifest(inspection.manifest())?;
    if inspection.payloads().len() != declared.len() {
        return Err(snapshot_invariant(
            "inspected payload count does not match manifest declaration",
        ));
    }
    let mut payloads = inspection.payloads().iter().collect::<Vec<_>>();
    payloads.sort_by(|left, right| left.path().cmp(right.path()));
    for payload in &payloads {
        let Some(expected) = declared.get(payload.path().as_str()) else {
            return Err(snapshot_invariant(format!(
                "inspected payload {} is undeclared",
                payload.path().as_str()
            )));
        };
        if payload.bytes().len() as u64 != expected.byte_length {
            return Err(snapshot_invariant(format!(
                "inspected payload {} length differs from manifest",
                payload.path().as_str()
            )));
        }
        if Sha256Digest::from_bytes(payload.bytes()) != expected.sha256 {
            return Err(snapshot_invariant(format!(
                "inspected payload {} digest differs from manifest",
                payload.path().as_str()
            )));
        }
    }
    Ok(payloads
        .into_iter()
        .map(|payload| ArchivePayload {
            path: payload.path(),
            bytes: payload.bytes(),
        })
        .collect())
}

fn validate_encoding_bounds(
    manifest: &[u8],
    payloads: &[ArchivePayload<'_>],
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
        total = checked_encoding_entry(payload.path.as_str(), payload.bytes.len(), total, limits)?;
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;

    use super::*;
    use crate::inspect_backup_archive;

    #[test]
    fn verified_operational_archive_reencodes_as_the_same_user_export_snapshot() {
        let payload = b"exact settings bytes";
        let path = "longhorn/adapters/nucleus.settings/document.bin";
        let manifest = serde_json::from_value::<BackupManifest>(json!({
            "format": "longhorn.config-backup",
            "formatVersion": 1,
            "archiveId": "operational-42",
            "kind": "operational",
            "createdAt": "2026-08-02T12:00:00Z",
            "application": {"id": "com.example.app", "version": "1.0.0"},
            "producer": {"name": "longhorn-config", "version": "0.1.0"},
            "consistencyGroups": [{
                "id": "nucleus.settings",
                "mode": "external-snapshot",
                "authority": "settings-test"
            }],
            "domains": [{
                "domain": "nucleus.settings",
                "storageClass": "user-config",
                "consistencyGroup": "nucleus.settings",
                "adapter": "settings-test",
                "state": "present",
                "sourceSchemaVersion": 1,
                "sourceIssue": null,
                "payloads": [{
                    "path": path,
                    "byteLength": payload.len(),
                    "sha256": Sha256Digest::from_bytes(payload).as_str()
                }]
            }],
            "exclusions": []
        }))
        .unwrap();
        let source = encode_parts_for_test(&manifest, path, payload);
        let source_inspection =
            inspect_backup_archive(source.bytes(), BackupArchiveLimits::default()).unwrap();

        let export =
            encode_backup_export_archive(&source_inspection, BackupArchiveLimits::default())
                .unwrap();
        let exported =
            inspect_backup_archive(export.bytes(), BackupArchiveLimits::default()).unwrap();

        assert_eq!(source_inspection.manifest().kind(), BackupKind::Operational);
        assert_eq!(exported.manifest().kind(), BackupKind::UserExport);
        assert_eq!(exported.manifest().archive_id(), "operational-42");
        assert_eq!(
            exported.manifest().created_at(),
            source_inspection.manifest().created_at()
        );
        assert_eq!(
            exported.manifest().domains(),
            source_inspection.manifest().domains()
        );
        assert_eq!(exported.payloads(), source_inspection.payloads());
        assert_ne!(export.sha256(), source.sha256());
    }

    fn encode_parts_for_test(
        manifest: &BackupManifest,
        path: &str,
        payload: &[u8],
    ) -> EncodedBackupArchive {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        write_entry(
            &mut writer,
            MANIFEST_PATH,
            &serde_json::to_vec(manifest).unwrap(),
        )
        .unwrap();
        write_entry(&mut writer, path, payload).unwrap();
        EncodedBackupArchive::new(writer.finish().unwrap().into_inner())
    }
}
