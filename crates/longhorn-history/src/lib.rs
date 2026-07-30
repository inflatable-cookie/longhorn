//! Pure typed linear history state and consumer-owned payload policy seams.
//!
//! Product mutations must apply successfully before
//! [`LinearHistory::record_applied`] is called. This crate does not interpret
//! payloads, mutate product models, encode persistence, or own a clock.

mod entry;
mod identity;
mod limits;
mod policy;
mod state;

pub use entry::{
    AppliedHistoryRecord, HistoryEntry, HistoryEntryMetadata, HistoryLabel, HistoryLabelError,
    MAXIMUM_HISTORY_LABEL_BYTES,
};
pub use identity::{HistoryEntrySequence, HistoryEntrySequenceOverflow, HistoryEntrySequenceZero};
pub use limits::{HistoryLimits, HistoryLimitsError};
pub use policy::{HistoryCoalesce, HistoryPolicy};
pub use state::{
    HistoryRecordError, HistoryRecordOutcome, HistoryRecordResult, HistoryStateError,
    LinearHistory, LinearHistoryState,
};
