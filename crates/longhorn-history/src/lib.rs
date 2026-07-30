//! Pure typed linear history state and consumer-owned payload policy seams.
//!
//! Product mutations must apply successfully before
//! [`LinearHistory::record_applied`] is called. This crate does not interpret
//! payloads, mutate product models, encode persistence, or own a clock.

mod entry;
mod group;
mod identity;
mod limits;
mod navigation;
mod policy;
mod projection;
mod retention;
mod state;

pub use entry::{
    AppliedHistoryRecord, HistoryEntry, HistoryEntryMetadata, HistoryLabel, HistoryLabelError,
    MAXIMUM_HISTORY_LABEL_BYTES,
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
pub use policy::{HistoryCoalesce, HistoryCoalesceContext, HistoryPolicy};
pub use projection::{
    HistoryEntryPosition, HistoryEntryProjection, HistoryMode, HistoryPage, HistoryPageRequest,
    HistoryPageRequestError, HistoryProjectionError, HistorySummary,
};
pub use retention::{
    HistoryLimitChangeError, HistoryLimitChangeReceipt, HistoryPrunedEntry, HistoryPruningReceipt,
    HistoryRetainedBaseline, HistoryRetentionError,
};
pub use state::{
    HistoryRecordError, HistoryRecordOutcome, HistoryRecordResult, HistoryStateError,
    LinearHistory, LinearHistoryState,
};
