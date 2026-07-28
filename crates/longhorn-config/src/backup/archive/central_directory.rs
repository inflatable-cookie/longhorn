use std::collections::BTreeSet;

use super::{BackupArchiveError, BackupArchiveLimits};

const EOCD_BYTES: usize = 22;
const CENTRAL_HEADER_BYTES: usize = 46;

pub(super) fn preflight_central_directory(
    bytes: &[u8],
    limits: BackupArchiveLimits,
) -> Result<(), BackupArchiveError> {
    let search_start = bytes.len().saturating_sub(EOCD_BYTES + u16::MAX as usize);
    let Some(eocd) = (search_start..=bytes.len().saturating_sub(EOCD_BYTES))
        .rev()
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(b"PK\x05\x06"))
    else {
        return Ok(());
    };
    let comment_length = read_u16(bytes, eocd + 20)? as usize;
    if eocd + EOCD_BYTES + comment_length != bytes.len() {
        return Err(zip_error("end-of-central-directory length is inconsistent"));
    }
    let disk = read_u16(bytes, eocd + 4)?;
    let central_disk = read_u16(bytes, eocd + 6)?;
    let disk_entries = read_u16(bytes, eocd + 8)?;
    let entries = read_u16(bytes, eocd + 10)?;
    let central_size = read_u32(bytes, eocd + 12)? as usize;
    let central_offset = read_u32(bytes, eocd + 16)? as usize;
    if disk != 0
        || central_disk != 0
        || disk_entries != entries
        || entries == u16::MAX
        || central_size == u32::MAX as usize
        || central_offset == u32::MAX as usize
    {
        return Err(zip_error("multi-disk and ZIP64 archives are unsupported"));
    }
    if usize::from(entries) > limits.max_entries() {
        return Err(BackupArchiveError::TooManyEntries {
            limit: limits.max_entries(),
            observed: usize::from(entries),
        });
    }
    let central_end = central_offset
        .checked_add(central_size)
        .ok_or_else(|| zip_error("central-directory range overflow"))?;
    if central_end != eocd || central_end > bytes.len() {
        return Err(zip_error("central-directory range is outside the archive"));
    }

    let mut cursor = central_offset;
    let mut seen = BTreeSet::new();
    for _ in 0..entries {
        if cursor + CENTRAL_HEADER_BYTES > central_end
            || bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02")
        {
            return Err(zip_error("invalid central-directory entry"));
        }
        let flags = read_u16(bytes, cursor + 8)?;
        let method = read_u16(bytes, cursor + 10)?;
        let name_length = read_u16(bytes, cursor + 28)? as usize;
        let extra_length = read_u16(bytes, cursor + 30)? as usize;
        let entry_comment_length = read_u16(bytes, cursor + 32)? as usize;
        let header_end = cursor
            .checked_add(CENTRAL_HEADER_BYTES)
            .and_then(|value| value.checked_add(name_length))
            .and_then(|value| value.checked_add(extra_length))
            .and_then(|value| value.checked_add(entry_comment_length))
            .ok_or_else(|| zip_error("central-directory entry length overflow"))?;
        if header_end > central_end {
            return Err(zip_error(
                "central-directory entry exceeds its declared range",
            ));
        }
        let raw_name =
            &bytes[cursor + CENTRAL_HEADER_BYTES..cursor + CENTRAL_HEADER_BYTES + name_length];
        let path = std::str::from_utf8(raw_name)
            .map_err(|error| BackupArchiveError::InvalidEntryName {
                path: String::from_utf8_lossy(raw_name).into_owned(),
                detail: error.to_string(),
            })?
            .to_owned();
        if !seen.insert(path.clone()) {
            return Err(BackupArchiveError::DuplicateEntry { path });
        }
        if flags & 1 != 0 {
            return Err(BackupArchiveError::EncryptedEntry { path });
        }
        if !matches!(method, 0 | 8) {
            return Err(BackupArchiveError::UnsupportedCompression {
                path,
                method: method.to_string(),
            });
        }
        cursor = header_end;
    }
    if cursor != central_end {
        return Err(zip_error("central-directory entry count is inconsistent"));
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, BackupArchiveError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| zip_error("truncated ZIP metadata"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, BackupArchiveError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| zip_error("truncated ZIP metadata"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn zip_error(detail: impl Into<String>) -> BackupArchiveError {
    BackupArchiveError::Zip {
        detail: detail.into(),
    }
}
