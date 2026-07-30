use std::{error::Error, fmt};

use longhorn_core::{HistoryGroupId, HistoryGroupKeyId};

use crate::{
    AppliedHistoryRecord, HistoryPolicy, HistoryRecordError, HistoryRecordResult, LinearHistory,
};

/// Caller-injected monotonic millisecond reading.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HistoryMonotonicMillis(u64);

impl HistoryMonotonicMillis {
    /// Constructs an injected monotonic reading.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the injected reading.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Nonzero consumer-selected timed-group gap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryGroupDurationMillis(u64);

impl HistoryGroupDurationMillis {
    /// Validates a consumer-selected duration.
    pub const fn new(value: u64) -> Result<Self, HistoryGroupDurationError> {
        if value == 0 {
            return Err(HistoryGroupDurationError);
        }
        Ok(Self(value))
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A zero timed-group duration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryGroupDurationError;

impl fmt::Display for HistoryGroupDurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("history group duration must be nonzero")
    }
}

impl Error for HistoryGroupDurationError {}

/// One timed grouping request with injected identity and time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryTimedGroupRequest {
    candidate_group_id: HistoryGroupId,
    key: HistoryGroupKeyId,
    now: HistoryMonotonicMillis,
    maximum_gap: HistoryGroupDurationMillis,
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
    group_id: HistoryGroupId,
    mode: HistoryActiveGroupMode,
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
    group_id: HistoryGroupId,
    reason: HistoryGroupCloseReason,
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
    group_id: HistoryGroupId,
    opened_new_group: bool,
    closed_group: Option<HistoryGroupClosure>,
    record: HistoryRecordResult,
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

impl<P> LinearHistory<P> {
    /// Returns transient active grouping state.
    #[must_use]
    pub const fn active_group(&self) -> Option<&HistoryActiveGroup> {
        self.active_group.as_ref()
    }

    /// Opens one explicit group. A previous group must close first.
    pub fn open_group(&mut self, group_id: HistoryGroupId) -> Result<(), HistoryGroupError> {
        if let Some(active) = &self.active_group {
            return Err(HistoryGroupError::AlreadyOpen(active.group_id.clone()));
        }
        self.ensure_fresh_group_id(&group_id)?;
        self.active_group = Some(HistoryActiveGroup {
            group_id,
            mode: HistoryActiveGroupMode::Explicit,
        });
        self.coalescing_open = false;
        Ok(())
    }

    /// Records into the exact active explicit or timed group.
    pub fn record_in_group<T>(
        &mut self,
        record: AppliedHistoryRecord<P>,
        group_id: &HistoryGroupId,
        policy: &T,
    ) -> Result<HistoryGroupedRecordResult, HistoryGroupedRecordError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        let active = self
            .active_group
            .as_ref()
            .ok_or(HistoryGroupedRecordError::Group(
                HistoryGroupError::NoActiveGroup,
            ))?;
        if active.group_id != *group_id {
            return Err(HistoryGroupedRecordError::Group(
                HistoryGroupError::WrongActiveGroup {
                    expected: active.group_id.clone(),
                    actual: group_id.clone(),
                },
            ));
        }
        let result = self
            .record_applied_with_group(record, Some(group_id.clone()), self.coalescing_open, policy)
            .map_err(HistoryGroupedRecordError::Record)?;
        Ok(HistoryGroupedRecordResult {
            group_id: group_id.clone(),
            opened_new_group: false,
            closed_group: None,
            record: result,
        })
    }

    /// Records through a timed group selected only from injected evidence.
    pub fn record_timed<T>(
        &mut self,
        record: AppliedHistoryRecord<P>,
        request: HistoryTimedGroupRequest,
        policy: &T,
    ) -> Result<HistoryGroupedRecordResult, HistoryGroupedRecordError<T::Error>>
    where
        T: HistoryPolicy<P>,
    {
        let (group_id, opened_new_group, may_coalesce, closed_group) = match &self.active_group {
            Some(HistoryActiveGroup {
                mode: HistoryActiveGroupMode::Explicit,
                group_id,
            }) => {
                return Err(HistoryGroupedRecordError::Group(
                    HistoryGroupError::AlreadyOpen(group_id.clone()),
                ));
            }
            Some(HistoryActiveGroup {
                group_id,
                mode:
                    HistoryActiveGroupMode::Timed {
                        key,
                        last_activity,
                        maximum_gap,
                    },
            }) => {
                let elapsed = request.now.get().checked_sub(last_activity.get()).ok_or(
                    HistoryGroupedRecordError::Group(HistoryGroupError::TimeWentBackwards {
                        previous: *last_activity,
                        actual: request.now,
                    }),
                )?;
                if key == &request.key
                    && maximum_gap == &request.maximum_gap
                    && elapsed < request.maximum_gap.get()
                {
                    (group_id.clone(), false, self.coalescing_open, None)
                } else {
                    self.ensure_fresh_group_id(&request.candidate_group_id)
                        .map_err(HistoryGroupedRecordError::Group)?;
                    let reason = if key == &request.key
                        && maximum_gap == &request.maximum_gap
                        && elapsed >= request.maximum_gap.get()
                    {
                        HistoryGroupCloseReason::TimedOut
                    } else {
                        HistoryGroupCloseReason::Replaced
                    };
                    (
                        request.candidate_group_id.clone(),
                        true,
                        false,
                        Some(HistoryGroupClosure {
                            group_id: group_id.clone(),
                            reason,
                        }),
                    )
                }
            }
            None => {
                self.ensure_fresh_group_id(&request.candidate_group_id)
                    .map_err(HistoryGroupedRecordError::Group)?;
                (request.candidate_group_id.clone(), true, false, None)
            }
        };

        let result = self
            .record_applied_with_group(record, Some(group_id.clone()), may_coalesce, policy)
            .map_err(HistoryGroupedRecordError::Record)?;
        self.active_group = Some(HistoryActiveGroup {
            group_id: group_id.clone(),
            mode: HistoryActiveGroupMode::Timed {
                key: request.key,
                last_activity: request.now,
                maximum_gap: request.maximum_gap,
            },
        });
        Ok(HistoryGroupedRecordResult {
            group_id,
            opened_new_group,
            closed_group,
            record: result,
        })
    }

    /// Closes the exact active group.
    pub fn close_group(
        &mut self,
        group_id: &HistoryGroupId,
    ) -> Result<HistoryGroupClosure, HistoryGroupError> {
        self.close_expected_group(group_id, HistoryGroupCloseReason::Closed)
    }

    /// Cancels the exact active group without changing committed entries.
    pub fn cancel_group(
        &mut self,
        group_id: &HistoryGroupId,
    ) -> Result<HistoryGroupClosure, HistoryGroupError> {
        self.close_expected_group(group_id, HistoryGroupCloseReason::Cancelled)
    }

    /// Ends all transient grouping for host teardown.
    pub fn teardown_transient_state(&mut self) -> Option<HistoryGroupClosure> {
        self.close_transient_group(HistoryGroupCloseReason::Teardown)
    }

    fn close_expected_group(
        &mut self,
        group_id: &HistoryGroupId,
        reason: HistoryGroupCloseReason,
    ) -> Result<HistoryGroupClosure, HistoryGroupError> {
        let active = self
            .active_group
            .as_ref()
            .ok_or(HistoryGroupError::NoActiveGroup)?;
        if active.group_id != *group_id {
            return Err(HistoryGroupError::WrongActiveGroup {
                expected: active.group_id.clone(),
                actual: group_id.clone(),
            });
        }
        Ok(self
            .close_transient_group(reason)
            .expect("active group checked above"))
    }

    fn ensure_fresh_group_id(&self, group_id: &HistoryGroupId) -> Result<(), HistoryGroupError> {
        if self
            .state
            .applied
            .iter()
            .chain(&self.state.future)
            .any(|entry| entry.metadata().group_id() == Some(group_id))
        {
            return Err(HistoryGroupError::DuplicateGroupId(group_id.clone()));
        }
        Ok(())
    }

    pub(crate) fn close_transient_group(
        &mut self,
        reason: HistoryGroupCloseReason,
    ) -> Option<HistoryGroupClosure> {
        self.coalescing_open = false;
        self.active_group.take().map(|active| HistoryGroupClosure {
            group_id: active.group_id,
            reason,
        })
    }
}

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
