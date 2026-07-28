use std::io::{Cursor, Write};

use longhorn_config::{
    BackupApplication, BackupArchiveLimits, BackupCatalog, BackupKind, BackupMetadata,
    BackupProducer, BackupPublicationOptions, BackupScope, DurabilityRequirement,
};
use serde_json::{Value, json};
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::common::{Fixture, config_domain, document};

pub(super) fn snapshot(
    archive_id: &str,
    created_at: &str,
    application_id: &str,
    kind: BackupKind,
) -> longhorn_config::BackupSnapshot {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": archive_id, "enabled": true}),
        ),
    );
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();
    store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            BackupMetadata::new(
                archive_id,
                kind,
                created_at,
                BackupApplication::new(application_id, "1.0.0").unwrap(),
                BackupProducer::new("longhorn-config", "0.1.0").unwrap(),
            )
            .unwrap(),
            super::super::options(longhorn_config::BackupLimits::default()),
        )
        .unwrap()
}

pub(super) fn archive_with(
    manifest: Value,
    entries: &[(&str, &[u8])],
    compression: CompressionMethod,
) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_entry(
        &mut writer,
        "longhorn/manifest.json",
        &serde_json::to_vec(&manifest).unwrap(),
        compression,
    );
    for (path, bytes) in entries {
        write_entry(&mut writer, path, bytes, compression);
    }
    writer.finish().unwrap().into_inner()
}

pub(super) fn archive_with_directory(manifest: Value) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_entry(
        &mut writer,
        "longhorn/manifest.json",
        &serde_json::to_vec(&manifest).unwrap(),
        CompressionMethod::Stored,
    );
    writer
        .add_directory(
            "longhorn/directory/",
            canonical_options(CompressionMethod::Stored),
        )
        .unwrap();
    writer.finish().unwrap().into_inner()
}

pub(super) fn archive_with_symlink(manifest: Value) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_entry(
        &mut writer,
        "longhorn/manifest.json",
        &serde_json::to_vec(&manifest).unwrap(),
        CompressionMethod::Stored,
    );
    writer
        .add_symlink(
            "longhorn/link",
            "target",
            canonical_options(CompressionMethod::Stored),
        )
        .unwrap();
    writer.finish().unwrap().into_inner()
}

pub(super) fn archive_with_device(manifest: Value) -> Vec<u8> {
    let mut archive = archive_with(
        manifest,
        &[("longhorn/device", b"device")],
        CompressionMethod::Stored,
    );
    let central = archive
        .windows(4)
        .rposition(|window| window == b"PK\x01\x02")
        .unwrap();
    let attributes = (0o020600_u32 << 16).to_le_bytes();
    archive[central + 38..central + 42].copy_from_slice(&attributes);
    archive
}

pub(super) fn patch_compression_method(bytes: &mut [u8], method: u16) {
    let mut cursor = 0;
    while cursor + 12 <= bytes.len() {
        let signature = &bytes[cursor..cursor + 4];
        if signature == b"PK\x03\x04" {
            bytes[cursor + 8..cursor + 10].copy_from_slice(&method.to_le_bytes());
        } else if signature == b"PK\x01\x02" {
            bytes[cursor + 10..cursor + 12].copy_from_slice(&method.to_le_bytes());
        }
        cursor += 1;
    }
}

pub(super) fn replace_all_same_length(bytes: &mut [u8], from: &[u8], to: &[u8]) {
    assert_eq!(from.len(), to.len());
    for offset in 0..=bytes.len() - from.len() {
        if &bytes[offset..offset + from.len()] == from {
            bytes[offset..offset + to.len()].copy_from_slice(to);
        }
    }
}

pub(super) fn publication_options() -> BackupPublicationOptions {
    BackupPublicationOptions::new(
        DurabilityRequirement::Durable,
        BackupArchiveLimits::default(),
    )
}

fn write_entry(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    path: &str,
    bytes: &[u8],
    compression: CompressionMethod,
) {
    let options = canonical_options(compression);
    writer.start_file(path, options).unwrap();
    writer.write_all(bytes).unwrap();
}

fn canonical_options(compression: CompressionMethod) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(compression)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o600)
        .large_file(false)
}
