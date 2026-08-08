//! Validated mutable linear history state.

mod model;
mod record;
mod result;

pub use model::{LinearHistory, LinearHistoryState};
pub use result::{
    HistoryRecordError, HistoryRecordOutcome, HistoryRecordResult, HistoryStateError,
};
