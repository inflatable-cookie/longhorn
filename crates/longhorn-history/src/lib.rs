//! Pure typed linear history state and consumer-owned payload policy seams.
//!
//! Product mutations must apply successfully before
//! [`LinearHistory::record_applied`] is called. This crate does not interpret
//! payloads, mutate product models, choose persistence paths, or own a clock.

mod entry;
mod group;
mod identity;
mod limits;
mod navigation;
mod persistence;
mod policy;
mod projection;
mod protocol;
mod retention;
mod state;
mod transition;

pub use entry::{
    AppliedHistoryRecord, HistoryEntry, HistoryEntryMetadata, HistoryLabel, HistoryLabelError,
    HistoryRecordedAt, MAXIMUM_HISTORY_LABEL_BYTES,
};
pub use group::{
    HistoryActiveGroup, HistoryActiveGroupMode, HistoryGroupCloseReason, HistoryGroupClosure,
    HistoryGroupDurationError, HistoryGroupDurationMillis, HistoryGroupError,
    HistoryGroupedRecordError, HistoryGroupedRecordResult, HistoryMonotonicMillis,
    HistoryTimedGroupRequest,
};
pub use identity::{HistoryEntrySequence, HistoryEntrySequenceOverflow, HistoryEntrySequenceZero};
pub use limits::{
    HistoryLimits, HistoryLimitsError, HistoryNavigationLimits, HistoryNavigationLimitsError,
    HistoryProjectionLimits, HistoryProjectionLimitsError, MAXIMUM_HISTORY_ENCODED_WEIGHT,
    MAXIMUM_HISTORY_NAVIGATION_STEPS, MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
    MAXIMUM_RECENT_HISTORY_PLANS,
};
pub use navigation::{
    HistoryNavigationDirection, HistoryNavigationExecutionError, HistoryNavigationPlan,
    HistoryNavigationPlanningError, HistoryNavigationPosition, HistoryNavigationReceipt,
    HistoryNavigationRejection, HistoryNavigationRequest, HistoryNavigationStep,
    HistoryNavigationTarget, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
};
pub use persistence::{
    CURRENT_HISTORY_STRUCTURAL_VERSION, HISTORY_FORMAT_FAMILY, HistoryDiscardRecovery,
    HistoryDiscardRecoveryReceipt, HistoryEncodeError, HistoryLoadAttempt, HistoryLoadError,
    HistoryLoadOutcome, HistoryLoadReceipt, HistoryLoadResult, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecFamilyError, HistoryPayloadCodecVersion,
    HistoryPayloadMigrationStep, HistoryPayloadMigrationTarget, HistoryPersistence,
    HistoryPersistenceLimits, HistoryPersistenceLimitsError, HistoryStructuralHeaderError,
    HistoryStructuralMigration, HistoryStructuralMigrationStep, HistoryStructuralMigrationTarget,
    MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES, MAXIMUM_HISTORY_PERSISTED_BYTES,
    NoHistoryStructuralMigration, discard_persisted_history,
};
pub use policy::{HistoryCoalesce, HistoryCoalesceContext, HistoryPolicy};
pub use projection::{
    HistoryEntryPosition, HistoryEntryProjection, HistoryMode, HistoryPage, HistoryPageRequest,
    HistoryPageRequestError, HistoryProjectionError, HistorySummary,
};
pub use protocol::{
    HISTORY_PROTOCOL_VERSION, HistoryAuthorityEpoch, HistoryAuthorityEpochError,
    HistoryBaselineProjection, HistoryChangedEvent, HistoryChangedKind, HistoryEntryRecord,
    HistoryNavigationCommand, HistoryNavigationDirectionProjection,
    HistoryNavigationPositionProjection, HistoryNavigationReceiptProjection,
    HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection, HistoryNavigationResult,
    HistoryNavigationTargetProjection, HistoryPageCommand, HistoryPageSnapshot,
    HistoryProjectionPosition, HistoryProtocolMode, HistoryProtocolProjectionError,
    HistoryProtocolVersion, HistorySnapshot, HistorySummaryProjection,
};
pub use retention::{
    HistoryLimitChangeError, HistoryLimitChangeReceipt, HistoryPrunedEntry, HistoryPruningReceipt,
    HistoryRetainedBaseline, HistoryRetentionError,
};
pub use state::{
    HistoryRecordError, HistoryRecordOutcome, HistoryRecordResult, HistoryStateError,
    LinearHistory, LinearHistoryState,
};
pub use transition::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryDiscardReason,
    HistoryRecordTransitionEffect, HistoryResetError, HistoryResetReceipt,
};
