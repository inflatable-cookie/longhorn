//! Explicit and timed grouping admission on [`LinearHistory`].

use longhorn_core::HistoryGroupId;

use crate::{AppliedHistoryRecord, HistoryPolicy, LinearHistory};

use super::{
    HistoryActiveGroup, HistoryActiveGroupMode, HistoryGroupCloseReason, HistoryGroupClosure,
    HistoryGroupError, HistoryGroupedRecordError, HistoryGroupedRecordResult,
    HistoryTimedGroupRequest,
};

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
