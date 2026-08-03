//! Optional immutable-node fork history authority.
//!
//! Product mutations must apply successfully before [`ForkHistory::record_applied`]
//! is called. This crate owns graph topology and branch references, not product
//! models, persistence, clocks, or project-version identity.

mod branch;
mod error;
mod identity;
mod node;
mod state;

pub use branch::{
    ForkBranch, ForkBranchMetadata, ForkBranchMetadataError, ForkBranchSeed,
    MAXIMUM_FORK_BRANCH_ANNOTATION_BYTES, MAXIMUM_FORK_BRANCH_NAME_BYTES,
};
pub use error::{ForkHistoryError, ForkHistoryStateError};
pub use identity::{ForkBranchId, ForkIdentityError, MAXIMUM_FORK_ID_BYTES};
pub use node::ForkHistoryNode;
pub use state::{
    ForkBranchUpdateReceipt, ForkHistory, ForkHistoryState, ForkPreferredChild, ForkRecord,
    ForkRecordReceipt, MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES,
};
