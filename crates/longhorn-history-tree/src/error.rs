use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryRevision};

use crate::{ForkBranchId, ForkCheckpointId};

/// Rejected fork-history transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkHistoryError {
    /// Request revision was stale.
    StaleRevision {
        /// Requested revision.
        expected: HistoryRevision,
        /// Current revision.
        actual: HistoryRevision,
    },
    /// Entry identity already exists.
    DuplicateEntry(HistoryEntryId),
    /// Branch identity already exists.
    DuplicateBranch(ForkBranchId),
    /// Divergent record omitted its new stable branch identity.
    DivergentBranchRequired,
    /// Non-divergent record supplied an unnecessary branch identity.
    UnexpectedDivergentBranch,
    /// Branch identity does not exist.
    UnknownBranch(ForkBranchId),
    /// Node hard limit was reached.
    NodeLimitReached {
        /// Hard node limit.
        maximum: usize,
    },
    /// Branch hard limit was reached.
    BranchLimitReached {
        /// Hard branch limit.
        maximum: usize,
    },
    /// Retained encoded weight would exceed its hard limit.
    EncodedWeightLimitExceeded {
        /// Hard encoded-weight limit.
        maximum: u64,
        /// Requested resulting weight, or `u64::MAX` on overflow.
        requested: u64,
    },
    /// Revision could not advance.
    RevisionOverflow,
    /// Entry sequence could not advance.
    SequenceOverflow,
}

impl fmt::Display for ForkHistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "fork history revision {} is stale; current revision is {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicateEntry(id) => write!(formatter, "fork history entry {id} already exists"),
            Self::DuplicateBranch(id) => write!(formatter, "fork branch {id} already exists"),
            Self::DivergentBranchRequired => {
                formatter.write_str("divergent record requires a new branch seed")
            }
            Self::UnexpectedDivergentBranch => {
                formatter.write_str("non-divergent record cannot create a branch")
            }
            Self::UnknownBranch(id) => write!(formatter, "fork branch {id} does not exist"),
            Self::NodeLimitReached { maximum } => {
                write!(
                    formatter,
                    "fork history reached its {maximum}-node hard limit"
                )
            }
            Self::BranchLimitReached { maximum } => {
                write!(
                    formatter,
                    "fork history reached its {maximum}-branch hard limit"
                )
            }
            Self::EncodedWeightLimitExceeded { maximum, requested } => write!(
                formatter,
                "fork history encoded weight {requested} exceeds hard limit {maximum}"
            ),
            Self::RevisionOverflow => formatter.write_str("fork history revision cannot advance"),
            Self::SequenceOverflow => formatter.write_str("fork history sequence cannot advance"),
        }
    }
}

impl Error for ForkHistoryError {}

/// Invalid structural state rejected before it became authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForkHistoryStateError {
    /// State had no branch references.
    MissingBranch,
    /// State exceeded the node hard limit.
    TooManyNodes {
        /// Hard node limit.
        maximum: usize,
        /// Supplied node count.
        actual: usize,
    },
    /// State exceeded the branch hard limit.
    TooManyBranches {
        /// Hard branch limit.
        maximum: usize,
        /// Supplied branch count.
        actual: usize,
    },
    /// State exceeded the checkpoint hard limit.
    TooManyCheckpoints {
        /// Hard checkpoint limit.
        maximum: usize,
        /// Supplied checkpoint count.
        actual: usize,
    },
    /// Node identity appeared more than once.
    DuplicateNode(HistoryEntryId),
    /// Branch identity appeared more than once.
    DuplicateBranch(ForkBranchId),
    /// Node sequence appeared more than once.
    DuplicateSequence(u64),
    /// Node commit revision appeared more than once.
    DuplicateCommittedRevision(u64),
    /// Next sequence was not strictly after all retained nodes.
    InvalidNextSequence,
    /// A node revision was zero, after the graph revision, or not after its parent.
    InvalidCommittedRevision(HistoryEntryId),
    /// A node parent did not exist or was not structurally earlier.
    InvalidParent(HistoryEntryId),
    /// A branch head did not exist.
    InvalidBranchHead(ForkBranchId),
    /// Preferred-child relation was duplicated.
    DuplicatePreferredParent,
    /// Preferred child was not a direct child of its declared parent.
    InvalidPreferredChild(HistoryEntryId),
    /// A node with a choice of children declared no preference among them.
    ///
    /// Every forward walk in this crate -- redo, the default path, a
    /// continuation run -- follows preferred children and stops where there is
    /// none. A node with two or more children and no preference stops those
    /// walks early, so every one of them becomes unreachable and a projection
    /// reports a run's end where the graph has more. Recording and pruning
    /// both maintain the preference; only a hand-built state can omit it, so
    /// it is rejected here rather than left to surface as a fork nobody can
    /// open.
    ///
    /// A single child is not affected: there is nothing to prefer, and
    /// `preferred_child_id` resolves it without a recorded preference.
    ///
    /// `None` names the root, which has children like any other position.
    MissingPreferredChild(Option<HistoryEntryId>),
    /// Checkpoint identity appeared more than once.
    DuplicateCheckpoint(ForkCheckpointId),
    /// A checkpoint referenced an absent node.
    InvalidCheckpoint(ForkCheckpointId),
    /// Current branch did not exist.
    UnknownCurrentBranch(ForkBranchId),
    /// Current node did not exist or was outside the current branch lineage.
    InvalidCurrentNode,
    /// Retained encoded weight overflowed or exceeded its hard limit.
    InvalidEncodedWeight,
}

impl fmt::Display for ForkHistoryStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBranch => {
                formatter.write_str("fork history state has no branch references")
            }
            Self::TooManyNodes { maximum, actual } => write!(
                formatter,
                "fork history state holds {actual} nodes; hard limit is {maximum}"
            ),
            Self::TooManyBranches { maximum, actual } => write!(
                formatter,
                "fork history state holds {actual} branches; hard limit is {maximum}"
            ),
            Self::TooManyCheckpoints { maximum, actual } => write!(
                formatter,
                "fork history state holds {actual} checkpoints; hard limit is {maximum}"
            ),
            Self::DuplicateNode(id) => {
                write!(
                    formatter,
                    "fork history state lists node {id} more than once"
                )
            }
            Self::DuplicateBranch(id) => write!(
                formatter,
                "fork history state lists branch {id} more than once"
            ),
            Self::DuplicateSequence(sequence) => write!(
                formatter,
                "fork history state lists sequence {sequence} more than once"
            ),
            Self::DuplicateCommittedRevision(revision) => write!(
                formatter,
                "fork history state lists commit revision {revision} more than once"
            ),
            Self::InvalidNextSequence => formatter.write_str(
                "fork history state next sequence is not strictly after all retained nodes",
            ),
            Self::InvalidCommittedRevision(id) => {
                write!(
                    formatter,
                    "fork history entry {id} has an invalid commit revision"
                )
            }
            Self::InvalidParent(id) => {
                write!(formatter, "fork history entry {id} has an invalid parent")
            }
            Self::InvalidBranchHead(id) => {
                write!(formatter, "fork branch {id} has an invalid head")
            }
            Self::DuplicatePreferredParent => formatter
                .write_str("fork history state lists a preferred-child relation more than once"),
            Self::InvalidPreferredChild(id) => write!(
                formatter,
                "fork history entry {id} is not a direct child of its preferred parent"
            ),
            Self::MissingPreferredChild(None) => {
                formatter.write_str("fork history root declares no preferred child")
            }
            Self::MissingPreferredChild(Some(id)) => {
                write!(
                    formatter,
                    "fork history entry {id} declares no preferred child"
                )
            }
            Self::DuplicateCheckpoint(id) => write!(
                formatter,
                "fork history state lists checkpoint {id} more than once"
            ),
            Self::InvalidCheckpoint(id) => {
                write!(formatter, "fork checkpoint {id} references an absent node")
            }
            Self::UnknownCurrentBranch(id) => {
                write!(formatter, "fork history current branch {id} does not exist")
            }
            Self::InvalidCurrentNode => formatter.write_str(
                "fork history current node is absent or outside the current branch lineage",
            ),
            Self::InvalidEncodedWeight => formatter
                .write_str("fork history encoded weight overflowed or exceeds its hard limit"),
        }
    }
}

impl Error for ForkHistoryStateError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_id(value: &str) -> HistoryEntryId {
        HistoryEntryId::new(value).expect("fixture entry id")
    }

    fn branch_id(value: &str) -> ForkBranchId {
        ForkBranchId::new(value).expect("fixture branch id")
    }

    fn checkpoint_id(value: &str) -> ForkCheckpointId {
        ForkCheckpointId::new(value).expect("fixture checkpoint id")
    }

    #[test]
    fn fork_history_state_error_messages_are_hand_written() {
        let cases: [(ForkHistoryStateError, &str); 20] = [
            (
                ForkHistoryStateError::MissingBranch,
                "fork history state has no branch references",
            ),
            (
                ForkHistoryStateError::TooManyNodes {
                    maximum: 10,
                    actual: 11,
                },
                "fork history state holds 11 nodes; hard limit is 10",
            ),
            (
                ForkHistoryStateError::TooManyBranches {
                    maximum: 4,
                    actual: 5,
                },
                "fork history state holds 5 branches; hard limit is 4",
            ),
            (
                ForkHistoryStateError::TooManyCheckpoints {
                    maximum: 2,
                    actual: 3,
                },
                "fork history state holds 3 checkpoints; hard limit is 2",
            ),
            (
                ForkHistoryStateError::DuplicateNode(entry_id("entry:a")),
                "fork history state lists node entry:a more than once",
            ),
            (
                ForkHistoryStateError::DuplicateBranch(branch_id("branch:main")),
                "fork history state lists branch branch:main more than once",
            ),
            (
                ForkHistoryStateError::DuplicateSequence(7),
                "fork history state lists sequence 7 more than once",
            ),
            (
                ForkHistoryStateError::DuplicateCommittedRevision(9),
                "fork history state lists commit revision 9 more than once",
            ),
            (
                ForkHistoryStateError::InvalidNextSequence,
                "fork history state next sequence is not strictly after all retained nodes",
            ),
            (
                ForkHistoryStateError::InvalidCommittedRevision(entry_id("entry:b")),
                "fork history entry entry:b has an invalid commit revision",
            ),
            (
                ForkHistoryStateError::InvalidParent(entry_id("entry:c")),
                "fork history entry entry:c has an invalid parent",
            ),
            (
                ForkHistoryStateError::InvalidBranchHead(branch_id("branch:main")),
                "fork branch branch:main has an invalid head",
            ),
            (
                ForkHistoryStateError::DuplicatePreferredParent,
                "fork history state lists a preferred-child relation more than once",
            ),
            (
                ForkHistoryStateError::InvalidPreferredChild(entry_id("entry:d")),
                "fork history entry entry:d is not a direct child of its preferred parent",
            ),
            (
                ForkHistoryStateError::MissingPreferredChild(None),
                "fork history root declares no preferred child",
            ),
            (
                ForkHistoryStateError::MissingPreferredChild(Some(entry_id("entry:e"))),
                "fork history entry entry:e declares no preferred child",
            ),
            (
                ForkHistoryStateError::DuplicateCheckpoint(checkpoint_id("checkpoint:one")),
                "fork history state lists checkpoint checkpoint:one more than once",
            ),
            (
                ForkHistoryStateError::InvalidCheckpoint(checkpoint_id("checkpoint:two")),
                "fork checkpoint checkpoint:two references an absent node",
            ),
            (
                ForkHistoryStateError::UnknownCurrentBranch(branch_id("branch:gone")),
                "fork history current branch branch:gone does not exist",
            ),
            (
                ForkHistoryStateError::InvalidCurrentNode,
                "fork history current node is absent or outside the current branch lineage",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
        assert_eq!(
            ForkHistoryStateError::InvalidEncodedWeight.to_string(),
            "fork history encoded weight overflowed or exceeds its hard limit"
        );
    }
}
