use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{HistoryEntryId, HistoryRevision};
use longhorn_history::{HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel};

use super::{
    ForkLoadError, ForkPayloadCodec, ForkPayloadMigrationStep, ForkPayloadMigrationTarget,
    ForkPersistenceValidationError,
    wire::{Envelope, Node},
};
use crate::{ForkBranch, ForkBranchMetadata, ForkCheckpoint, ForkHistory, ForkHistoryNode};

pub(super) fn decode_graph<P, C, ME>(
    envelope: Envelope,
    codec: &C,
) -> Result<ForkHistory<P>, ForkLoadError<C::Error, ME>>
where
    C: ForkPayloadCodec<P>,
{
    let source_version = envelope.payload_codec.version;
    let target = ForkPayloadMigrationTarget {
        family: codec.family(),
        version: codec.version(),
    };
    let mut nodes = BTreeMap::new();
    let mut sequences = BTreeSet::new();
    let mut retained_weight = 0_u64;
    for node in envelope.nodes {
        let entry_id = node.entry_id.clone();
        let decoded = decode_node::<P, C, ME>(node, source_version, target, codec)?;
        if !sequences.insert(decoded.sequence().get()) {
            return invalid(ForkPersistenceValidationError::InvalidSequence);
        }
        retained_weight = retained_weight
            .checked_add(decoded.encoded_weight())
            .ok_or_else(|| {
                ForkLoadError::Validation(ForkPersistenceValidationError::WeightOverflow)
            })?;
        if nodes.insert(entry_id.clone(), decoded).is_some() {
            return invalid(ForkPersistenceValidationError::DuplicateNode(entry_id));
        }
    }
    validate_nodes(&nodes, envelope.revision, envelope.next_sequence)?;

    let mut branches = BTreeMap::new();
    for branch in envelope.branches {
        if branch
            .head_entry_id
            .as_ref()
            .is_some_and(|entry_id| !nodes.contains_key(entry_id))
        {
            return invalid(ForkPersistenceValidationError::UnknownNodeReference);
        }
        let metadata = ForkBranchMetadata::new(branch.name, branch.annotation, branch.pinned)
            .map_err(|_| {
                ForkLoadError::Validation(ForkPersistenceValidationError::InvalidBranchMetadata)
            })?;
        let branch_id = branch.branch_id.clone();
        if branches
            .insert(
                branch_id,
                ForkBranch::new(branch.branch_id, branch.head_entry_id, metadata),
            )
            .is_some()
        {
            return invalid(ForkPersistenceValidationError::DuplicateBranch);
        }
    }

    let mut children: BTreeMap<Option<HistoryEntryId>, Vec<HistoryEntryId>> = BTreeMap::new();
    for node in nodes.values() {
        children
            .entry(node.parent_entry_id().cloned())
            .or_default()
            .push(node.entry_id().clone());
    }
    for child_ids in children.values_mut() {
        child_ids.sort_by_key(|entry_id| {
            nodes
                .get(entry_id)
                .expect("child index was built from nodes")
                .sequence()
        });
    }

    let mut preferred_children = BTreeMap::new();
    for preferred in envelope.preferred_children {
        let is_direct = nodes
            .get(&preferred.child_entry_id)
            .is_some_and(|node| node.parent_entry_id() == preferred.parent_entry_id.as_ref());
        if !is_direct {
            return invalid(ForkPersistenceValidationError::InvalidPreferredChild);
        }
        if preferred_children
            .insert(preferred.parent_entry_id, preferred.child_entry_id)
            .is_some()
        {
            return invalid(ForkPersistenceValidationError::DuplicatePreference);
        }
    }

    let mut checkpoints = BTreeMap::new();
    for checkpoint in envelope.checkpoints {
        if checkpoint.consumer_reference.is_empty()
            || checkpoint.consumer_reference.len() > crate::MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES
        {
            return invalid(ForkPersistenceValidationError::InvalidCheckpointReference);
        }
        if checkpoint
            .after_entry_id
            .as_ref()
            .is_some_and(|entry_id| !nodes.contains_key(entry_id))
        {
            return invalid(ForkPersistenceValidationError::UnknownNodeReference);
        }
        let checkpoint_id = checkpoint.checkpoint_id.clone();
        if checkpoints
            .insert(
                checkpoint_id,
                ForkCheckpoint::new(
                    checkpoint.checkpoint_id,
                    checkpoint.after_entry_id,
                    checkpoint.consumer_reference,
                ),
            )
            .is_some()
        {
            return invalid(ForkPersistenceValidationError::DuplicateCheckpoint);
        }
    }

    let next_sequence = HistoryEntrySequence::new(envelope.next_sequence).map_err(|_| {
        ForkLoadError::Validation(ForkPersistenceValidationError::InvalidNextSequence)
    })?;
    let history = ForkHistory {
        history_id: envelope.history_id,
        revision: envelope.revision,
        nodes,
        children,
        branches,
        current_branch_id: envelope.current_branch_id,
        current_node_id: envelope.current_node_id,
        preferred_children,
        checkpoints,
        next_sequence,
    };
    if !history.branches.contains_key(history.current_branch_id())
        || history
            .current_node_id()
            .is_some_and(|entry_id| !history.nodes.contains_key(entry_id))
        || !history.branch_contains(history.current_branch_id(), history.current_node_id())
    {
        return invalid(ForkPersistenceValidationError::InvalidCurrentPosition);
    }
    Ok(history)
}

fn decode_node<P, C, ME>(
    node: Node,
    source_version: longhorn_history::HistoryPayloadCodecVersion,
    target: ForkPayloadMigrationTarget<'_>,
    codec: &C,
) -> Result<ForkHistoryNode<P>, ForkLoadError<C::Error, ME>>
where
    C: ForkPayloadCodec<P>,
{
    let source_weight = u64::try_from(node.payload.len())
        .map_err(|_| ForkLoadError::Validation(ForkPersistenceValidationError::WeightOverflow))?;
    if source_weight != node.encoded_weight {
        return invalid(ForkPersistenceValidationError::PayloadWeightMismatch);
    }
    let mut version = source_version;
    let mut bytes = node.payload;
    while version < codec.version() {
        let expected = version
            .get()
            .checked_add(1)
            .map(longhorn_history::HistoryPayloadCodecVersion::new)
            .ok_or_else(|| ForkLoadError::InvalidPayloadMigration {
                entry_id: node.entry_id.clone(),
                from: version,
                produced: version,
            })?;
        let step = codec
            .migrate_one(version, bytes, target)
            .map_err(|error| ForkLoadError::Payload {
                entry_id: node.entry_id.clone(),
                error,
            })?
            .ok_or_else(|| ForkLoadError::MissingPayloadMigration {
                entry_id: node.entry_id.clone(),
                from: version,
            })?;
        let ForkPayloadMigrationStep {
            version: produced,
            bytes: migrated,
        } = step;
        if produced != expected {
            return Err(ForkLoadError::InvalidPayloadMigration {
                entry_id: node.entry_id,
                from: version,
                produced,
            });
        }
        version = produced;
        bytes = migrated;
    }
    let encoded_weight = u64::try_from(bytes.len())
        .map_err(|_| ForkLoadError::Validation(ForkPersistenceValidationError::WeightOverflow))?;
    let payload = codec
        .decode(&bytes)
        .map_err(|error| ForkLoadError::Payload {
            entry_id: node.entry_id.clone(),
            error,
        })?;
    let sequence = HistoryEntrySequence::new(node.sequence)
        .map_err(|_| ForkLoadError::Validation(ForkPersistenceValidationError::InvalidSequence))?;
    let label = HistoryLabel::new(node.label)
        .map_err(|_| ForkLoadError::Validation(ForkPersistenceValidationError::InvalidLabel))?;
    Ok(ForkHistoryNode::new(
        node.entry_id,
        node.parent_entry_id,
        HistoryEntryMetadata::new(label, node.kind_id, node.group_id),
        sequence,
        node.committed_revision,
        encoded_weight,
        payload,
    ))
}

fn validate_nodes<P, CE, ME>(
    nodes: &BTreeMap<HistoryEntryId, ForkHistoryNode<P>>,
    revision: HistoryRevision,
    next_sequence: u64,
) -> Result<(), ForkLoadError<CE, ME>> {
    let maximum_sequence = nodes
        .values()
        .map(|node| node.sequence().get())
        .max()
        .unwrap_or(0);
    if next_sequence == 0 || next_sequence <= maximum_sequence {
        return invalid(ForkPersistenceValidationError::InvalidNextSequence);
    }
    for node in nodes.values() {
        if node.committed_revision() > revision {
            return invalid(ForkPersistenceValidationError::InvalidCommittedRevision);
        }
        if let Some(parent_id) = node.parent_entry_id() {
            let Some(parent) = nodes.get(parent_id) else {
                return invalid(ForkPersistenceValidationError::InvalidParent);
            };
            if parent.sequence() >= node.sequence() {
                return invalid(ForkPersistenceValidationError::InvalidParent);
            }
        }
    }
    Ok(())
}

fn invalid<T, CE, ME>(error: ForkPersistenceValidationError) -> Result<T, ForkLoadError<CE, ME>> {
    Err(ForkLoadError::Validation(error))
}
