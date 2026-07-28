use std::{error::Error, fmt};

use crate::{BackupManifest, BackupPayloadPath, Sha256Digest};

const DEFAULT_MAX_ARCHIVE_BYTES: usize = 300 * 1024 * 1024;
const DEFAULT_MAX_ENTRIES: usize = 4_096;
const DEFAULT_MAX_PATH_BYTES: usize = 512;
const DEFAULT_MAX_ENTRY_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
const DEFAULT_MAX_COMPRESSION_RATIO: u32 = 200;
const HARD_MAX_ARCHIVE_BYTES: usize = 512 * 1024 * 1024;
const HARD_MAX_ENTRIES: usize = 16_384;
const HARD_MAX_PATH_BYTES: usize = 1_024;
const HARD_MAX_ENTRY_BYTES: usize = 256 * 1024 * 1024;
const HARD_MAX_TOTAL_BYTES: usize = 512 * 1024 * 1024;
const HARD_MAX_COMPRESSION_RATIO: u32 = 10_000;

/// Finite ZIP encoding and inspection bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackupArchiveLimits {
    max_archive_bytes: usize,
    max_entries: usize,
    max_path_bytes: usize,
    max_entry_bytes: usize,
    max_total_bytes: usize,
    max_compression_ratio: u32,
}

impl BackupArchiveLimits {
    /// Constructs limits below the library hard ceilings.
    pub fn new(
        max_archive_bytes: usize,
        max_entries: usize,
        max_path_bytes: usize,
        max_entry_bytes: usize,
        max_total_bytes: usize,
        max_compression_ratio: u32,
    ) -> Result<Self, BackupArchiveLimitsError> {
        if [
            max_archive_bytes,
            max_entries,
            max_path_bytes,
            max_entry_bytes,
            max_total_bytes,
        ]
        .contains(&0)
            || max_compression_ratio == 0
        {
            return Err(BackupArchiveLimitsError::Zero);
        }
        if max_entry_bytes > max_total_bytes {
            return Err(BackupArchiveLimitsError::EntryExceedsTotal);
        }
        if max_archive_bytes > HARD_MAX_ARCHIVE_BYTES
            || max_entries > HARD_MAX_ENTRIES
            || max_path_bytes > HARD_MAX_PATH_BYTES
            || max_entry_bytes > HARD_MAX_ENTRY_BYTES
            || max_total_bytes > HARD_MAX_TOTAL_BYTES
            || max_compression_ratio > HARD_MAX_COMPRESSION_RATIO
        {
            return Err(BackupArchiveLimitsError::HardCeiling);
        }
        Ok(Self {
            max_archive_bytes,
            max_entries,
            max_path_bytes,
            max_entry_bytes,
            max_total_bytes,
            max_compression_ratio,
        })
    }

    #[must_use]
    /// Returns the complete encoded archive byte bound.
    pub const fn max_archive_bytes(self) -> usize {
        self.max_archive_bytes
    }

    #[must_use]
    /// Returns the maximum ZIP entry count.
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }

    #[must_use]
    /// Returns the maximum UTF-8 path byte length.
    pub const fn max_path_bytes(self) -> usize {
        self.max_path_bytes
    }

    #[must_use]
    /// Returns the maximum uncompressed bytes for one entry.
    pub const fn max_entry_bytes(self) -> usize {
        self.max_entry_bytes
    }

    #[must_use]
    /// Returns the maximum aggregate uncompressed entry bytes.
    pub const fn max_total_bytes(self) -> usize {
        self.max_total_bytes
    }

    #[must_use]
    /// Returns the maximum uncompressed-to-compressed ratio.
    pub const fn max_compression_ratio(self) -> u32 {
        self.max_compression_ratio
    }
}

impl Default for BackupArchiveLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: DEFAULT_MAX_ARCHIVE_BYTES,
            max_entries: DEFAULT_MAX_ENTRIES,
            max_path_bytes: DEFAULT_MAX_PATH_BYTES,
            max_entry_bytes: DEFAULT_MAX_ENTRY_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_compression_ratio: DEFAULT_MAX_COMPRESSION_RATIO,
        }
    }
}

/// Invalid archive safety bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupArchiveLimitsError {
    /// At least one configured bound is zero.
    Zero,
    /// Per-entry byte limit exceeds the aggregate limit.
    EntryExceedsTotal,
    /// At least one configured bound exceeds the library hard ceiling.
    HardCeiling,
}

impl fmt::Display for BackupArchiveLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("backup archive limits must be non-zero"),
            Self::EntryExceedsTotal => {
                formatter.write_str("backup archive entry limit cannot exceed total limit")
            }
            Self::HardCeiling => formatter.write_str("backup archive limits exceed hard ceilings"),
        }
    }
}

impl Error for BackupArchiveLimitsError {}

/// Fully encoded plaintext Longhorn backup archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedBackupArchive {
    bytes: Vec<u8>,
    sha256: Sha256Digest,
}

impl EncodedBackupArchive {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        let sha256 = Sha256Digest::from_bytes(&bytes);
        Self { bytes, sha256 }
    }

    #[must_use]
    /// Returns exact ZIP bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    /// Returns SHA-256 over the complete ZIP bytes.
    pub fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }
}

/// Verified payload extracted only into private memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectedBackupPayload {
    path: BackupPayloadPath,
    bytes: Vec<u8>,
}

impl InspectedBackupPayload {
    pub(crate) fn new(path: BackupPayloadPath, bytes: Vec<u8>) -> Self {
        Self { path, bytes }
    }

    #[must_use]
    /// Returns the validated manifest path.
    pub fn path(&self) -> &BackupPayloadPath {
        &self.path
    }

    #[must_use]
    /// Returns exact verified uncompressed payload bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Integrity state after complete manifest and payload verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupIntegrity {
    /// Every declared length and SHA-256 matched.
    Verified,
}

/// Authentication state for the plaintext card-006 archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupAuthenticity {
    /// Integrity is verified but no authenticated envelope exists.
    UnauthenticatedPlaintext,
}

/// Side-effect-free verified plaintext archive inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupArchiveInspection {
    manifest: BackupManifest,
    payloads: Vec<InspectedBackupPayload>,
    archive_sha256: Sha256Digest,
    integrity: BackupIntegrity,
    authenticity: BackupAuthenticity,
}

impl BackupArchiveInspection {
    pub(crate) fn new(
        manifest: BackupManifest,
        payloads: Vec<InspectedBackupPayload>,
        archive_sha256: Sha256Digest,
    ) -> Self {
        Self {
            manifest,
            payloads,
            archive_sha256,
            integrity: BackupIntegrity::Verified,
            authenticity: BackupAuthenticity::UnauthenticatedPlaintext,
        }
    }

    #[must_use]
    /// Returns the strict verified manifest.
    pub fn manifest(&self) -> &BackupManifest {
        &self.manifest
    }

    #[must_use]
    /// Returns verified payloads in manifest path order.
    pub fn payloads(&self) -> &[InspectedBackupPayload] {
        &self.payloads
    }

    #[must_use]
    /// Returns SHA-256 over the complete source archive.
    pub fn archive_sha256(&self) -> &Sha256Digest {
        &self.archive_sha256
    }

    #[must_use]
    /// Returns payload integrity state.
    pub const fn integrity(&self) -> BackupIntegrity {
        self.integrity
    }

    #[must_use]
    /// Returns plaintext authentication state.
    pub const fn authenticity(&self) -> BackupAuthenticity {
        self.authenticity
    }
}
