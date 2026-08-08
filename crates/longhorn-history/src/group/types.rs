//! Timed request, active group, closure, and grouped-record result types.

use longhorn_core::{HistoryGroupId, HistoryGroupKeyId};

use crate::HistoryRecordResult;

use super::time::{HistoryGroupDurationMillis, HistoryMonotonicMillis};

/// One timed grouping request with injected identity and time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryTimedGroupRequest {
    pub(crate) candidate_group_id: HistoryGroupId,
    pub(crate) key: HistoryGroupKeyId,
    pub(crate) now: HistoryMonotonicMillis,
    pub(crate) maximum_gap: HistoryGroupDurationMillis,
}

impl HistoryTimedGroupRequest {
    /// Constructs a timed group request.
    #[must_use]
    pub const fn new(
        candidate_group_id: HistoryGroupId,
        key: HistoryGroupKeyId,
        now: HistoryMonotonicMillis,
        maximum_gap: HistoryGroupDurationMillis,
    ) -> Self {
        Self {
            candidate_group_id,
            key,
            now,
            maximum_gap,
        }
    }

    /// Returns the candidate identity used only when a new group opens.
    #[must_use]
    pub const fn candidate_group_id(&self) -> &HistoryGroupId {
        &self.candidate_group_id
    }

    /// Returns the consumer-owned grouping key.
    #[must_use]
    pub const fn key(&self) -> &HistoryGroupKeyId {
        &self.key
    }

    /// Returns the injected monotonic reading.
    #[must_use]
    pub const fn now(&self) -> HistoryMonotonicMillis {
        self.now
    }

    /// Returns the consumer-selected maximum gap.
    #[must_use]
    pub const fn maximum_gap(&self) -> HistoryGroupDurationMillis {
        self.maximum_gap
    }
}

/// Transient active grouping mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryActiveGroupMode {
    /// Explicit lifecycle controlled by open and close calls.
    Explicit,
    /// Timed lifecycle controlled by injected readings.
    Timed {
        /// Consumer-owned compatibility key.
        key: HistoryGroupKeyId,
        /// Last successfully recorded activity.
        last_activity: HistoryMonotonicMillis,
        /// Consumer-selected maximum gap.
        maximum_gap: HistoryGroupDurationMillis,
    },
}

/// Transient active group state; never part of structural persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryActiveGroup {
    pub(crate) group_id: HistoryGroupId,
    pub(crate) mode: HistoryActiveGroupMode,
}

impl HistoryActiveGroup {
    /// Returns the active group identity.
    #[must_use]
    pub const fn group_id(&self) -> &HistoryGroupId {
        &self.group_id
    }

    /// Returns the active grouping mode.
    #[must_use]
    pub const fn mode(&self) -> &HistoryActiveGroupMode {
        &self.mode
    }
}

/// Why transient grouping ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryGroupCloseReason {
    /// Caller closed a completed gesture.
    Closed,
    /// Caller cancelled a gesture.
    Cancelled,
    /// Injected time exceeded the consumer-selected gap.
    TimedOut,
    /// A different timed key or duration replaced the active group.
    Replaced,
    /// A committed navigation ended coalescing continuity.
    Navigation,
    /// Limits or another structural authority setting changed.
    AuthorityChange,
    /// Host teardown discarded transient state.
    Teardown,
}

/// Exact transient group closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryGroupClosure {
    pub(crate) group_id: HistoryGroupId,
    pub(crate) reason: HistoryGroupCloseReason,
}

impl HistoryGroupClosure {
    /// Returns the closed group identity.
    #[must_use]
    pub const fn group_id(&self) -> &HistoryGroupId {
        &self.group_id
    }

    /// Returns why the group closed.
    #[must_use]
    pub const fn reason(&self) -> HistoryGroupCloseReason {
        self.reason
    }
}

/// Record result plus the authoritative group selected by the core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryGroupedRecordResult {
    pub(crate) group_id: HistoryGroupId,
    pub(crate) opened_new_group: bool,
    pub(crate) closed_group: Option<HistoryGroupClosure>,
    pub(crate) record: HistoryRecordResult,
}

impl HistoryGroupedRecordResult {
    /// Returns the committed group identity.
    #[must_use]
    pub const fn group_id(&self) -> &HistoryGroupId {
        &self.group_id
    }

    /// Returns whether this record opened a fresh group.
    #[must_use]
    pub const fn opened_new_group(&self) -> bool {
        self.opened_new_group
    }

    /// Returns a timed group ended by this record, if any.
    #[must_use]
    pub const fn closed_group(&self) -> Option<&HistoryGroupClosure> {
        self.closed_group.as_ref()
    }

    /// Returns the structural record result.
    #[must_use]
    pub const fn record(&self) -> &HistoryRecordResult {
        &self.record
    }
}
