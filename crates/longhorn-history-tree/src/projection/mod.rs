//! Bounded fork-history metadata projections.

mod error;
mod project;
mod support;
mod types;

pub use error::ForkProjectionError;
pub(crate) use support::check_offset;
pub use types::{
    ForkBranchPage, ForkBranchProjection, ForkContinuation, ForkContinuationPage,
    ForkEntryProjection, ForkPathPage, ForkProjectionPageRequest, ForkSummary,
    MAXIMUM_FORK_PROJECTION_PAGE_SIZE,
};
