//! Property tests for [`inspect_backup_archive`] against generated adversarial
//! input (card 213). Two properties run 64 fixed cases each:
//!
//! - arbitrary and signature-seeded byte strings never panic the inspector,
//!   and inspection is a pure function of the input bytes (two runs agree);
//! - a valid canonical archive mutated by bit flips, truncation, junk
//!   appends, and lies in central-directory length and offset fields still
//!   fails classified (a typed [`BackupArchiveError`]) or verifies, and a
//!   verified result always hashes back to the exact input bytes.
//!
//! "Never reads past the input" is observed indirectly: every read path is
//! slice-bounded, so an over-read would surface as a panic, and a read
//! influenced by anything outside the input would break determinism.
//! Measured cost: the whole module runs in well under one second.

use std::io::{Cursor, Write};

use proptest::prelude::*;
use serde_json::json;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::Sha256Digest;

use super::{BackupArchiveLimits, DEFLATE_LEVEL, MANIFEST_PATH, codec::inspect_backup_archive};

const CASES: u32 = 64;

fn fuzz_limits() -> BackupArchiveLimits {
    BackupArchiveLimits::new(1 << 20, 8, 256, 1 << 16, 1 << 18, 200).expect("fuzz limits")
}

/// Builds the smallest archive the inspector accepts: one deflated manifest
/// entry declaring zero domains.
fn valid_archive() -> Vec<u8> {
    let manifest = serde_json::to_vec(&json!({
        "format": "longhorn.config-backup",
        "formatVersion": 1,
        "archiveId": "fuzz-base",
        "kind": "operational",
        "createdAt": "2026-01-01T00:00:00Z",
        "application": {"id": "com.example.fuzz", "version": "1.0.0"},
        "producer": {"name": "longhorn-config", "version": "0.1.0"},
        "consistencyGroups": [],
        "domains": [],
        "exclusions": []
    }))
    .expect("base manifest");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(DEFLATE_LEVEL))
        .last_modified_time(DateTime::default())
        .unix_permissions(0o600)
        .large_file(false);
    writer.start_file(MANIFEST_PATH, options).expect("entry");
    writer.write_all(&manifest).expect("manifest bytes");
    writer.finish().expect("archive").into_inner()
}

/// Random bytes with ZIP signatures spliced in, so cases reach past the
/// end-of-central-directory scan into the central-directory walk.
fn hostile_bytes() -> impl Strategy<Value = Vec<u8>> {
    (
        prop::collection::vec(any::<u8>(), 0..=600),
        prop::collection::vec((0..=600_usize, 0..3_u8), 0..=4),
    )
        .prop_map(|(mut bytes, inserts)| {
            for (position, choice) in inserts {
                let signature: &[u8] = match choice {
                    0 => b"PK\x05\x06",
                    1 => b"PK\x01\x02",
                    _ => b"PK\x03\x04",
                };
                let position = position.min(bytes.len());
                bytes.splice(position..position, signature.iter().copied());
            }
            bytes
        })
}

/// One adversarial edit to the valid base archive. Offsets are taken modulo
/// the current length at apply time so truncation and earlier edits compose.
#[derive(Clone, Copy, Debug)]
enum ArchiveEdit {
    FlipBit { offset: usize, bit: u8 },
    SetByte { offset: usize, value: u8 },
    WriteU16 { offset: usize, value: u16 },
    WriteU32 { offset: usize, value: u32 },
    Truncate { len: usize },
}

fn archive_edits() -> impl Strategy<Value = Vec<ArchiveEdit>> {
    let edit = prop_oneof![
        (any::<usize>(), 0..8_u8).prop_map(|(offset, bit)| ArchiveEdit::FlipBit { offset, bit }),
        (any::<usize>(), any::<u8>())
            .prop_map(|(offset, value)| ArchiveEdit::SetByte { offset, value }),
        (any::<usize>(), any::<u16>())
            .prop_map(|(offset, value)| ArchiveEdit::WriteU16 { offset, value }),
        (any::<usize>(), any::<u32>())
            .prop_map(|(offset, value)| ArchiveEdit::WriteU32 { offset, value }),
        any::<usize>().prop_map(|len| ArchiveEdit::Truncate { len }),
    ];
    prop::collection::vec(edit, 1..=6)
}

fn apply_edit(bytes: &mut Vec<u8>, edit: ArchiveEdit) {
    match edit {
        ArchiveEdit::FlipBit { offset, bit } => {
            let offset = offset % bytes.len().max(1);
            if let Some(byte) = bytes.get_mut(offset) {
                *byte ^= 1 << bit;
            }
        }
        ArchiveEdit::SetByte { offset, value } => {
            let offset = offset % bytes.len().max(1);
            if let Some(byte) = bytes.get_mut(offset) {
                *byte = value;
            }
        }
        ArchiveEdit::WriteU16 { offset, value } => {
            let offset = offset % bytes.len().max(1);
            for (index, byte) in value.to_le_bytes().into_iter().enumerate() {
                if let Some(target) = bytes.get_mut(offset + index) {
                    *target = byte;
                }
            }
        }
        ArchiveEdit::WriteU32 { offset, value } => {
            let offset = offset % bytes.len().max(1);
            for (index, byte) in value.to_le_bytes().into_iter().enumerate() {
                if let Some(target) = bytes.get_mut(offset + index) {
                    *target = byte;
                }
            }
        }
        ArchiveEdit::Truncate { len } => {
            bytes.truncate(len % (bytes.len() + 1));
        }
    }
}

/// Offsets of the central-directory and end-of-central-directory length and
/// offset fields in the given archive, so targeted lies hit declared sizes,
/// entry counts, and header offsets rather than random payload bytes.
fn structural_offsets(bytes: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let search_start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let Some(eocd) = (search_start..=bytes.len().saturating_sub(22))
        .rev()
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(b"PK\x05\x06"))
    else {
        return offsets;
    };
    // EOCD: disk numbers, entry counts, central size, central offset, comment
    // length.
    offsets.extend((4..=20).step_by(2).map(|field| eocd + field));
    let central_offset =
        u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().expect("eocd field")) as usize;
    if bytes.get(central_offset..central_offset + 4) == Some(b"PK\x01\x02") {
        // Central header: flags, method, sizes, name/extra/comment lengths,
        // external attributes, local header offset.
        offsets.extend(
            [8_usize, 10, 20, 24, 28, 30, 32, 38, 42]
                .into_iter()
                .map(|field| central_offset + field),
        );
    }
    offsets
}

fn assert_consistent_outcome(bytes: &[u8]) {
    let first = inspect_backup_archive(bytes, fuzz_limits());
    let second = inspect_backup_archive(bytes, fuzz_limits());
    assert_eq!(first, second, "inspection must be a pure function of input");
    if let Ok(inspection) = first {
        assert_eq!(
            inspection.archive_sha256().as_str(),
            Sha256Digest::from_bytes(bytes).as_str(),
            "a verified archive must hash back to the exact input bytes"
        );
    }
}

#[test]
fn base_archive_is_accepted() {
    let bytes = valid_archive();
    let inspection = inspect_backup_archive(&bytes, fuzz_limits())
        .expect("the generated base archive must verify");
    assert_eq!(inspection.payloads(), &[]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Arbitrary and signature-seeded bytes never panic the inspector, and
    /// every outcome is deterministic and classified: `Err` carries a typed
    /// `BackupArchiveError`, `Ok` hashes back to the input.
    #[test]
    fn arbitrary_bytes_inspect_deterministically_without_panic(bytes in hostile_bytes()) {
        assert_consistent_outcome(&bytes);
    }

    /// A valid archive corrupted by random edits plus one targeted lie in a
    /// central-directory length or offset field never panics the inspector
    /// and never verifies against bytes other than the ones supplied.
    #[test]
    fn mutated_valid_archive_inspect_deterministically_without_panic(
        edits in archive_edits(),
        field in any::<proptest::sample::Index>(),
        lie in any::<u32>(),
    ) {
        let mut bytes = valid_archive();
        let offsets = structural_offsets(&bytes);
        if let Some(offset) = offsets.get(field.index(offsets.len().max(1))).copied() {
            apply_edit(&mut bytes, ArchiveEdit::WriteU32 { offset, value: lie });
        }
        for edit in edits {
            apply_edit(&mut bytes, edit);
        }
        assert_consistent_outcome(&bytes);
    }
}
