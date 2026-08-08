//! Encode and load errors for fork-history persistence.

use std::{error::Error, fmt};

use longhorn_core::{
    CompatibilityStore, FutureSchemaRefusal, FutureSchemaRefused, HistoryEntryId, HistoryId,
};
use longhorn_history::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};
use crate::{ForkBranchId, ForkCheckpointId, ForkHistoryStateError};

#[derive(Debug)]
/// Failed graph encoding.
pub enum ForkEncodeError<E> {
    /// Consumer payload encoding failed.
    Payload {
        /// Entry whose payload failed.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: E,
    },
    /// Encoded bytes disagreed with retained exact weight.
    PayloadWeightMismatch {
        /// Entry whose retained measurement disagreed.
        entry_id: HistoryEntryId,
        /// Retained exact byte weight.
        recorded: u64,
        /// Encoded payload byte count.
        actual: u64,
    },
    /// One size conversion overflowed.
    SizeOverflow,
    /// Structural JSON encoding failed.
    Structural(serde_json::Error),
    /// Encoded bytes exceeded the hard ceiling.
    EnvelopeTooLarge {
        /// Configured ceiling.
        maximum: usize,
        /// Produced bytes.
        actual: usize,
    },
}

impl<E> fmt::Display for ForkEncodeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("fork history encode failed")
    }
}

impl<E: Error + fmt::Debug> Error for ForkEncodeError<E> {}

/// Failed checked load. No graph is returned or replaced.
#[derive(Debug)]
pub enum ForkLoadError<CE, ME> {
    /// Source bytes exceeded the configured ceiling.
    EnvelopeTooLarge {
        /// Configured ceiling.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// Source was not JSON.
    InvalidJson(serde_json::Error),
    /// The minimal version header was absent or invalid.
    InvalidHeader(serde_json::Error),
    /// The envelope belonged to another structural family.
    ForeignFormatFamily {
        /// Supplied format family.
        actual: String,
    },
    /// The envelope requires unsupported future structural meaning.
    FutureStructuralVersion {
        /// Supplied version.
        actual: u32,
        /// Current supported version.
        maximum: u32,
    },
    /// No exact next structural migration was registered.
    MissingStructuralMigration {
        /// Version requiring a next step.
        from: u32,
    },
    /// A structural hook skipped or misreported its next version.
    InvalidStructuralMigration {
        /// Version offered to the hook.
        from: u32,
        /// Version produced by the hook.
        produced: u32,
    },
    /// The registered structural migration failed.
    StructuralMigration(ME),
    /// The current complete envelope shape was invalid.
    InvalidEnvelope(serde_json::Error),
    /// The envelope belonged to another history authority.
    ForeignHistory {
        /// Required history identity.
        expected: HistoryId,
        /// Supplied history identity.
        actual: HistoryId,
    },
    /// The payloads belonged to another codec family.
    ForeignPayloadCodecFamily {
        /// Registered codec family.
        expected: HistoryPayloadCodecFamily,
        /// Supplied codec family.
        actual: HistoryPayloadCodecFamily,
    },
    /// Payloads require unsupported future codec meaning.
    FuturePayloadCodecVersion {
        /// Supplied version.
        actual: HistoryPayloadCodecVersion,
        /// Current supported version.
        maximum: HistoryPayloadCodecVersion,
    },
    /// Consumer payload decode or migration failed.
    Payload {
        /// Entry whose payload failed.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: CE,
    },
    /// No exact next payload migration was registered.
    MissingPayloadMigration {
        /// Entry whose payload requires migration.
        entry_id: HistoryEntryId,
        /// Version requiring a next step.
        from: HistoryPayloadCodecVersion,
    },
    /// A payload hook skipped or misreported its next version.
    InvalidPayloadMigration {
        /// Entry whose payload was being migrated.
        entry_id: HistoryEntryId,
        /// Version offered to the hook.
        from: HistoryPayloadCodecVersion,
        /// Version produced by the hook.
        produced: HistoryPayloadCodecVersion,
    },
    /// Encoded payload bytes disagreed with the retained exact weight.
    PayloadWeightMismatch {
        /// Entry whose retained measurement disagreed.
        entry_id: HistoryEntryId,
        /// Retained source byte weight.
        recorded: u64,
        /// Supplied source payload bytes.
        actual: u64,
    },
    /// A payload byte count could not fit the weight type.
    PayloadWeightOverflow,
    /// One node had a zero sequence.
    InvalidSequence(HistoryEntryId),
    /// The next sequence was zero.
    InvalidNextSequence,
    /// One node label violated shared label bounds.
    InvalidLabel(HistoryEntryId),
    /// One branch metadata record violated public bounds.
    InvalidBranchMetadata(ForkBranchId),
    /// One opaque checkpoint record violated public bounds.
    InvalidCheckpoint(ForkCheckpointId),
    /// Complete topology or authority state failed validation.
    Validation(ForkHistoryStateError),
}

impl<CE, ME> fmt::Display for ForkLoadError<CE, ME> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("fork history load failed")
    }
}

impl<CE: Error + fmt::Debug, ME: Error + fmt::Debug> Error for ForkLoadError<CE, ME> {}

impl<CE, ME> FutureSchemaRefused for ForkLoadError<CE, ME> {
    /// The fork tree versions its structural envelope and its payload codec
    /// independently, and either can be ahead on a channel rejoin.
    fn future_schema_refusal(&self) -> Option<FutureSchemaRefusal> {
        match self {
            Self::FutureStructuralVersion { actual, maximum } => Some(
                FutureSchemaRefusal::versioned(CompatibilityStore::HistoryTree, *actual, *maximum),
            ),
            Self::FuturePayloadCodecVersion { actual, maximum } => {
                Some(FutureSchemaRefusal::versioned(
                    CompatibilityStore::HistoryTree,
                    actual.get(),
                    maximum.get(),
                ))
            }
            _ => None,
        }
    }
}
