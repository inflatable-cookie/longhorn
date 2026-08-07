use std::{error::Error, fmt};

use longhorn_core::{CompatibilityStore, FutureSchemaRefusal, FutureSchemaRefused};

/// Failure to encode or safely inspect a plaintext backup archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupArchiveError {
    /// Complete encoded input exceeds its byte bound.
    ArchiveTooLarge {
        /// Configured byte bound.
        limit: usize,
        /// Observed bytes.
        observed: usize,
    },
    /// ZIP central directory declares too many entries.
    TooManyEntries {
        /// Configured entry bound.
        limit: usize,
        /// Observed entries.
        observed: usize,
    },
    /// Entry path exceeds its portable byte bound.
    PathTooLong {
        /// Rejected path.
        path: String,
        /// Configured path bound.
        limit: usize,
    },
    /// One uncompressed entry exceeds its byte bound.
    EntryTooLarge {
        /// Rejected entry path.
        path: String,
        /// Configured entry bound.
        limit: usize,
        /// Observed uncompressed bytes.
        observed: u64,
    },
    /// Aggregate uncompressed entries exceed their byte bound.
    TotalTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Observed aggregate bytes.
        observed: u64,
    },
    /// Entry expansion exceeds the configured compression ratio.
    CompressionRatio {
        /// Rejected entry path.
        path: String,
        /// Configured maximum ratio.
        limit: u32,
        /// Declared compressed bytes.
        compressed: u64,
        /// Declared uncompressed bytes.
        uncompressed: u64,
    },
    /// ZIP container parsing failed.
    Zip {
        /// Parser detail.
        detail: String,
    },
    /// Deterministic ZIP encoding failed.
    Encoding {
        /// Encoder detail.
        detail: String,
    },
    /// Strict manifest JSON parsing failed.
    ManifestJson {
        /// Parser detail.
        detail: String,
    },
    /// Manifest names an unsupported format.
    UnsupportedFormat {
        /// Observed format value.
        found: String,
    },
    /// Manifest names an unsupported format version.
    UnsupportedFormatVersion {
        /// Observed version value.
        found: String,
    },
    /// Required manifest entry is absent from position zero.
    ManifestNotFirst,
    /// ZIP-level comment is present.
    ArchiveComment,
    /// Entry path is unsafe or non-canonical.
    InvalidEntryName {
        /// Rejected path.
        path: String,
        /// Validation detail.
        detail: String,
    },
    /// ZIP contains the same entry path more than once.
    DuplicateEntry {
        /// Duplicate path.
        path: String,
    },
    /// Payload entries are not in lexicographic order.
    EntryOrder {
        /// Previous path.
        previous: String,
        /// Out-of-order path.
        current: String,
    },
    /// Entry uses neither Stored nor DEFLATE.
    UnsupportedCompression {
        /// Rejected entry path.
        path: String,
        /// Observed compression method.
        method: String,
    },
    /// ZIP entry uses forbidden per-entry encryption.
    EncryptedEntry {
        /// Rejected entry path.
        path: String,
    },
    /// Entry is a directory, link, device, or other non-file type.
    NonRegularEntry {
        /// Rejected entry path.
        path: String,
    },
    /// Entry timestamp, mode, comment, or extra data is non-canonical.
    NonCanonicalMetadata {
        /// Rejected entry path.
        path: String,
        /// Metadata detail.
        detail: String,
    },
    /// Bounded entry decompression or read failed.
    Read {
        /// Affected entry path.
        path: String,
        /// Reader detail.
        detail: String,
    },
    /// Manifest relationships or source evidence are inconsistent.
    ManifestInvariant {
        /// Invariant detail.
        detail: String,
    },
    /// Immutable snapshot differs from its own manifest.
    SnapshotInvariant {
        /// Invariant detail.
        detail: String,
    },
    /// ZIP contains a payload absent from the manifest.
    UndeclaredEntry {
        /// Undeclared path.
        path: String,
    },
    /// Manifest declares a payload absent from the ZIP.
    MissingEntry {
        /// Missing path.
        path: String,
    },
    /// Payload length differs from the manifest.
    LengthMismatch {
        /// Affected payload path.
        path: String,
        /// Manifest byte length.
        expected: u64,
        /// Observed byte length.
        observed: u64,
    },
    /// Payload SHA-256 differs from the manifest.
    ChecksumMismatch {
        /// Affected payload path.
        path: String,
    },
}

impl fmt::Display for BackupArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveTooLarge { limit, observed } => {
                write!(formatter, "archive has {observed} bytes; limit is {limit}")
            }
            Self::TooManyEntries { limit, observed } => {
                write!(
                    formatter,
                    "archive has {observed} entries; limit is {limit}"
                )
            }
            Self::PathTooLong { path, limit } => {
                write!(formatter, "archive path {path} exceeds {limit} bytes")
            }
            Self::EntryTooLarge {
                path,
                limit,
                observed,
            } => write!(
                formatter,
                "archive entry {path} has {observed} bytes; limit is {limit}"
            ),
            Self::TotalTooLarge { limit, observed } => {
                write!(
                    formatter,
                    "archive entries total {observed} bytes; limit is {limit}"
                )
            }
            Self::CompressionRatio {
                path,
                limit,
                compressed,
                uncompressed,
            } => write!(
                formatter,
                "archive entry {path} ratio {uncompressed}:{compressed} exceeds {limit}:1"
            ),
            Self::Zip { detail } => write!(formatter, "invalid ZIP: {detail}"),
            Self::Encoding { detail } => write!(formatter, "archive encoding failed: {detail}"),
            Self::ManifestJson { detail } => write!(formatter, "invalid manifest JSON: {detail}"),
            Self::UnsupportedFormat { found } => {
                write!(formatter, "unsupported backup format {found}")
            }
            Self::UnsupportedFormatVersion { found } => {
                write!(formatter, "unsupported backup format version {found}")
            }
            Self::ManifestNotFirst => formatter.write_str("manifest must be the first ZIP entry"),
            Self::ArchiveComment => formatter.write_str("archive comments are forbidden"),
            Self::InvalidEntryName { path, detail } => {
                write!(formatter, "invalid archive entry name {path}: {detail}")
            }
            Self::DuplicateEntry { path } => write!(formatter, "duplicate archive entry {path}"),
            Self::EntryOrder { previous, current } => {
                write!(formatter, "archive entry {current} sorts before {previous}")
            }
            Self::UnsupportedCompression { path, method } => {
                write!(
                    formatter,
                    "archive entry {path} uses unsupported compression {method}"
                )
            }
            Self::EncryptedEntry { path } => {
                write!(
                    formatter,
                    "archive entry {path} uses forbidden ZIP encryption"
                )
            }
            Self::NonRegularEntry { path } => {
                write!(formatter, "archive entry {path} is not a regular file")
            }
            Self::NonCanonicalMetadata { path, detail } => {
                write!(
                    formatter,
                    "archive entry {path} has non-canonical metadata: {detail}"
                )
            }
            Self::Read { path, detail } => {
                write!(formatter, "cannot read archive entry {path}: {detail}")
            }
            Self::ManifestInvariant { detail } => {
                write!(formatter, "invalid backup manifest: {detail}")
            }
            Self::SnapshotInvariant { detail } => {
                write!(formatter, "invalid backup snapshot: {detail}")
            }
            Self::UndeclaredEntry { path } => write!(formatter, "undeclared archive entry {path}"),
            Self::MissingEntry { path } => write!(formatter, "missing archive entry {path}"),
            Self::LengthMismatch {
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "archive entry {path} length is {observed}; manifest declares {expected}"
            ),
            Self::ChecksumMismatch { path } => {
                write!(formatter, "archive entry {path} SHA-256 does not match")
            }
        }
    }
}

impl Error for BackupArchiveError {}

impl FutureSchemaRefused for BackupArchiveError {
    /// The archive reports the version it found as text, because a
    /// non-numeric value is itself one of the rejections. Only a value that
    /// parses can be reported as a version.
    fn future_schema_refusal(&self) -> Option<FutureSchemaRefusal> {
        match self {
            Self::UnsupportedFormatVersion { found } => Some(FutureSchemaRefusal {
                store: CompatibilityStore::BackupArchive,
                found: found.parse().ok(),
                supported: Some(crate::backup::types::BACKUP_FORMAT_VERSION),
            }),
            _ => None,
        }
    }
}
