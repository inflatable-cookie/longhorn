//! Optional immutable-node fork history authority.
//!
//! Product mutations must apply successfully before [`ForkHistory::record_applied`]
//! is called. This crate owns graph topology and branch references, not product
//! models, storage durability, clocks, or project-version identity.
//!
//! # Keeping a graph inside a budget
//!
//! A fork history grows on every divergence. Two operations shrink one, and
//! **this crate schedules neither** -- it has no clock and no runtime, so
//! every trigger is the host's.
//!
//! - [`ForkHistory::delete_continuation`] removes one fork the operator named,
//!   and everything below it. Irreversible.
//! - [`ForkHistory::prune_to`] removes the oldest unprotected leaves until the
//!   unprotected share fits a budget.
//!
//! [`ForkRetentionLimits`] bounds the **unprotected** share: everything not on
//! the current branch and not on a pinned branch. A protected entry is a core
//! record of the project, so it is never counted against a budget nor removed
//! by one, and a graph with a large pinned set exceeds any budget without that
//! being an error.
//!
//! Three triggers are usual -- after a record, on a timer, and on an explicit
//! operator action -- and an app opts into each by calling, or out by not
//! calling. Nothing here has to be configured for that:
//! [`ForkSummaryProjection::retained_entry_count`] and
//! [`ForkSummaryProjection::retained_encoded_weight`] report the pressure after
//! any change, so a host decides for itself.

mod branch;
mod checkpoint;
mod error;
mod identity;
mod navigation;
mod node;
mod persistence;
mod projection;
mod protocol;
mod retention;
mod state;

pub use branch::{
    ForkBranch, ForkBranchMetadata, ForkBranchMetadataError, ForkBranchSeed,
    MAXIMUM_FORK_BRANCH_ANNOTATION_BYTES, MAXIMUM_FORK_BRANCH_NAME_BYTES,
};
pub use checkpoint::{
    ForkCheckpoint, ForkCheckpointError, ForkCheckpointReceipt, ForkReplayCost,
    MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES, MAXIMUM_FORK_CHECKPOINTS,
};
pub use error::{ForkHistoryError, ForkHistoryStateError};
pub use identity::{ForkBranchId, ForkCheckpointId, ForkIdentityError, MAXIMUM_FORK_ID_BYTES};
pub use navigation::{
    ForkNavigationError, ForkNavigationPlan, ForkNavigationReceipt, ForkNavigationTarget,
    ForkNavigationTransaction,
};
pub use node::ForkHistoryNode;
pub use persistence::{
    CURRENT_FORK_HISTORY_STRUCTURAL_VERSION, FORK_HISTORY_FORMAT_FAMILY, ForkEncodeError,
    ForkLoadError, ForkLoadOutcome, ForkLoadReceipt, ForkLoadResult, ForkPersistence,
    ForkPersistenceLimits, ForkPersistenceLimitsError, ForkStructuralMigration,
    ForkStructuralMigrationStep, ForkStructuralMigrationTarget,
    MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES, NoForkStructuralMigration,
};
pub use projection::{
    ForkBranchPage, ForkBranchProjection, ForkContinuation, ForkContinuationPage,
    ForkEntryProjection, ForkPathPage, ForkProjectionError, ForkProjectionPageRequest, ForkSummary,
    MAXIMUM_FORK_PROJECTION_PAGE_SIZE,
};
pub use protocol::{
    FORK_HISTORY_PROTOCOL_VERSION, ForkBranchPageCommand, ForkBranchPageSnapshot, ForkBranchRecord,
    ForkChangedEvent, ForkChangedKind, ForkContinuationPageCommand, ForkContinuationPageSnapshot,
    ForkContinuationRecord, ForkDeleteContinuationCommand, ForkEntryRecord,
    ForkHistoryProtocolVersion, ForkNavigationCommand, ForkNavigationReceiptProjection,
    ForkNavigationRejectionCode, ForkNavigationRejectionProjection, ForkNavigationResult,
    ForkNavigationTargetProjection, ForkPathFloorProjection, ForkPathPageCommand,
    ForkPathPageSnapshot, ForkPathTargetProjection, ForkProjectionPosition,
    ForkProtocolProjectionError, ForkPruneCommand, ForkPruneResult, ForkRemovalReceiptProjection,
    ForkRemovedEntryRecord, ForkSnapshot, ForkSummaryProjection,
};
pub use retention::{
    ForkPrunedNode, ForkPruningOutcome, ForkPruningReceipt, ForkRetentionError, ForkRetentionLimits,
};
pub use state::{
    ForkBranchUpdateReceipt, ForkHistory, ForkHistoryState, ForkPreferredChild, ForkRecord,
    ForkRecordReceipt, MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES,
};
