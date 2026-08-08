//! Transient explicit and timed history grouping.

mod error;
mod record;
mod time;
mod types;

pub use error::{HistoryGroupError, HistoryGroupedRecordError};
pub use time::{HistoryGroupDurationError, HistoryGroupDurationMillis, HistoryMonotonicMillis};
pub use types::{
    HistoryActiveGroup, HistoryActiveGroupMode, HistoryGroupCloseReason, HistoryGroupClosure,
    HistoryGroupedRecordResult, HistoryTimedGroupRequest,
};
