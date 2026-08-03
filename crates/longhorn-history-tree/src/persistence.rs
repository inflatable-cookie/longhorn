use std::{convert::Infallible, error::Error, fmt};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryPayloadMigrationTarget,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;

use crate::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkCheckpoint, ForkCheckpointId, ForkHistory,
    ForkHistoryNode, ForkHistoryState, ForkHistoryStateError, ForkPreferredChild,
};

/// Stable structural format family for fork-tree envelopes.
pub const FORK_HISTORY_FORMAT_FAMILY: &str = "longhorn.history-tree";
/// Current graph envelope version.
pub const CURRENT_FORK_HISTORY_STRUCTURAL_VERSION: u32 = 1;
/// Defensive ceiling for one encoded graph envelope.
pub const MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES: usize = 1 << 30;

/// Caller-selected bound for untrusted graph-envelope bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkPersistenceLimits {
    maximum_envelope_bytes: usize,
}

impl ForkPersistenceLimits {
    /// Validates one explicit load and encode bound.
    pub const fn new(maximum_envelope_bytes: usize) -> Result<Self, ForkPersistenceLimitsError> {
        if maximum_envelope_bytes == 0 {
            return Err(ForkPersistenceLimitsError::Zero);
        }
        if maximum_envelope_bytes > MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES {
            return Err(ForkPersistenceLimitsError::TooLarge {
                maximum: MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES,
                actual: maximum_envelope_bytes,
            });
        }
        Ok(Self {
            maximum_envelope_bytes,
        })
    }

    /// Returns the maximum accepted or produced envelope size.
    #[must_use]
    pub const fn maximum_envelope_bytes(self) -> usize {
        self.maximum_envelope_bytes
    }
}

/// Invalid graph-persistence byte bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkPersistenceLimitsError {
    /// The bound was zero.
    Zero,
    /// The bound exceeded the defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied bound.
        actual: usize,
    },
}

impl fmt::Display for ForkPersistenceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("fork persistence bound must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "fork persistence bound is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for ForkPersistenceLimitsError {}

/// Current structural migration authority.
#[derive(Clone, Copy, Debug)]
pub struct ForkStructuralMigrationTarget {
    version: u32,
}

impl ForkStructuralMigrationTarget {
    /// Returns the structural family.
    #[must_use]
    pub const fn family(self) -> &'static str {
        FORK_HISTORY_FORMAT_FAMILY
    }

    /// Returns the current structural version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }
}

/// One exact next-version structural migration step.
#[derive(Clone, Debug, PartialEq)]
pub struct ForkStructuralMigrationStep {
    version: u32,
    document: Value,
}

impl ForkStructuralMigrationStep {
    /// Constructs one structural migration step.
    #[must_use]
    pub const fn new(version: u32, document: Value) -> Self {
        Self { version, document }
    }
}

/// Registered one-step migration for older graph envelopes.
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

/// Explicit registration with no older structural migration.
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

/// Registered graph persistence authority.
#[derive(Clone, Debug)]
pub struct ForkPersistence<C, M> {
    codec: C,
    structural_migration: M,
    limits: ForkPersistenceLimits,
}

impl<C, M> ForkPersistence<C, M> {
    /// Registers payload and structural migration authority.
    #[must_use]
    pub const fn new(codec: C, structural_migration: M, limits: ForkPersistenceLimits) -> Self {
        Self {
            codec,
            structural_migration,
            limits,
        }
    }

    /// Returns the configured untrusted-byte bound.
    #[must_use]
    pub const fn limits(&self) -> ForkPersistenceLimits {
        self.limits
    }
}

impl<C> ForkPersistence<C, NoForkStructuralMigration> {
    /// Registers a codec with no older structural migration.
    #[must_use]
    pub const fn without_structural_migration(codec: C, limits: ForkPersistenceLimits) -> Self {
        Self::new(codec, NoForkStructuralMigration, limits)
    }
}

impl<C, M> ForkPersistence<C, M>
where
    M: ForkStructuralMigration,
{
    /// Encodes one complete graph deterministically with base64 payload bytes.
    pub fn encode<P>(&self, history: &ForkHistory<P>) -> Result<Vec<u8>, ForkEncodeError<C::Error>>
    where
        C: HistoryPayloadCodec<P>,
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
        if bytes.len() > self.limits.maximum_envelope_bytes() {
            return Err(ForkEncodeError::EnvelopeTooLarge {
                maximum: self.limits.maximum_envelope_bytes(),
                actual: bytes.len(),
            });
        }
        Ok(bytes)
    }

    /// Loads a fully validated graph without replacing live authority.
    pub fn load<P>(
        &self,
        expected_history_id: &HistoryId,
        bytes: &[u8],
    ) -> Result<ForkLoadResult<P>, ForkLoadError<C::Error, M::Error>>
    where
        C: HistoryPayloadCodec<P>,
    {
        if bytes.len() > self.limits.maximum_envelope_bytes() {
            return Err(ForkLoadError::EnvelopeTooLarge {
                maximum: self.limits.maximum_envelope_bytes(),
                actual: bytes.len(),
            });
        }
        let mut document: Value =
            serde_json::from_slice(bytes).map_err(ForkLoadError::InvalidJson)?;
        let header = Header::deserialize(&document).map_err(ForkLoadError::InvalidHeader)?;
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
            let migrated_header =
                Header::deserialize(&step.document).map_err(ForkLoadError::InvalidHeader)?;
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
            return Err(ForkLoadError::ForeignPayloadCodecFamily {
                expected: self.codec.family().clone(),
                actual: envelope.payload_codec.family,
            });
        }
        if envelope.payload_codec.version > self.codec.version() {
            return Err(ForkLoadError::FuturePayloadCodecVersion {
                actual: envelope.payload_codec.version,
                maximum: self.codec.version(),
            });
        }
        let source_payload_version = envelope.payload_codec.version;
        let history = decode_graph(envelope, &self.codec)?;
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

fn decode_graph<P, C, ME>(
    envelope: Envelope,
    codec: &C,
) -> Result<ForkHistory<P>, ForkLoadError<C::Error, ME>>
where
    C: HistoryPayloadCodec<P>,
{
    let source_version = envelope.payload_codec.version;
    let target = HistoryPayloadMigrationTarget::new(codec.family(), codec.version());
    let mut nodes = Vec::with_capacity(envelope.nodes.len());
    for node in envelope.nodes {
        let source_weight =
            u64::try_from(node.payload.len()).map_err(|_| ForkLoadError::PayloadWeightOverflow)?;
        if source_weight != node.encoded_weight {
            return Err(ForkLoadError::PayloadWeightMismatch {
                entry_id: node.entry_id,
                recorded: node.encoded_weight,
                actual: source_weight,
            });
        }
        let entry_id = node.entry_id.clone();
        let mut payload_version = source_version;
        let mut bytes = node.payload;
        while payload_version < codec.version() {
            let expected = payload_version
                .get()
                .checked_add(1)
                .map(HistoryPayloadCodecVersion::new)
                .ok_or_else(|| ForkLoadError::InvalidPayloadMigration {
                    entry_id: entry_id.clone(),
                    from: payload_version,
                    produced: payload_version,
                })?;
            let step = codec
                .migrate_one(payload_version, bytes, target)
                .map_err(|error| ForkLoadError::Payload {
                    entry_id: entry_id.clone(),
                    error,
                })?
                .ok_or_else(|| ForkLoadError::MissingPayloadMigration {
                    entry_id: entry_id.clone(),
                    from: payload_version,
                })?;
            if step.version() != expected {
                return Err(ForkLoadError::InvalidPayloadMigration {
                    entry_id,
                    from: payload_version,
                    produced: step.version(),
                });
            }
            (payload_version, bytes) = step.into_parts();
        }
        let encoded_weight =
            u64::try_from(bytes.len()).map_err(|_| ForkLoadError::PayloadWeightOverflow)?;
        let payload = codec
            .decode(&bytes)
            .map_err(|error| ForkLoadError::Payload {
                entry_id: node.entry_id.clone(),
                error,
            })?;
        let sequence = HistoryEntrySequence::new(node.sequence)
            .map_err(|_| ForkLoadError::InvalidSequence(node.entry_id.clone()))?;
        let label = HistoryLabel::new(node.label)
            .map_err(|_| ForkLoadError::InvalidLabel(node.entry_id.clone()))?;
        nodes.push(ForkHistoryNode::new(
            node.entry_id,
            node.parent_entry_id,
            HistoryEntryMetadata::new(label, node.kind_id, node.group_id),
            sequence,
            node.committed_revision,
            encoded_weight,
            payload,
        ));
    }
    let branches = envelope
        .branches
        .into_iter()
        .map(|branch| {
            let metadata =
                ForkBranchMetadata::new(branch.name, branch.annotation, branch.pinned)
                    .map_err(|_| ForkLoadError::InvalidBranchMetadata(branch.branch_id.clone()))?;
            Ok(ForkBranch::new(
                branch.branch_id,
                branch.head_entry_id,
                metadata,
            ))
        })
        .collect::<Result<Vec<_>, ForkLoadError<C::Error, ME>>>()?;
    let checkpoints = envelope
        .checkpoints
        .into_iter()
        .map(|checkpoint| {
            let checkpoint_id = checkpoint.checkpoint_id.clone();
            ForkCheckpoint::new(
                checkpoint.checkpoint_id,
                checkpoint.after_entry_id,
                checkpoint.consumer_reference,
            )
            .map_err(|_| ForkLoadError::InvalidCheckpoint(checkpoint_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let next_sequence = HistoryEntrySequence::new(envelope.next_sequence)
        .map_err(|_| ForkLoadError::InvalidNextSequence)?;
    let state = ForkHistoryState::new(
        envelope.history_id,
        envelope.revision,
        envelope.current_branch_id,
        envelope.current_node_id,
        next_sequence,
    )
    .with_nodes(nodes)
    .with_branches(branches)
    .with_preferred_children(
        envelope
            .preferred_children
            .into_iter()
            .map(|preferred| {
                ForkPreferredChild::new(preferred.parent_entry_id, preferred.child_entry_id)
            })
            .collect(),
    )
    .with_checkpoints(checkpoints);
    ForkHistory::from_state(state).map_err(ForkLoadError::Validation)
}

/// Successful compatibility outcome.
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

/// Successful load receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkLoadReceipt {
    outcome: ForkLoadOutcome,
    source_structural_version: u32,
    source_payload_version: HistoryPayloadCodecVersion,
}

impl ForkLoadReceipt {
    /// Returns whether source bytes were preserved or migrated.
    #[must_use]
    pub const fn outcome(self) -> ForkLoadOutcome {
        self.outcome
    }

    /// Returns the source structural version.
    #[must_use]
    pub const fn source_structural_version(self) -> u32 {
        self.source_structural_version
    }

    /// Returns the source payload version.
    #[must_use]
    pub const fn source_payload_version(self) -> HistoryPayloadCodecVersion {
        self.source_payload_version
    }
}

/// A fully validated graph and visible load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkLoadResult<P> {
    history: ForkHistory<P>,
    receipt: ForkLoadReceipt,
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

/// Failed graph encoding.
#[derive(Debug)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    format_family: String,
    structural_version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Envelope {
    format_family: String,
    structural_version: u32,
    payload_codec: PayloadCodec,
    history_id: HistoryId,
    revision: HistoryRevision,
    current_branch_id: ForkBranchId,
    current_node_id: Option<HistoryEntryId>,
    next_sequence: u64,
    nodes: Vec<Node>,
    branches: Vec<Branch>,
    preferred_children: Vec<PreferredChild>,
    checkpoints: Vec<Checkpoint>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PayloadCodec {
    family: HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Node {
    entry_id: HistoryEntryId,
    parent_entry_id: Option<HistoryEntryId>,
    label: String,
    kind_id: Option<HistoryKindId>,
    group_id: Option<HistoryGroupId>,
    sequence: u64,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    #[serde(with = "base64_bytes")]
    payload: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Branch {
    branch_id: ForkBranchId,
    head_entry_id: Option<HistoryEntryId>,
    name: Option<String>,
    annotation: Option<String>,
    pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PreferredChild {
    parent_entry_id: Option<HistoryEntryId>,
    child_entry_id: HistoryEntryId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Checkpoint {
    checkpoint_id: ForkCheckpointId,
    after_entry_id: Option<HistoryEntryId>,
    consumer_reference: String,
}

mod base64_bytes {
    use super::*;

    pub(super) fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        STANDARD.decode(value).map_err(de::Error::custom)
    }
}
