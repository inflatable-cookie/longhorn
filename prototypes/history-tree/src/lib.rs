//! Non-publishable forkable history evidence for Longhorn Card 068.
//!
//! This crate deliberately remains outside the public workspace and package
//! graph. Entry payload meaning and product application remain consumer-owned.

mod branch;
mod checkpoint;
mod graph;
mod identity;
mod navigation;
mod node;
mod persistence;
mod projection;
mod retention;

pub use branch::{
    ForkBranch, ForkBranchMetadata, ForkBranchMetadataError, ForkBranchSeed,
    MAXIMUM_FORK_BRANCH_ANNOTATION_BYTES, MAXIMUM_FORK_BRANCH_NAME_BYTES,
};
pub use checkpoint::{
    ForkCheckpoint, ForkCheckpointError, ForkCheckpointReceipt, ForkReplayCost,
    MAXIMUM_FORK_CHECKPOINT_REFERENCE_BYTES,
};
pub use graph::{ForkHistory, ForkHistoryError, ForkRecord, ForkRecordReceipt};
pub use identity::{ForkBranchId, ForkCheckpointId, ForkIdentityError};
pub use navigation::{
    ForkNavigationError, ForkNavigationPlan, ForkNavigationReceipt, ForkNavigationTarget,
    ForkNavigationTransaction,
};
pub use node::ForkHistoryNode;
pub use persistence::{
    CURRENT_FORK_HISTORY_STRUCTURAL_VERSION, FORK_HISTORY_FORMAT_FAMILY, ForkEncodeError,
    ForkLoadError, ForkLoadOutcome, ForkLoadReceipt, ForkLoadResult, ForkPayloadCodec,
    ForkPayloadMigrationStep, ForkPayloadMigrationTarget, ForkPersistence,
    ForkPersistenceValidationError, ForkStructuralMigration, ForkStructuralMigrationStep,
    ForkStructuralMigrationTarget, MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES, NoForkStructuralMigration,
};
pub use projection::{
    DerivedForkPath, ForkAlternateProjection, ForkBranchProjection, ForkLinearEntryProjection,
    ForkLinearProjection, ForkProjectionError,
};
pub use retention::{
    ForkPrunedNode, ForkPruningOutcome, ForkPruningReceipt, ForkRetentionError, ForkRetentionLimits,
};
