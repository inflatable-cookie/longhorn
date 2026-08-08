//! Group lifecycle and grouped-record rejection errors.

use std::{error::Error, fmt};

use longhorn_core::HistoryGroupId;

use crate::HistoryRecordError;

use super::time::HistoryMonotonicMillis;

/// Rejected group lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryGroupError {
    /// Another group is already active.
    AlreadyOpen(HistoryGroupId),
    /// No group is active.
    NoActiveGroup,
    /// The caller named a different active group.
    WrongActiveGroup {
        /// Current active identity.
        expected: HistoryGroupId,
        /// Supplied identity.
        actual: HistoryGroupId,
    },
    /// A candidate identity already belongs to retained history.
    DuplicateGroupId(HistoryGroupId),
    /// Injected monotonic time regressed.
    TimeWentBackwards {
        /// Prior accepted reading.
        previous: HistoryMonotonicMillis,
        /// Supplied reading.
        actual: HistoryMonotonicMillis,
    },
}

impl fmt::Display for HistoryGroupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyOpen(group_id) => write!(formatter, "history group {group_id} is open"),
            Self::NoActiveGroup => formatter.write_str("history has no active group"),
            Self::WrongActiveGroup { expected, actual } => write!(
                formatter,
                "history group {actual} is not active; current group is {expected}"
            ),
            Self::DuplicateGroupId(group_id) => {
                write!(formatter, "history group id {group_id} is already retained")
            }
            Self::TimeWentBackwards { previous, actual } => write!(
                formatter,
                "history monotonic time regressed from {} to {}",
                previous.get(),
                actual.get()
            ),
        }
    }
}

impl Error for HistoryGroupError {}

/// Rejected grouped record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryGroupedRecordError<E> {
    /// Group lifecycle admission failed.
    Group(HistoryGroupError),
    /// Structural record admission failed.
    Record(HistoryRecordError<E>),
}

impl<E: fmt::Display> fmt::Display for HistoryGroupedRecordError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(error) => write!(formatter, "history group rejected record: {error}"),
            Self::Record(error) => error.fmt(formatter),
        }
    }
}

impl<E> Error for HistoryGroupedRecordError<E> where E: Error + 'static {}
