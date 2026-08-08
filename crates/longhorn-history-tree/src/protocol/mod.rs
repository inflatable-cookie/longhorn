//! Exact metadata-only fork-history protocol projections.

mod branch;
mod changed;
mod navigation;
mod path;
mod summary;

pub use branch::{ForkBranchPageCommand, ForkBranchPageSnapshot, ForkBranchRecord};
pub(crate) use changed::count;
pub use changed::{ForkChangedEvent, ForkChangedKind, ForkProtocolProjectionError};
pub use navigation::{
    ForkNavigationCommand, ForkNavigationReceiptProjection, ForkNavigationRejectionCode,
    ForkNavigationRejectionProjection, ForkNavigationResult, ForkNavigationTargetProjection,
};
pub use path::{
    ForkEntryRecord, ForkPathPageCommand, ForkPathPageSnapshot, ForkPathTargetProjection,
};
pub(crate) use summary::project_position;
pub use summary::{
    FORK_HISTORY_PROTOCOL_VERSION, ForkHistoryProtocolVersion, ForkProjectionPosition,
    ForkSnapshot, ForkSummaryProjection,
};
