use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

use serde_json::Value;
use zip::{CompressionMethod, DateTime, ZipArchive};

use crate::{BackupManifest, BackupPayloadPath, Sha256Digest};

use super::{
    BackupArchiveError, BackupArchiveInspection, BackupArchiveLimits, InspectedBackupPayload,
    MANIFEST_PATH,
    central_directory::preflight_central_directory,
    manifest::{DeclaredPayload, validate_manifest},
};

/// General-purpose bit 11: the entry name is UTF-8.
const LANGUAGE_ENCODING_FLAG: u16 = 1 << 11;

/// Parses and verifies a plaintext backup without extracting to the filesystem.
pub fn inspect_backup_archive(
    bytes: &[u8],
    limits: BackupArchiveLimits,
) -> Result<BackupArchiveInspection, BackupArchiveError> {
    if bytes.len() > limits.max_archive_bytes() {
        return Err(BackupArchiveError::ArchiveTooLarge {
            limit: limits.max_archive_bytes(),
            observed: bytes.len(),
        });
    }
    preflight_central_directory(bytes, limits)?;
    let archive_sha256 = Sha256Digest::from_bytes(bytes);
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(zip_read_error)?;
    if !archive.comment().is_empty() {
        return Err(BackupArchiveError::ArchiveComment);
    }
    if archive.len() > limits.max_entries() {
        return Err(BackupArchiveError::TooManyEntries {
            limit: limits.max_entries(),
            observed: archive.len(),
        });
    }
    if archive.is_empty() {
        return Err(BackupArchiveError::ManifestNotFirst);
    }

    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(archive.len());
    let mut total = 0_u64;
    let mut previous = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_read_error)?;
        let path = validate_entry_metadata(&entry, index, previous.as_deref(), limits)?;
        if !seen.insert(path.clone()) {
            return Err(BackupArchiveError::DuplicateEntry { path });
        }
        if index > 0 {
            previous = Some(path.clone());
        }
        let declared_size = entry.size();
        let compressed_size = entry.compressed_size();
        let entry_bytes = read_entry_bounded(
            entry.by_ref(),
            path.clone(),
            declared_size,
            compressed_size,
            &mut total,
            limits,
        )?;
        entries.push((path, entry_bytes));
    }

    if entries[0].0 != MANIFEST_PATH {
        return Err(BackupArchiveError::ManifestNotFirst);
    }
    let manifest = parse_manifest(&entries[0].1)?;
    let declared = validate_manifest(&manifest)?;
    verify_inventory(&entries[1..], &declared)?;
    let payloads = entries
        .into_iter()
        .skip(1)
        .map(|(path, bytes)| {
            BackupPayloadPath::new(path.clone())
                .map(|path| InspectedBackupPayload::new(path, bytes))
                .map_err(|error| BackupArchiveError::InvalidEntryName {
                    path,
                    detail: error.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(BackupArchiveInspection::new(
        manifest,
        payloads,
        archive_sha256,
    ))
}

fn validate_entry_metadata<R: Read>(
    entry: &zip::read::ZipFile<'_, R>,
    index: usize,
    previous: Option<&str>,
    limits: BackupArchiveLimits,
) -> Result<String, BackupArchiveError> {
    let raw_name = std::str::from_utf8(entry.name_raw()).map_err(|error| {
        BackupArchiveError::InvalidEntryName {
            path: String::from_utf8_lossy(entry.name_raw()).into_owned(),
            detail: error.to_string(),
        }
    })?;
    // A non-ASCII name without the UTF-8 flag decodes as CP437 elsewhere.
    if !raw_name.is_ascii() && entry.flags().as_u16() & LANGUAGE_ENCODING_FLAG == 0 {
        return Err(BackupArchiveError::InvalidEntryName {
            path: raw_name.into(),
            detail: "name is not canonical UTF-8".into(),
        });
    }
    let path = raw_name.to_owned();
    if path.len() > limits.max_path_bytes() {
        return Err(BackupArchiveError::PathTooLong {
            path,
            limit: limits.max_path_bytes(),
        });
    }
    if index == 0 {
        if path != MANIFEST_PATH {
            return Err(BackupArchiveError::ManifestNotFirst);
        }
    } else {
        BackupPayloadPath::new(path.clone()).map_err(|error| {
            BackupArchiveError::InvalidEntryName {
                path: path.clone(),
                detail: error.to_string(),
            }
        })?;
        if let Some(previous) = previous
            && previous >= path.as_str()
        {
            return Err(BackupArchiveError::EntryOrder {
                previous: previous.into(),
                current: path,
            });
        }
    }
    if entry.encrypted() {
        return Err(BackupArchiveError::EncryptedEntry { path });
    }
    if !entry.is_file() {
        return Err(BackupArchiveError::NonRegularEntry { path });
    }
    if !matches!(
        entry.compression(),
        CompressionMethod::Stored | CompressionMethod::Deflated
    ) {
        return Err(BackupArchiveError::UnsupportedCompression {
            path,
            method: entry.compression().to_string(),
        });
    }
    if entry.size() > limits.max_entry_bytes() as u64 {
        return Err(BackupArchiveError::EntryTooLarge {
            path,
            limit: limits.max_entry_bytes(),
            observed: entry.size(),
        });
    }
    if ratio_exceeds(
        entry.size(),
        entry.compressed_size(),
        limits.max_compression_ratio(),
    ) {
        return Err(BackupArchiveError::CompressionRatio {
            path,
            limit: limits.max_compression_ratio(),
            compressed: entry.compressed_size(),
            uncompressed: entry.size(),
        });
    }
    if entry.last_modified() != Some(DateTime::default()) {
        return Err(noncanonical(path, "timestamp is not 1980-01-01T00:00:00"));
    }
    let mode = entry
        .unix_mode()
        .ok_or_else(|| noncanonical(path.clone(), "Unix mode is absent"))?;
    if mode & 0o170_000 != 0o100_000 || mode & 0o777 != 0o600 {
        return Err(noncanonical(path, "Unix mode is not regular 0600"));
    }
    if !entry.comment().is_empty() {
        return Err(noncanonical(path, "entry comment is present"));
    }
    if entry.extra_data().is_some_and(|extra| !extra.is_empty()) {
        return Err(noncanonical(path, "extra fields are present"));
    }
    Ok(path)
}

fn parse_manifest(bytes: &[u8]) -> Result<BackupManifest, BackupArchiveError> {
    let value = serde_json::from_slice::<Value>(bytes).map_err(|error| {
        BackupArchiveError::ManifestJson {
            detail: error.to_string(),
        }
    })?;
    let format = value
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if format != "longhorn.config-backup" {
        return Err(BackupArchiveError::UnsupportedFormat {
            found: format.into(),
        });
    }
    let version = value
        .get("formatVersion")
        .map_or_else(|| "<missing>".into(), Value::to_string);
    if value.get("formatVersion").and_then(Value::as_u64) != Some(1) {
        return Err(BackupArchiveError::UnsupportedFormatVersion { found: version });
    }
    serde_json::from_value(value).map_err(|error| BackupArchiveError::ManifestJson {
        detail: error.to_string(),
    })
}

fn verify_inventory(
    entries: &[(String, Vec<u8>)],
    declared: &BTreeMap<String, DeclaredPayload>,
) -> Result<(), BackupArchiveError> {
    for (path, bytes) in entries {
        let Some(expected) = declared.get(path) else {
            return Err(BackupArchiveError::UndeclaredEntry { path: path.clone() });
        };
        if bytes.len() as u64 != expected.byte_length {
            return Err(BackupArchiveError::LengthMismatch {
                path: path.clone(),
                expected: expected.byte_length,
                observed: bytes.len() as u64,
            });
        }
        if Sha256Digest::from_bytes(bytes) != expected.sha256 {
            return Err(BackupArchiveError::ChecksumMismatch { path: path.clone() });
        }
    }
    for (path, expected) in declared {
        if !entries.iter().any(|(entry, _)| entry == path) {
            return Err(BackupArchiveError::MissingEntry {
                path: expected.path.as_str().into(),
            });
        }
    }
    Ok(())
}

fn read_entry_bounded<R: Read>(
    reader: R,
    path: String,
    declared_size: u64,
    compressed_size: u64,
    total: &mut u64,
    limits: BackupArchiveLimits,
) -> Result<Vec<u8>, BackupArchiveError> {
    if total
        .checked_add(declared_size)
        .is_none_or(|declared_total| declared_total > limits.max_total_bytes() as u64)
    {
        return Err(BackupArchiveError::TotalTooLarge {
            limit: limits.max_total_bytes(),
            observed: total.saturating_add(declared_size),
        });
    }
    let remaining_total = (limits.max_total_bytes() as u64).saturating_sub(*total);
    let max_by_ratio = compressed_size.saturating_mul(u64::from(limits.max_compression_ratio()));
    let read_limit = (limits.max_entry_bytes() as u64)
        .min(remaining_total)
        .min(max_by_ratio);
    let initial_capacity = usize::try_from(declared_size.min(read_limit)).unwrap_or(0);
    let mut entry_bytes = Vec::with_capacity(initial_capacity);
    reader
        .take(read_limit.saturating_add(1))
        .read_to_end(&mut entry_bytes)
        .map_err(|error| BackupArchiveError::Read {
            path: path.clone(),
            detail: error.to_string(),
        })?;
    let actual_len = entry_bytes.len() as u64;
    if entry_bytes.len() > limits.max_entry_bytes() {
        return Err(BackupArchiveError::EntryTooLarge {
            path,
            limit: limits.max_entry_bytes(),
            observed: actual_len,
        });
    }
    *total = total
        .checked_add(actual_len)
        .ok_or(BackupArchiveError::TotalTooLarge {
            limit: limits.max_total_bytes(),
            observed: u64::MAX,
        })?;
    if *total > limits.max_total_bytes() as u64 {
        return Err(BackupArchiveError::TotalTooLarge {
            limit: limits.max_total_bytes(),
            observed: *total,
        });
    }
    if ratio_exceeds(actual_len, compressed_size, limits.max_compression_ratio()) {
        return Err(BackupArchiveError::CompressionRatio {
            path,
            limit: limits.max_compression_ratio(),
            compressed: compressed_size,
            uncompressed: actual_len,
        });
    }
    Ok(entry_bytes)
}

pub(super) fn ratio_exceeds(uncompressed: u64, compressed: u64, limit: u32) -> bool {
    if uncompressed == 0 {
        return false;
    }
    compressed == 0 || uncompressed.div_ceil(compressed) > u64::from(limit)
}

fn noncanonical(path: String, detail: impl Into<String>) -> BackupArchiveError {
    BackupArchiveError::NonCanonicalMetadata {
        path,
        detail: detail.into(),
    }
}

fn zip_read_error(error: zip::result::ZipError) -> BackupArchiveError {
    match error {
        zip::result::ZipError::CompressionMethodNotSupported(method) => {
            BackupArchiveError::UnsupportedCompression {
                path: "<archive>".into(),
                method: method.to_string(),
            }
        }
        error => BackupArchiveError::Zip {
            detail: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_math_is_checked_and_zero_safe() {
        assert!(!ratio_exceeds(0, 0, 1));
        assert!(ratio_exceeds(1, 0, 1));
        assert!(!ratio_exceeds(200, 1, 200));
        assert!(ratio_exceeds(201, 1, 200));
        assert!(!ratio_exceeds(u64::MAX, u64::MAX, u32::MAX));
    }

    struct CountingZeroReader {
        remaining: usize,
        bytes_read: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl Read for CountingZeroReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let count = buf.len().min(self.remaining);
            buf[..count].fill(0);
            self.remaining -= count;
            self.bytes_read.set(self.bytes_read.get() + count);
            Ok(count)
        }
    }

    #[test]
    fn read_entry_bounded_stops_at_total_ratio_and_entry_limits_with_understated_header() {
        let counter = || std::rc::Rc::new(std::cell::Cell::new(0_usize));

        // 1. Aggregate total stopping point: declared=4 passes preflight,
        //    remaining_total=1096 stops reading at 1097 bytes (never 1 MiB).
        let read_count = counter();
        let reader = CountingZeroReader {
            remaining: 1024 * 1024,
            bytes_read: std::rc::Rc::clone(&read_count),
        };
        let mut total = 3000_u64;
        let limits =
            BackupArchiveLimits::new(1 << 20, 8, 256, 4096, 4096, 200).expect("total limits");
        let err = read_entry_bounded(
            reader,
            "longhorn/domains/a.json".into(),
            4,
            4096,
            &mut total,
            limits,
        )
        .unwrap_err();
        assert_eq!(
            err,
            BackupArchiveError::TotalTooLarge {
                limit: 4096,
                observed: 4097,
            }
        );
        assert_eq!(read_count.get(), 1097);

        // 2. Actual ratio stopping point: declared=4 vs compressed=10 (ratio 0.4 <= 10)
        //    passes metadata, but actual bytes stop at 10*10 + 1 = 101 bytes.
        let read_count = counter();
        let reader = CountingZeroReader {
            remaining: 1024 * 1024,
            bytes_read: std::rc::Rc::clone(&read_count),
        };
        let mut total = 0_u64;
        let limits =
            BackupArchiveLimits::new(1 << 20, 8, 256, 4096, 8192, 10).expect("ratio limits");
        let err = read_entry_bounded(
            reader,
            "longhorn/domains/b.json".into(),
            4,
            10,
            &mut total,
            limits,
        )
        .unwrap_err();
        assert_eq!(
            err,
            BackupArchiveError::CompressionRatio {
                path: "longhorn/domains/b.json".into(),
                limit: 10,
                compressed: 10,
                uncompressed: 101,
            }
        );
        assert_eq!(read_count.get(), 101);

        // 3. Per-entry stopping point: declared=4 passes metadata, actual bytes
        //    stop at max_entry_bytes + 1 = 257 bytes.
        let read_count = counter();
        let reader = CountingZeroReader {
            remaining: 1024 * 1024,
            bytes_read: std::rc::Rc::clone(&read_count),
        };
        let mut total = 0_u64;
        let limits =
            BackupArchiveLimits::new(1 << 20, 8, 256, 256, 4096, 200).expect("entry limits");
        let err = read_entry_bounded(
            reader,
            "longhorn/domains/c.json".into(),
            4,
            256,
            &mut total,
            limits,
        )
        .unwrap_err();
        assert_eq!(
            err,
            BackupArchiveError::EntryTooLarge {
                path: "longhorn/domains/c.json".into(),
                limit: 256,
                observed: 257,
            }
        );
        assert_eq!(read_count.get(), 257);
    }
}
