//! Optional immutable-node fork history authority.
//!
//! Product mutations must apply successfully before [`ForkHistory::record_applied`]
//! is called. This crate owns graph topology and branch references, not product
//! models, storage durability, clocks, or project-version identity.

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
    ForkBranchPage, ForkBranchProjection, ForkEntryProjection, ForkPathPage, ForkProjectionError,
    ForkProjectionPageRequest, ForkSummary, MAXIMUM_FORK_PROJECTION_PAGE_SIZE,
};
pub use protocol::{
    FORK_HISTORY_PROTOCOL_VERSION, ForkBranchPageCommand, ForkBranchPageSnapshot, ForkBranchRecord,
    ForkChangedEvent, ForkChangedKind, ForkEntryRecord, ForkHistoryProtocolVersion,
    ForkNavigationCommand, ForkNavigationReceiptProjection, ForkNavigationRejectionCode,
    ForkNavigationRejectionProjection, ForkNavigationResult, ForkNavigationTargetProjection,
    ForkPathPageCommand, ForkPathPageSnapshot, ForkPathTargetProjection, ForkProjectionPosition,
    ForkProtocolProjectionError, ForkSnapshot, ForkSummaryProjection,
};
pub use retention::{
    ForkPrunedNode, ForkPruningOutcome, ForkPruningReceipt, ForkRetentionError, ForkRetentionLimits,
};
pub use state::{
    ForkBranchUpdateReceipt, ForkHistory, ForkHistoryState, ForkPreferredChild, ForkRecord,
    ForkRecordReceipt, MAXIMUM_FORK_BRANCHES, MAXIMUM_FORK_NODES,
};
