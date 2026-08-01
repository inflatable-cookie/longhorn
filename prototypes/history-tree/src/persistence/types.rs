use std::fmt;

use longhorn_core::{HistoryEntryId, HistoryId};
use longhorn_history::HistoryPayloadCodecVersion;

use crate::ForkHistory;

/// Successful private graph compatibility outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkLoadOutcome {
    /// Current structural and payload versions were preserved.
    Preserved,
    /// One or both independent version families migrated.
    Migrated {
        /// Structural migration ran.
        structural: bool,
        /// Payload migration ran.
        payload: bool,
    },
}

/// Successful private graph load receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkLoadReceipt {
    pub(super) outcome: ForkLoadOutcome,
    pub(super) source_structural_version: u32,
    pub(super) source_payload_version: HistoryPayloadCodecVersion,
}

impl ForkLoadReceipt {
    /// Returns whether source bytes were preserved or migrated.
    #[must_use]
    pub const fn outcome(self) -> ForkLoadOutcome {
        self.outcome
    }
}

/// A fully validated graph and its visible load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkLoadResult<P> {
    pub(super) history: ForkHistory<P>,
    pub(super) receipt: ForkLoadReceipt,
}

impl<P> ForkLoadResult<P> {
    /// Returns the validated graph.
    #[must_use]
    pub const fn history(&self) -> &ForkHistory<P> {
        &self.history
    }

    /// Returns the compatibility receipt.
    #[must_use]
    pub const fn receipt(&self) -> ForkLoadReceipt {
        self.receipt
    }

    /// Consumes the result.
    #[must_use]
    pub fn into_parts(self) -> (ForkHistory<P>, ForkLoadReceipt) {
        (self.history, self.receipt)
    }
}

/// Failed private graph encoding.
#[derive(Debug)]
pub enum ForkEncodeError<E> {
    /// Consumer payload encoding failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: E,
    },
    /// Encoded bytes disagreed with retained exact weight.
    PayloadWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Retained weight.
        recorded: u64,
        /// Codec bytes.
        actual: u64,
    },
    /// One size conversion overflowed.
    SizeOverflow,
    /// Structural JSON encoding failed.
    Structural(serde_json::Error),
    /// Encoded bytes exceeded the private ceiling.
    EnvelopeTooLarge {
        /// Maximum bytes.
        maximum: usize,
        /// Actual bytes.
        actual: usize,
    },
}

/// Failed checked load. No graph is returned or replaced.
#[derive(Debug)]
pub enum ForkLoadError<CE, ME> {
    /// Source exceeded the private byte ceiling.
    EnvelopeTooLarge {
        /// Maximum bytes.
        maximum: usize,
        /// Actual bytes.
        actual: usize,
    },
    /// Source was not JSON.
    InvalidJson(serde_json::Error),
    /// Minimum header was malformed.
    InvalidHeader(serde_json::Error),
    /// Source used another format family.
    ForeignFormatFamily {
        /// Supplied family.
        actual: String,
    },
    /// Source structural version is newer.
    FutureStructuralVersion {
        /// Supplied version.
        actual: u32,
        /// Maximum supported version.
        maximum: u32,
    },
    /// No older structural step was registered.
    MissingStructuralMigration {
        /// Unsupported source version.
        from: u32,
    },
    /// A structural migration skipped or mis-stamped a version.
    InvalidStructuralMigration {
        /// Source version.
        from: u32,
        /// Produced version.
        produced: u32,
    },
    /// Structural migration hook failed.
    StructuralMigration(ME),
    /// Current envelope was not strict.
    InvalidEnvelope(serde_json::Error),
    /// Source belongs to another authority.
    ForeignHistory {
        /// Expected authority.
        expected: HistoryId,
        /// Supplied authority.
        actual: HistoryId,
    },
    /// Source used another payload family.
    ForeignPayloadCodecFamily,
    /// Source payload version is newer.
    FuturePayloadCodecVersion {
        /// Supplied version.
        actual: HistoryPayloadCodecVersion,
        /// Maximum supported version.
        maximum: HistoryPayloadCodecVersion,
    },
    /// Payload codec or migration failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Codec failure.
        error: CE,
    },
    /// Payload migration was unavailable.
    MissingPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Unsupported source version.
        from: HistoryPayloadCodecVersion,
    },
    /// Payload migration skipped a version.
    InvalidPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Source version.
        from: HistoryPayloadCodecVersion,
        /// Produced version.
        produced: HistoryPayloadCodecVersion,
    },
    /// Graph invariants failed.
    Validation(ForkPersistenceValidationError),
}

impl<CE, ME> fmt::Display for ForkLoadError<CE, ME> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("private fork history load failed")
    }
}

/// Invalid persisted private graph topology or bounded metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkPersistenceValidationError {
    /// Duplicate node identity.
    DuplicateNode(HistoryEntryId),
    /// Duplicate branch identity.
    DuplicateBranch,
    /// Duplicate checkpoint identity.
    DuplicateCheckpoint,
    /// Duplicate preferred parent.
    DuplicatePreference,
    /// Entry sequence was zero or duplicated.
    InvalidSequence,
    /// Next sequence did not exceed every retained sequence.
    InvalidNextSequence,
    /// A parent did not exist or did not precede its child.
    InvalidParent,
    /// A committed node revision exceeded the graph revision.
    InvalidCommittedRevision,
    /// A reference pointed at an absent node.
    UnknownNodeReference,
    /// Current branch was absent or did not contain current node.
    InvalidCurrentPosition,
    /// Preferred child was not a direct child.
    InvalidPreferredChild,
    /// Entry label was invalid.
    InvalidLabel,
    /// Branch metadata was invalid.
    InvalidBranchMetadata,
    /// Checkpoint reference was invalid.
    InvalidCheckpointReference,
    /// Encoded payload bytes disagreed with declared weight.
    PayloadWeightMismatch,
    /// Retained payload weight overflowed.
    WeightOverflow,
}
