//! Decode a validated envelope into fork-history authority.

use longhorn_core::HistoryRevision;
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel, HistoryPayloadCodec,
    HistoryPayloadCodecVersion, HistoryPayloadMigrationTarget,
};

use crate::{
    ForkBranch, ForkBranchMetadata, ForkCheckpoint, ForkHistory, ForkHistoryNode, ForkHistoryState,
    ForkPreferredChild,
};

use super::{
    ForkLoadError,
    wire::{Envelope, Node},
};

pub(crate) fn decode_graph<P, C, ME>(
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
