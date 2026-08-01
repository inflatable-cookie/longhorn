use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::{ForkCheckpointId, ForkHistory};

/// Hard byte limit for an opaque consumer checkpoint reference.
pub const MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES: usize = 4_096;

/// Opaque consumer checkpoint attached after one node or at the root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkCheckpoint {
    checkpoint_id: ForkCheckpointId,
    after_entry_id: Option<HistoryEntryId>,
    consumer_reference: String,
}

impl ForkCheckpoint {
    pub(crate) const fn new(
        checkpoint_id: ForkCheckpointId,
        after_entry_id: Option<HistoryEntryId>,
        consumer_reference: String,
    ) -> Self {
        Self {
            checkpoint_id,
            after_entry_id,
            consumer_reference,
        }
    }

    /// Returns the stable checkpoint identity.
    #[must_use]
    pub const fn checkpoint_id(&self) -> &ForkCheckpointId {
        &self.checkpoint_id
    }

    /// Returns the node captured by the checkpoint, or the graph root.
    #[must_use]
    pub const fn after_entry_id(&self) -> Option<&HistoryEntryId> {
        self.after_entry_id.as_ref()
    }

    /// Returns the opaque consumer reference without interpreting it.
    #[must_use]
    pub fn consumer_reference(&self) -> &str {
        &self.consumer_reference
    }
}

/// Successful checkpoint registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkCheckpointReceipt {
    previous_revision: HistoryRevision,
    committed_revision: HistoryRevision,
    checkpoint_id: ForkCheckpointId,
}

impl ForkCheckpointReceipt {
    /// Returns the source graph revision.
    #[must_use]
    pub const fn previous_revision(&self) -> HistoryRevision {
        self.previous_revision
    }

    /// Returns the committed graph revision.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the registered checkpoint identity.
    #[must_use]
    pub const fn checkpoint_id(&self) -> &ForkCheckpointId {
        &self.checkpoint_id
    }
}

/// Replay work after the nearest checkpoint ancestor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkReplayCost {
    checkpoint_id: Option<ForkCheckpointId>,
    entry_count: usize,
    encoded_weight: u64,
}

impl ForkReplayCost {
    /// Returns the selected nearest checkpoint.
    #[must_use]
    pub const fn checkpoint_id(&self) -> Option<&ForkCheckpointId> {
        self.checkpoint_id.as_ref()
    }

    /// Returns forward entries required after the checkpoint.
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Returns consumer-measured payload bytes requiring replay.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }
}

impl<P> ForkHistory<P> {
    /// Registers an opaque consumer checkpoint without storing checkpoint data.
    pub fn register_checkpoint(
        &mut self,
        expected_revision: HistoryRevision,
        checkpoint_id: ForkCheckpointId,
        after_entry_id: Option<HistoryEntryId>,
        consumer_reference: String,
    ) -> Result<ForkCheckpointReceipt, ForkCheckpointError> {
        if expected_revision != self.revision {
            return Err(ForkCheckpointError::StaleRevision {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if self.checkpoints.contains_key(&checkpoint_id) {
            return Err(ForkCheckpointError::DuplicateCheckpoint(checkpoint_id));
        }
        if consumer_reference.is_empty() {
            return Err(ForkCheckpointError::EmptyReference);
        }
        if consumer_reference.len() > MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES {
            return Err(ForkCheckpointError::ReferenceTooLong {
                maximum: MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES,
                actual: consumer_reference.len(),
            });
        }
        if let Some(entry_id) = &after_entry_id {
            if !self.nodes.contains_key(entry_id) {
                return Err(ForkCheckpointError::UnknownEntry(entry_id.clone()));
            }
        }
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkCheckpointError::RevisionOverflow)?;
        self.checkpoints.insert(
            checkpoint_id.clone(),
            ForkCheckpoint::new(checkpoint_id.clone(), after_entry_id, consumer_reference),
        );
        self.revision = committed_revision;
        Ok(ForkCheckpointReceipt {
            previous_revision: expected_revision,
            committed_revision,
            checkpoint_id,
        })
    }

    /// Returns all opaque checkpoints in stable identity order.
    pub fn checkpoints(&self) -> impl Iterator<Item = &ForkCheckpoint> {
        self.checkpoints.values()
    }

    /// Computes replay work from the nearest checkpoint ancestor.
    pub fn replay_cost(
        &self,
        target: Option<&HistoryEntryId>,
    ) -> Result<ForkReplayCost, ForkCheckpointError> {
        let lineage = self
            .lineage::<ForkCheckpointError>(target)
            .map_err(|error| match error {
                crate::ForkNavigationError::UnknownEntry(entry_id) => {
                    ForkCheckpointError::UnknownEntry(entry_id)
                }
                _ => ForkCheckpointError::InvalidTopology,
            })?;
        let mut selected: Option<(usize, &ForkCheckpoint)> = None;
        for checkpoint in self.checkpoints.values() {
            let depth = match checkpoint.after_entry_id() {
                None => 0,
                Some(entry_id) => lineage
                    .iter()
                    .position(|candidate| candidate == entry_id)
                    .map_or(usize::MAX, |index| index + 1),
            };
            if depth == usize::MAX {
                continue;
            }
            let replace = selected.as_ref().is_none_or(|(current_depth, current)| {
                depth > *current_depth
                    || (depth == *current_depth
                        && checkpoint.checkpoint_id() < current.checkpoint_id())
            });
            if replace {
                selected = Some((depth, checkpoint));
            }
        }
        let replay_start = selected.as_ref().map_or(0, |(depth, _)| *depth);
        let encoded_weight = lineage[replay_start..]
            .iter()
            .try_fold(0_u64, |total, entry_id| {
                total.checked_add(
                    self.nodes
                        .get(entry_id)
                        .expect("lineage node exists")
                        .encoded_weight(),
                )
            })
            .ok_or(ForkCheckpointError::WeightOverflow)?;
        Ok(ForkReplayCost {
            checkpoint_id: selected.map(|(_, checkpoint)| checkpoint.checkpoint_id().clone()),
            entry_count: lineage.len() - replay_start,
            encoded_weight,
        })
    }
}

/// Rejected checkpoint or replay-cost operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkCheckpointError {
    /// Request revision was stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Checkpoint identity already exists.
    DuplicateCheckpoint(ForkCheckpointId),
    /// Opaque consumer reference was empty.
    EmptyReference,
    /// Opaque consumer reference exceeded its hard limit.
    ReferenceTooLong {
        /// Maximum accepted bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// Attached or target entry does not exist.
    UnknownEntry(HistoryEntryId),
    /// Topology could not produce one finite lineage.
    InvalidTopology,
    /// Replay weight overflowed.
    WeightOverflow,
    /// Revision could not advance.
    RevisionOverflow,
}
