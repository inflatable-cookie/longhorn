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
        write!(formatter, "invalid fork history state: {self:?}")
    }
}

impl Error for ForkHistoryStateError {}
