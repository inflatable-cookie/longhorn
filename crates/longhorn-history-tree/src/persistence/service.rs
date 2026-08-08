//! Encode and load service for fork-history envelopes.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::{HistoryPayloadCodec, HistoryPayloadMigrationTarget};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    ForkBranch, ForkHistory, ForkHistoryState, ForkPreferredChild,
};

use super::{
    CURRENT_FORK_HISTORY_STRUCTURAL_VERSION, FORK_HISTORY_FORMAT_FAMILY, ForkEncodeError,
    ForkLoadError, ForkLoadOutcome, ForkLoadReceipt, ForkLoadResult, ForkPersistenceLimits,
    ForkStructuralMigration, ForkStructuralMigrationTarget, NoForkStructuralMigration,
    decode_graph,
    wire::{Branch, Checkpoint, Envelope, Header, Node, PayloadCodec, PreferredChild},
};

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
