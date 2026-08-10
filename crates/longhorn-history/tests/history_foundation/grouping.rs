use std::convert::Infallible;

use longhorn_core::{HistoryGroupId, HistoryGroupKeyId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryGroupCloseReason, HistoryGroupDurationMillis,
    HistoryGroupedRecordError, HistoryLimits, HistoryMonotonicMillis, HistoryNavigationTarget,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure, HistoryPolicy,
    HistoryRecordError, HistoryTimedGroupRequest, LinearHistory,
};

use crate::{
    pulse_shaped::{PulseFixtureMutation, PulseFixturePolicy, rename},
    support::*,
};

fn timed(group: &str, key: &str, now_ms: u64, duration_ms: u64) -> HistoryTimedGroupRequest {
    HistoryTimedGroupRequest::new(
        HistoryGroupId::new(group).unwrap(),
        HistoryGroupKeyId::new(key).unwrap(),
        HistoryMonotonicMillis::new(now_ms),
        HistoryGroupDurationMillis::new(duration_ms).unwrap(),
    )
}

#[test]
fn pulse_shaped_timed_group_uses_injected_750_ms_and_compound_policy() {
    let mut history = LinearHistory::new(history_id("history:pulse"), HistoryLimits::default());
    let policy = PulseFixturePolicy;
    let first = history
        .record_timed(
            record(
                0,
                "entry:1",
                metadata("Adjust track", "track:gesture"),
                rename(7, "A", "B"),
            ),
            timed("group:1", "gesture:track", 1_000, 750),
            &policy,
        )
        .unwrap();
    assert!(first.opened_new_group());
    assert_eq!(first.group_id().as_str(), "group:1");

    let continued = history
        .record_timed(
            record(
                1,
                "entry:2",
                metadata("Adjust track", "track:gesture"),
                PulseFixtureMutation::DeleteTrack {
                    track_id: 9,
                    snapshot: "FX".to_owned(),
                },
            ),
            timed("group:unused", "gesture:track", 1_749, 750),
            &policy,
        )
        .unwrap();
    assert!(!continued.opened_new_group());
    assert_eq!(continued.group_id().as_str(), "group:1");
    assert_eq!(history.applied().len(), 1);
    assert!(matches!(
        history.current().unwrap().payload(),
        PulseFixtureMutation::Compound { mutations } if mutations.len() == 2
    ));

    let expired = history
        .record_timed(
            record(
                2,
                "entry:3",
                metadata("Adjust track again", "track:gesture"),
                rename(7, "B", "C"),
            ),
            timed("group:2", "gesture:track", 2_499, 750),
            &policy,
        )
        .unwrap();
    assert!(expired.opened_new_group());
    assert_eq!(
        expired.closed_group().unwrap().reason(),
        HistoryGroupCloseReason::TimedOut
    );
    assert_eq!(history.applied().len(), 2);

    let before = history.clone();
    assert!(matches!(
        history.record_timed(
            record(
                3,
                "entry:4",
                metadata("Regressed time", "track:gesture"),
                rename(7, "C", "D"),
            ),
            timed("group:3", "gesture:track", 2_498, 750),
            &policy,
        ),
        Err(HistoryGroupedRecordError::Group(
            longhorn_history::HistoryGroupError::TimeWentBackwards { .. }
        ))
    ));
    assert_eq!(history, before);
}

struct NonAtomicGroupPolicy;

impl HistoryPolicy<PulseFixtureMutation> for NonAtomicGroupPolicy {
    type Error = crate::pulse_shaped::PulseFixturePolicyError;

    fn inverse(&self, payload: &PulseFixtureMutation) -> Result<PulseFixtureMutation, Self::Error> {
        PulseFixturePolicy.inverse(payload)
    }

    fn is_noop(&self, payload: &PulseFixtureMutation) -> bool {
        PulseFixturePolicy.is_noop(payload)
    }

    fn encoded_weight(&self, _: &PulseFixtureMutation) -> Result<u64, Self::Error> {
        Ok(1)
    }

    fn coalesce(
        &self,
        _: &PulseFixtureMutation,
        _: &PulseFixtureMutation,
        _: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<PulseFixtureMutation>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[test]
fn continuing_group_requires_one_atomic_consumer_payload() {
    let mut history =
        LinearHistory::new(history_id("history:atomic-group"), HistoryLimits::default());
    let group = HistoryGroupId::new("group:atomic").unwrap();
    history.open_group(group.clone()).unwrap();
    history
        .record_in_group(
            record(
                0,
                "entry:1",
                metadata("First", "track:gesture"),
                rename(1, "A", "B"),
            ),
            &group,
            &NonAtomicGroupPolicy,
        )
        .unwrap();
    let before = history.clone();
    assert!(matches!(
        history.record_in_group(
            record(
                1,
                "entry:2",
                metadata("Second", "track:gesture"),
                rename(1, "B", "C"),
            ),
            &group,
            &NonAtomicGroupPolicy,
        ),
        Err(HistoryGroupedRecordError::Record(
            HistoryRecordError::GroupPolicyKeptSeparate
        ))
    ));
    assert_eq!(history, before);
}

#[test]
fn explicit_close_cancel_teardown_and_restore_end_coalescing_continuity() {
    let mut history = LinearHistory::new(history_id("history:groups"), HistoryLimits::default());
    let policy = PulseFixturePolicy;
    let group_1 = HistoryGroupId::new("group:1").unwrap();
    history.open_group(group_1.clone()).unwrap();
    history
        .record_in_group(
            record(
                0,
                "entry:1",
                metadata("Rename", "track:rename"),
                rename(1, "A", "B"),
            ),
            &group_1,
            &policy,
        )
        .unwrap();
    assert_eq!(
        history.close_group(&group_1).unwrap().reason(),
        HistoryGroupCloseReason::Closed
    );
    history
        .record_applied(
            record(
                1,
                "entry:2",
                metadata("Rename again", "track:rename"),
                rename(1, "B", "C"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(history.applied().len(), 2);

    let group_2 = HistoryGroupId::new("group:2").unwrap();
    history.open_group(group_2.clone()).unwrap();
    let before = history.clone();
    assert_eq!(
        history.record_applied(
            record(
                2,
                "entry:blocked",
                metadata("Blocked", "track:rename"),
                rename(1, "C", "D"),
            ),
            &policy,
        ),
        Err(HistoryRecordError::ActiveGroupOpen(group_2.clone()))
    );
    assert_eq!(history, before);
    assert_eq!(
        history.cancel_group(&group_2).unwrap().reason(),
        HistoryGroupCloseReason::Cancelled
    );
    let group_3 = HistoryGroupId::new("group:3").unwrap();
    history.open_group(group_3.clone()).unwrap();
    assert_eq!(
        history.teardown_transient_state().unwrap().reason(),
        HistoryGroupCloseReason::Teardown
    );
    assert!(history.active_group().is_none());

    history.open_group(group_3).unwrap();
    let structural = history.into_state();
    let mut restored = LinearHistory::from_state(HistoryLimits::default(), structural).unwrap();
    assert!(restored.active_group().is_none());
    restored
        .record_applied(
            record(
                2,
                "entry:3",
                metadata("Rename after restore", "track:rename"),
                rename(1, "C", "D"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(restored.applied().len(), 3);
}

struct SuccessfulTransaction;

impl HistoryNavigationTransaction<PulseFixtureMutation> for SuccessfulTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &longhorn_history::HistoryNavigationPlan<PulseFixtureMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

#[test]
fn navigation_closes_active_group_and_adjacent_coalescing_boundary() {
    let mut history =
        LinearHistory::new(history_id("history:navigation"), HistoryLimits::default());
    let policy = PulseFixturePolicy;
    history
        .record_applied(
            record(
                0,
                "entry:1",
                metadata("Rename", "track:rename"),
                rename(1, "A", "B"),
            ),
            &policy,
        )
        .unwrap();
    history
        .open_group(HistoryGroupId::new("group:open").unwrap())
        .unwrap();
    let plan = history
        .plan_navigation(
            longhorn_history::HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:undo").unwrap(),
                HistoryRevision::new(1),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    history
        .execute_navigation(plan, &mut SuccessfulTransaction)
        .unwrap();
    assert!(history.active_group().is_none());
    let redo = history
        .plan_navigation(
            longhorn_history::HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:redo").unwrap(),
                HistoryRevision::new(2),
                HistoryNavigationTarget::Redo,
            ),
            &policy,
        )
        .unwrap();
    history
        .execute_navigation(redo, &mut SuccessfulTransaction)
        .unwrap();
    history
        .record_applied(
            record(
                3,
                "entry:2",
                metadata("Rename again", "track:rename"),
                rename(1, "B", "C"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(history.applied().len(), 2);
}

#[test]
fn pulse_shaped_default_100_entry_limit_prunes_entry_101_exactly() {
    let mut history = LinearHistory::new(history_id("history:pulse-100"), HistoryLimits::default());
    for index in 0..=100_u64 {
        let result = history
            .record_applied(
                record(
                    index,
                    &format!("entry:{index:04}"),
                    metadata("Delete track", "track:delete"),
                    PulseFixtureMutation::DeleteTrack {
                        track_id: u32::try_from(index).unwrap(),
                        snapshot: format!("Track {index}"),
                    },
                ),
                &PulseFixturePolicy,
            )
            .unwrap();
        if index == 100 {
            assert_eq!(
                result.pruning().advanced_baseline()[0].entry_id().as_str(),
                "entry:0000"
            );
        }
    }
    assert_eq!(history.applied().len(), 100);
    assert_eq!(history.retained_baseline().pruned_entry_count(), 1);
}
