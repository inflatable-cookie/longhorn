mod types;
mod validation;
mod wire;

use std::convert::Infallible;

use longhorn_core::HistoryId;
use longhorn_history::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};
use serde_json::Value;

use crate::ForkHistory;
pub use types::{
    ForkEncodeError, ForkLoadError, ForkLoadOutcome, ForkLoadReceipt, ForkLoadResult,
    ForkPersistenceValidationError,
};
use wire::{Branch, Checkpoint, Envelope, Node, PayloadCodec, PreferredChild};

/// Stable format family for private fork-tree evidence.
pub const FORK_HISTORY_FORMAT_FAMILY: &str = "longhorn.private.history-tree";
/// Current private graph envelope version.
pub const CURRENT_FORK_HISTORY_STRUCTURAL_VERSION: u32 = 1;
/// Defensive ceiling for one private graph envelope.
pub const MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES: usize = 1 << 30;

/// Current private payload migration authority.
#[derive(Clone, Copy, Debug)]
pub struct ForkPayloadMigrationTarget<'target> {
    family: &'target HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

impl<'target> ForkPayloadMigrationTarget<'target> {
    /// Returns the registered codec family.
    #[must_use]
    pub fn family(self) -> &'target str {
        self.family.as_str()
    }

    /// Returns the registered current version.
    #[must_use]
    pub const fn version(self) -> HistoryPayloadCodecVersion {
        self.version
    }
}

/// One exact private payload migration step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkPayloadMigrationStep {
    version: HistoryPayloadCodecVersion,
    bytes: Vec<u8>,
}

impl ForkPayloadMigrationStep {
    /// Constructs one exact next-version step.
    #[must_use]
    pub const fn new(version: HistoryPayloadCodecVersion, bytes: Vec<u8>) -> Self {
        Self { version, bytes }
    }
}

/// Consumer-owned payload codec for the private prototype.
pub trait ForkPayloadCodec<P> {
    /// Codec or migration failure.
    type Error;

    /// Returns the stable codec family.
    fn family(&self) -> &HistoryPayloadCodecFamily;

    /// Returns the current codec version.
    fn version(&self) -> HistoryPayloadCodecVersion;

    /// Encodes one current typed payload.
    fn encode(&self, payload: &P) -> Result<Vec<u8>, Self::Error>;

    /// Decodes one payload at the current version.
    fn decode(&self, bytes: &[u8]) -> Result<P, Self::Error>;

    /// Produces one exact next-version migration step.
    fn migrate_one(
        &self,
        _from: HistoryPayloadCodecVersion,
        _bytes: Vec<u8>,
        _target: ForkPayloadMigrationTarget<'_>,
    ) -> Result<Option<ForkPayloadMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// Current private structural migration authority.
#[derive(Clone, Copy, Debug)]
pub struct ForkStructuralMigrationTarget {
    version: u32,
}

impl ForkStructuralMigrationTarget {
    /// Returns the private structural family.
    #[must_use]
    pub const fn family(self) -> &'static str {
        FORK_HISTORY_FORMAT_FAMILY
    }

    /// Returns the current private structural version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }
}

/// One exact private structural migration step.
#[derive(Clone, Debug, PartialEq)]
pub struct ForkStructuralMigrationStep {
    version: u32,
    document: Value,
}

impl ForkStructuralMigrationStep {
    /// Constructs one exact next-version structural step.
    #[must_use]
    pub const fn new(version: u32, document: Value) -> Self {
        Self { version, document }
    }
}

/// Registered one-step migration for older private graph envelopes.
pub trait ForkStructuralMigration {
    /// Structural migration failure.
    type Error;

    /// Produces one exact next-version migration step.
    fn migrate_one(
        &self,
        from: u32,
        document: Value,
        target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error>;
}

/// Explicit registration with no older private structural migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoForkStructuralMigration;

impl ForkStructuralMigration for NoForkStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        _from: u32,
        _document: Value,
        _target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// Registered private graph persistence authority.
#[derive(Clone, Debug)]
pub struct ForkPersistence<C, M> {
    codec: C,
    structural_migration: M,
}

impl<C, M> ForkPersistence<C, M> {
    /// Registers payload and structural migration authority.
    #[must_use]
    pub const fn new(codec: C, structural_migration: M) -> Self {
        Self {
            codec,
            structural_migration,
        }
    }
}

impl<C> ForkPersistence<C, NoForkStructuralMigration> {
    /// Registers a codec with no older structural migration.
    #[must_use]
    pub const fn without_structural_migration(codec: C) -> Self {
        Self::new(codec, NoForkStructuralMigration)
    }
}

impl<C, M> ForkPersistence<C, M>
where
    M: ForkStructuralMigration,
{
    /// Encodes one complete private graph.
    pub fn encode<P>(&self, history: &ForkHistory<P>) -> Result<Vec<u8>, ForkEncodeError<C::Error>>
    where
        C: ForkPayloadCodec<P>,
    {
        let nodes = history
            .nodes
            .values()
            .map(|node| {
                let payload = self.codec.encode(node.payload()).map_err(|error| {
                    ForkEncodeError::Payload {
                        entry_id: node.entry_id().clone(),
                        error,
                    }
                })?;
                let actual =
                    u64::try_from(payload.len()).map_err(|_| ForkEncodeError::SizeOverflow)?;
                if actual != node.encoded_weight() {
                    return Err(ForkEncodeError::PayloadWeightMismatch {
                        entry_id: node.entry_id().clone(),
                        recorded: node.encoded_weight(),
                        actual,
                    });
                }
                Ok(Node {
                    entry_id: node.entry_id().clone(),
                    parent_entry_id: node.parent_entry_id().cloned(),
                    label: node.metadata().label().as_str().to_owned(),
                    kind_id: node.metadata().kind_id().cloned(),
                    group_id: node.metadata().group_id().cloned(),
                    sequence: node.sequence().get(),
                    committed_revision: node.committed_revision(),
                    encoded_weight: node.encoded_weight(),
                    payload,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let envelope = Envelope {
            format_family: FORK_HISTORY_FORMAT_FAMILY.to_owned(),
            structural_version: CURRENT_FORK_HISTORY_STRUCTURAL_VERSION,
            payload_codec: PayloadCodec {
                family: self.codec.family().clone(),
                version: self.codec.version(),
            },
            history_id: history.history_id.clone(),
            revision: history.revision,
            current_branch_id: history.current_branch_id.clone(),
            current_node_id: history.current_node_id.clone(),
            next_sequence: history.next_sequence.get(),
            nodes,
            branches: history
                .branches
                .values()
                .map(|branch| Branch {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    name: branch.metadata().name().map(str::to_owned),
                    annotation: branch.metadata().annotation().map(str::to_owned),
                    pinned: branch.metadata().pinned(),
                })
                .collect(),
            preferred_children: history
                .preferred_children
                .iter()
                .map(|(parent, child)| PreferredChild {
                    parent_entry_id: parent.clone(),
                    child_entry_id: child.clone(),
                })
                .collect(),
            checkpoints: history
                .checkpoints
                .values()
                .map(|checkpoint| Checkpoint {
                    checkpoint_id: checkpoint.checkpoint_id().clone(),
                    after_entry_id: checkpoint.after_entry_id().cloned(),
                    consumer_reference: checkpoint.consumer_reference().to_owned(),
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(ForkEncodeError::Structural)?;
        if bytes.len() > MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES {
            return Err(ForkEncodeError::EnvelopeTooLarge {
                maximum: MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES,
                actual: bytes.len(),
            });
        }
        Ok(bytes)
    }

    /// Loads a fully validated graph without replacing any live authority.
    pub fn load<P>(
        &self,
        expected_history_id: &HistoryId,
        bytes: &[u8],
    ) -> Result<ForkLoadResult<P>, ForkLoadError<C::Error, M::Error>>
    where
        C: ForkPayloadCodec<P>,
    {
        if bytes.len() > MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES {
            return Err(ForkLoadError::EnvelopeTooLarge {
                maximum: MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES,
                actual: bytes.len(),
            });
        }
        let mut document: Value =
            serde_json::from_slice(bytes).map_err(ForkLoadError::InvalidJson)?;
        let header = serde_json::from_value::<wire::Header>(document.clone())
            .map_err(ForkLoadError::InvalidHeader)?;
        if header.format_family != FORK_HISTORY_FORMAT_FAMILY {
            return Err(ForkLoadError::ForeignFormatFamily {
                actual: header.format_family,
            });
        }
        if header.structural_version > CURRENT_FORK_HISTORY_STRUCTURAL_VERSION {
            return Err(ForkLoadError::FutureStructuralVersion {
                actual: header.structural_version,
                maximum: CURRENT_FORK_HISTORY_STRUCTURAL_VERSION,
            });
        }
        let source_structural_version = header.structural_version;
        let target = ForkStructuralMigrationTarget {
            version: CURRENT_FORK_HISTORY_STRUCTURAL_VERSION,
        };
        let mut version = source_structural_version;
        while version < CURRENT_FORK_HISTORY_STRUCTURAL_VERSION {
            let expected =
                version
                    .checked_add(1)
                    .ok_or(ForkLoadError::InvalidStructuralMigration {
                        from: version,
                        produced: version,
                    })?;
            let step = self
                .structural_migration
                .migrate_one(version, document, target)
                .map_err(ForkLoadError::StructuralMigration)?
                .ok_or(ForkLoadError::MissingStructuralMigration { from: version })?;
            if step.version != expected {
                return Err(ForkLoadError::InvalidStructuralMigration {
                    from: version,
                    produced: step.version,
                });
            }
            let migrated_header = serde_json::from_value::<wire::Header>(step.document.clone())
                .map_err(ForkLoadError::InvalidHeader)?;
            if migrated_header.format_family != FORK_HISTORY_FORMAT_FAMILY
                || migrated_header.structural_version != step.version
            {
                return Err(ForkLoadError::InvalidStructuralMigration {
                    from: version,
                    produced: migrated_header.structural_version,
                });
            }
            version = step.version;
            document = step.document;
        }
        let envelope: Envelope =
            serde_json::from_value(document).map_err(ForkLoadError::InvalidEnvelope)?;
        if &envelope.history_id != expected_history_id {
            return Err(ForkLoadError::ForeignHistory {
                expected: expected_history_id.clone(),
                actual: envelope.history_id,
            });
        }
        if &envelope.payload_codec.family != self.codec.family() {
            return Err(ForkLoadError::ForeignPayloadCodecFamily);
        }
        if envelope.payload_codec.version > self.codec.version() {
            return Err(ForkLoadError::FuturePayloadCodecVersion {
                actual: envelope.payload_codec.version,
                maximum: self.codec.version(),
            });
        }
        let source_payload_version = envelope.payload_codec.version;
        let history = validation::decode_graph(envelope, &self.codec)?;
        let outcome = if source_structural_version == CURRENT_FORK_HISTORY_STRUCTURAL_VERSION
            && source_payload_version == self.codec.version()
        {
            ForkLoadOutcome::Preserved
        } else {
            ForkLoadOutcome::Migrated {
                structural: source_structural_version != CURRENT_FORK_HISTORY_STRUCTURAL_VERSION,
                payload: source_payload_version != self.codec.version(),
            }
        };
        Ok(ForkLoadResult {
            history,
            receipt: ForkLoadReceipt {
                outcome,
                source_structural_version,
                source_payload_version,
            },
        })
    }
}
