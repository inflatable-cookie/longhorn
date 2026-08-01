use std::convert::Infallible;

use longhorn_core::{HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryCommittedTransitionKind, HistoryLimitChangeReceipt, HistoryLimits,
    HistoryNavigationPlan, HistoryNavigationRequest, HistoryNavigationTarget,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
    HistoryRecordTransitionEffect, LinearHistory,
};

use crate::{
    loophole::{PulseFixtureMutation, PulseFixturePolicy, rename},
    support::*,
};

struct SuccessfulTransaction;

impl HistoryNavigationTransaction<PulseFixtureMutation> for SuccessfulTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &HistoryNavigationPlan<PulseFixtureMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

#[test]
fn every_committed_mutation_class_yields_one_payload_free_transition() {
    let policy = PulseFixturePolicy;
    let limits = HistoryLimits::new(10, 1_024, 1_024).unwrap();
    let mut history = LinearHistory::new(history_id("history:transitions"), limits);

    let added = history
        .record_applied(
            record(
                0,
                "entry:rename",
                metadata("Rename track", "track:rename"),
                rename(1, "A", "B"),
            ),
            &policy,
        )
        .unwrap();
    assert!(matches!(
        added.transition().unwrap().kind(),
        HistoryCommittedTransitionKind::Record {
            effect: HistoryRecordTransitionEffect::Added { .. },
            ..
        }
    ));

    let coalesced = history
        .record_applied(
            record(
                1,
                "entry:rename-2",
                metadata("Rename track again", "track:rename"),
                rename(1, "B", "C"),
            ),
            &policy,
        )
        .unwrap();
    assert!(matches!(
        coalesced.transition().unwrap().kind(),
        HistoryCommittedTransitionKind::Record {
            effect: HistoryRecordTransitionEffect::Replaced { .. },
            ..
        }
    ));

    let added_second = history
        .record_applied(
            record(
                2,
                "entry:delete",
                metadata("Delete track", "track:delete"),
                PulseFixtureMutation::DeleteTrack {
                    track_id: 2,
                    snapshot: "Keys".to_owned(),
                },
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        added_second.transition().unwrap().previous_revision(),
        Some(HistoryRevision::new(2))
    );

    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:transition-undo").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    let navigation = history
        .execute_navigation(plan, &mut SuccessfulTransaction)
        .unwrap();
    assert!(matches!(
        navigation.transition().kind(),
        HistoryCommittedTransitionKind::Navigation { .. }
    ));

    let limit_change: HistoryLimitChangeReceipt = history
        .change_limits(
            history.revision(),
            HistoryLimits::new(1, 1_024, 1_024).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        limit_change.transition().unwrap().kind(),
        HistoryCommittedTransitionKind::LimitsChanged { pruning, .. }
            if pruning.advanced_baseline().len() == 1
    ));

    let reset = history.reset_committed(history.revision()).unwrap();
    assert!(matches!(
        reset.transition().unwrap().kind(),
        HistoryCommittedTransitionKind::Reset {
            removed_future,
            previous_baseline,
            ..
        } if removed_future.len() == 1 && previous_baseline.is_advanced()
    ));
}

#[test]
fn rejected_and_structurally_unchanged_attempts_emit_nothing() {
    let policy = PulseFixturePolicy;
    let mut history = LinearHistory::new(
        history_id("history:no-transition"),
        HistoryLimits::default(),
    );

    history
        .record_applied(
            record(
                0,
                "entry:rename",
                metadata("Rename track", "track:rename"),
                rename(1, "A", "B"),
            ),
            &policy,
        )
        .unwrap();
    let removed = history
        .record_applied(
            record(
                1,
                "entry:rename-back",
                metadata("Restore track name", "track:rename"),
                rename(1, "B", "A"),
            ),
            &policy,
        )
        .unwrap();
    assert!(matches!(
        removed.transition().unwrap().kind(),
        HistoryCommittedTransitionKind::Record {
            effect: HistoryRecordTransitionEffect::Removed { .. },
            ..
        }
    ));

    let no_op = history
        .record_applied(
            record(
                2,
                "entry:noop",
                metadata("No change", "track:rename"),
                rename(1, "A", "A"),
            ),
            &policy,
        )
        .unwrap();
    assert!(no_op.transition().is_none());

    let unchanged_limits = history
        .change_limits(history.revision(), history.limits())
        .unwrap();
    assert!(unchanged_limits.transition().is_none());

    let empty_reset = history.reset_committed(history.revision()).unwrap();
    assert!(empty_reset.transition().is_none());
    assert!(history.reset_committed(HistoryRevision::new(9)).is_err());
    assert_eq!(history.revision(), HistoryRevision::new(2));
}
