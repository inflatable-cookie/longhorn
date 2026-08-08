//! Validated mutable fork-history graph state.

mod history;
mod model;
mod record;
mod validate;

pub use history::ForkHistory;
pub use model::{ForkHistoryState, ForkPreferredChild, MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES};
pub use record::{ForkBranchUpdateReceipt, ForkRecord, ForkRecordReceipt};
pub(crate) use validate::{branch_contains, build_children, validate_counts, validate_nodes};
