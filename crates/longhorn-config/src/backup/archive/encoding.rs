use std::io::{Seek, Write};

use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{
    BackupArchiveInspection, BackupKind, BackupManifest, BackupPayloadPath, BackupSnapshot,
    Sha256Digest,
};

use super::{
    BackupArchiveError, BackupArchiveLimits, DEFLATE_LEVEL, EncodedBackupArchive, MANIFEST_PATH,
    bounded_cursor::BoundedCursor, codec::ratio_exceeds, manifest::validate_manifest,
};

/// Encodes an immutable captured snapshot into the canonical ZIP bundle.
pub fn encode_backup_archive(
    snapshot: &BackupSnapshot,
    limits: BackupArchiveLimits,
) -> Result<EncodedBackupArchive, BackupArchiveError> {
    let payloads = validate_payloads(
        snapshot.manifest(),
        snapshot.payloads().iter().map(|payload| ArchivePayload {
            path: payload.path(),
            bytes: payload.bytes(),
        }),
    )?;
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
    let payloads = validate_payloads(
        inspection.manifest(),
        inspection.payloads().iter().map(|payload| ArchivePayload {
            path: payload.path(),
            bytes: payload.bytes(),
        }),
    )?;
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

    let entries = || {
        std::iter::once((MANIFEST_PATH, manifest_bytes.as_slice())).chain(
            payloads
                .iter()
                .map(|payload| (payload.path.as_str(), payload.bytes)),
        )
    };
    let write_zip = |stored: &[bool]| {
        let mut writer = ZipWriter::new(BoundedCursor::new(limits.max_archive_bytes()));
        for (index, (path, bytes)) in entries().enumerate() {
            let compression = if bytes.is_empty() || stored.get(index) == Some(&true) {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            };
            write_entry(&mut writer, path, bytes, compression)?;
        }
        writer.finish_into_readable().map_err(zip_encoding_error)
    };

    let compressed = write_zip(&[])?;
    let stored = (0..compressed.len())
        .map(|index| {
            let entry = compressed
                .by_index_data(index)
                .map_err(zip_encoding_error)?;
            Ok(ratio_exceeds(
                entry.size(),
                entry.compressed_size(),
                limits.max_compression_ratio(),
            ))
        })
        .collect::<Result<Vec<_>, BackupArchiveError>>()?;
    let archive = if stored.contains(&true) {
        drop(compressed);
        write_zip(&stored)?
    } else {
        compressed
    };
    Ok(EncodedBackupArchive::new(archive.into_inner().into_inner()))
}

fn write_entry<W: Write + Seek>(
    writer: &mut ZipWriter<W>,
    path: &str,
    bytes: &[u8],
    compression: CompressionMethod,
) -> Result<(), BackupArchiveError> {
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

fn validate_payloads<'a>(
    manifest: &BackupManifest,
    payloads: impl Iterator<Item = ArchivePayload<'a>>,
) -> Result<Vec<ArchivePayload<'a>>, BackupArchiveError> {
    let mut declared = validate_manifest(manifest)?;
    let mut payloads = payloads.collect::<Vec<_>>();
    if payloads.len() != declared.len() {
        return Err(snapshot_invariant(
            "payload count does not match manifest declaration",
        ));
    }
    payloads.sort_by_key(|payload| payload.path);
    for payload in &payloads {
        let path = payload.path.as_str();
        let Some(expected) = declared.remove(path) else {
            return Err(snapshot_invariant(format!(
                "payload {path} is duplicate or undeclared"
            )));
        };
        if payload.bytes.len() as u64 != expected.byte_length {
            return Err(snapshot_invariant(format!(
                "payload {path} length differs from manifest"
            )));
        }
        if Sha256Digest::from_bytes(payload.bytes) != expected.sha256 {
            return Err(snapshot_invariant(format!(
                "payload {path} digest differs from manifest"
            )));
        }
    }
    Ok(payloads)
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
        let source = encode_archive(
            &manifest,
            &[ArchivePayload {
                path: &BackupPayloadPath::new(path).unwrap(),
                bytes: payload,
            }],
            BackupArchiveLimits::default(),
        )
        .unwrap();
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

    #[test]
    fn repetitive_payload_is_stored_and_the_archive_passes_the_same_limits() {
        let payload = vec![b'x'; 512 * 1024];
        let path = "longhorn/adapters/example.settings/document.bin";
        let manifest = serde_json::from_value::<BackupManifest>(json!({
            "format": "longhorn.config-backup",
            "formatVersion": 1,
            "archiveId": "repetitive",
            "kind": "pre-restore",
            "createdAt": "2026-08-02T12:00:00Z",
            "application": {"id": "com.example.app", "version": "1.0.0"},
            "producer": {"name": "longhorn-config", "version": "0.1.0"},
            "consistencyGroups": [{"id": "example.settings", "mode": "external-snapshot", "authority": "settings-test"}],
            "domains": [{
                "domain": "example.settings",
                "storageClass": "user-config",
                "consistencyGroup": "example.settings",
                "adapter": "settings-test",
                "state": "present",
                "sourceSchemaVersion": 1,
                "sourceIssue": null,
                "payloads": [{"path": path, "byteLength": payload.len(), "sha256": Sha256Digest::from_bytes(&payload).as_str()}]
            }],
            "exclusions": []
        })).unwrap();
        let limits = BackupArchiveLimits::default();
        let archive = encode_archive(
            &manifest,
            &[ArchivePayload {
                path: &BackupPayloadPath::new(path).unwrap(),
                bytes: &payload,
            }],
            limits,
        )
        .unwrap();

        let inspected = inspect_backup_archive(archive.bytes(), limits).unwrap();
        assert_eq!(inspected.payloads()[0].bytes(), payload);
        let zip = zip::ZipArchive::new(std::io::Cursor::new(archive.bytes())).unwrap();
        assert_eq!(
            zip.by_index_data(0).unwrap().compression(),
            CompressionMethod::Deflated
        );
        assert_eq!(
            zip.by_index_data(1).unwrap().compression(),
            CompressionMethod::Stored
        );
    }
}
