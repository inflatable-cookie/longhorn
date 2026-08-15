use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::{ForkCheckpointId, ForkHistory};

/// Hard byte limit for an opaque consumer checkpoint reference.
pub const MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES: usize = 4_096;
/// Defensive hard ceiling for checkpoint references.
pub const MAXIMUM_FORK_CHECKPOINTS: usize = 65_536;

/// Opaque consumer checkpoint attached after one node or at the root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkCheckpoint {
    checkpoint_id: ForkCheckpointId,
    after_entry_id: Option<HistoryEntryId>,
    consumer_reference: String,
}

impl ForkCheckpoint {
    /// Validates and constructs a checkpoint for registration or state import.
    pub fn new(
        checkpoint_id: ForkCheckpointId,
        after_entry_id: Option<HistoryEntryId>,
        consumer_reference: String,
    ) -> Result<Self, ForkCheckpointError> {
        if consumer_reference.is_empty() {
            return Err(ForkCheckpointError::EmptyReference);
        }
        if consumer_reference.len() > MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES {
            return Err(ForkCheckpointError::ReferenceTooLong {
                maximum: MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES,
                actual: consumer_reference.len(),
            });
        }
        Ok(Self {
            checkpoint_id,
            after_entry_id,
            consumer_reference,
        })
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

    /// Returns consumer-measured payload weight requiring replay.
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
        if self.checkpoints.len() >= MAXIMUM_FORK_CHECKPOINTS {
            return Err(ForkCheckpointError::CheckpointLimitReached {
                maximum: MAXIMUM_FORK_CHECKPOINTS,
            });
        }
        if let Some(entry_id) = &after_entry_id
            && !self.nodes.contains_key(entry_id)
        {
            return Err(ForkCheckpointError::UnknownEntry(entry_id.clone()));
        }
        let checkpoint =
            ForkCheckpoint::new(checkpoint_id.clone(), after_entry_id, consumer_reference)?;
        let committed_revision = self
            .revision
            .checked_next()
            .map_err(|_| ForkCheckpointError::RevisionOverflow)?;
        self.checkpoints.insert(checkpoint_id.clone(), checkpoint);
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
            .lineage(target)
            .map_err(ForkCheckpointError::UnknownEntry)?;
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
                        .expect("lineage contains only retained nodes")
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
    /// Checkpoint hard limit was reached.
    CheckpointLimitReached {
        /// Hard checkpoint limit.
        maximum: usize,
    },
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
    /// Replay weight overflowed.
    WeightOverflow,
    /// Revision could not advance.
    RevisionOverflow,
}

impl fmt::Display for ForkCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "fork history revision {} is stale; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicateCheckpoint(id) => {
                write!(formatter, "fork checkpoint {id} already exists")
            }
            Self::CheckpointLimitReached { maximum } => write!(
                formatter,
                "fork history reached its {maximum}-checkpoint hard limit"
            ),
            Self::EmptyReference => formatter.write_str("fork checkpoint reference is empty"),
            Self::ReferenceTooLong { maximum, actual } => write!(
                formatter,
                "fork checkpoint reference is {actual} bytes; maximum is {maximum}"
            ),
            Self::UnknownEntry(id) => {
                write!(formatter, "fork history entry {id} does not exist")
            }
            Self::WeightOverflow => formatter.write_str("fork checkpoint replay weight overflowed"),
            Self::RevisionOverflow => formatter.write_str("fork history revision cannot advance"),
        }
    }
}

impl Error for ForkCheckpointError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_id(value: &str) -> HistoryEntryId {
        HistoryEntryId::new(value).expect("fixture entry id")
    }

    fn checkpoint_id(value: &str) -> ForkCheckpointId {
        ForkCheckpointId::new(value).expect("fixture checkpoint id")
    }

    #[test]
    fn fork_checkpoint_error_messages_are_hand_written() {
        let revision = HistoryRevision::new;
        let cases: [(ForkCheckpointError, &str); 8] = [
            (
                ForkCheckpointError::StaleRevision {
                    expected: revision(3),
                    actual: revision(9),
                },
                "fork history revision 3 is stale; current revision is 9",
            ),
            (
                ForkCheckpointError::DuplicateCheckpoint(checkpoint_id("checkpoint:one")),
                "fork checkpoint checkpoint:one already exists",
            ),
            (
                ForkCheckpointError::CheckpointLimitReached { maximum: 8 },
                "fork history reached its 8-checkpoint hard limit",
            ),
            (
                ForkCheckpointError::EmptyReference,
                "fork checkpoint reference is empty",
            ),
            (
                ForkCheckpointError::ReferenceTooLong {
                    maximum: 64,
                    actual: 65,
                },
                "fork checkpoint reference is 65 bytes; maximum is 64",
            ),
            (
                ForkCheckpointError::UnknownEntry(entry_id("entry:a")),
                "fork history entry entry:a does not exist",
            ),
            (
                ForkCheckpointError::WeightOverflow,
                "fork checkpoint replay weight overflowed",
            ),
            (
                ForkCheckpointError::RevisionOverflow,
                "fork history revision cannot advance",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
