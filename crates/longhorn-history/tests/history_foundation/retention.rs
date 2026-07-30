use std::convert::Infallible;

use longhorn_core::{HistoryGroupId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryEntry, HistoryEntrySequence,
    HistoryLimitChangeError, HistoryLimits, HistoryNavigationRequest, HistoryNavigationTarget,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure, HistoryPolicy,
    HistoryRecordError, HistoryRetainedBaseline, LinearHistory, LinearHistoryState,
};

use crate::support::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct WeightedValue {
    value: u32,
    encoded_weight: u64,
}

struct WeightedPolicy;

impl HistoryPolicy<WeightedValue> for WeightedPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &WeightedValue) -> Result<WeightedValue, Self::Error> {
        Ok(payload.clone())
    }

    fn is_noop(&self, _: &WeightedValue) -> bool {
        false
    }

    fn encoded_weight(&self, payload: &WeightedValue) -> Result<u64, Self::Error> {
        Ok(payload.encoded_weight)
    }

    fn coalesce(
        &self,
        _: &WeightedValue,
        _: &WeightedValue,
        _: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<WeightedValue>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

fn value(value: u32, encoded_weight: u64) -> WeightedValue {
    WeightedValue {
        value,
        encoded_weight,
    }
}

#[test]
fn record_enforces_count_and_weight_with_exact_baseline_pruning() {
    let limits = HistoryLimits::new(3, 6, 1_024).unwrap();
    let mut history = LinearHistory::new(history_id("history:weighted"), limits);
    for revision in 0..3_u64 {
        history
            .record_applied(
                record(
                    revision,
                    &format!("entry:{}", revision + 1),
                    metadata("Weighted", "document:weighted"),
                    value(u32::try_from(revision).unwrap(), 2),
                ),
                &WeightedPolicy,
            )
            .unwrap();
    }
    let count_prune = history
        .record_applied(
            record(
                3,
                "entry:4",
                metadata("Weighted", "document:weighted"),
                value(4, 2),
            ),
            &WeightedPolicy,
        )
        .unwrap();
    assert_eq!(
        count_prune.pruning().advanced_baseline()[0].entry_id(),
        &entry_id("entry:1")
    );
    assert_eq!(count_prune.retained_encoded_weight(), 6);

    let weight_prune = history
        .record_applied(
            record(
                4,
                "entry:5",
                metadata("Heavy", "document:weighted"),
                value(5, 4),
            ),
            &WeightedPolicy,
        )
        .unwrap();
    assert_eq!(
        weight_prune
            .pruning()
            .advanced_baseline()
            .iter()
            .map(|entry| entry.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:2", "entry:3"]
    );
    assert_eq!(history.retained_encoded_weight().unwrap(), 6);
    assert_eq!(history.retained_baseline().pruned_entry_count(), 3);
    assert_eq!(history.retained_baseline().pruned_encoded_weight(), 6);
    assert_eq!(
        history.retained_baseline().last_pruned_entry_id(),
        Some(&entry_id("entry:3"))
    );

    let before = history.clone();
    assert_eq!(
        history.record_applied(
            record(
                5,
                "entry:oversized",
                metadata("Oversized", "document:weighted"),
                value(9, 7),
            ),
            &WeightedPolicy,
        ),
        Err(HistoryRecordError::PayloadWeightExceedsLimit {
            maximum: 6,
            actual: 7,
        })
    );
    assert_eq!(history, before);
}

struct SuccessfulTransaction;

impl HistoryNavigationTransaction<WeightedValue> for SuccessfulTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &longhorn_history::HistoryNavigationPlan<WeightedValue>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

#[test]
fn limit_change_advances_applied_baseline_then_truncates_farthest_future() {
    let original_limits = HistoryLimits::new(10, 100, 1_024).unwrap();
    let mut history = LinearHistory::new(history_id("history:limits"), original_limits);
    for revision in 0..5_u64 {
        history
            .record_applied(
                record(
                    revision,
                    &format!("entry:{}", revision + 1),
                    metadata("Step", "document:step"),
                    value(u32::try_from(revision).unwrap(), 1),
                ),
                &WeightedPolicy,
            )
            .unwrap();
    }
    for (index, revision) in [5_u64, 6].into_iter().enumerate() {
        let plan = history
            .plan_navigation(
                HistoryNavigationRequest::new(
                    HistoryPlanId::new(format!("plan:undo-{index}")).unwrap(),
                    HistoryRevision::new(revision),
                    HistoryNavigationTarget::Undo,
                ),
                &WeightedPolicy,
            )
            .unwrap();
        history
            .execute_navigation(plan, &mut SuccessfulTransaction)
            .unwrap();
    }
    assert_eq!(history.applied().len(), 3);
    assert_eq!(history.future().len(), 2);
    history
        .open_group(HistoryGroupId::new("group:limits").unwrap())
        .unwrap();

    let receipt = history
        .change_limits(
            HistoryRevision::new(7),
            HistoryLimits::new(1, 100, 1_024).unwrap(),
        )
        .unwrap();
    assert_eq!(
        receipt
            .pruning()
            .advanced_baseline()
            .iter()
            .map(|entry| entry.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:1", "entry:2", "entry:3"]
    );
    assert_eq!(
        receipt
            .pruning()
            .discarded_future()
            .iter()
            .map(|entry| entry.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:5"]
    );
    assert!(history.applied().is_empty());
    assert_eq!(history.future().len(), 1);
    assert_eq!(
        history.next_redo().unwrap().entry_id(),
        &entry_id("entry:4")
    );
    assert_eq!(history.revision().get(), 8);
    assert_eq!(history.retained_baseline().pruned_entry_count(), 3);
    assert!(history.active_group().is_none());

    let before = history.clone();
    assert!(matches!(
        history.change_limits(HistoryRevision::new(7), original_limits),
        Err(HistoryLimitChangeError::StaleRevision { .. })
    ));
    assert_eq!(history, before);
}

#[test]
fn baseline_and_encoded_weight_overflow_fail_without_mutation() {
    assert!(HistoryLimits::new(1, 0, 1_024).is_err());
    let limits = HistoryLimits::new(1, 10, 1_024).unwrap();
    let state = LinearHistoryState::with_retained_baseline(
        history_id("history:overflow"),
        HistoryRevision::new(1),
        HistoryEntrySequence::new(3).unwrap(),
        HistoryRetainedBaseline::new(
            u64::MAX,
            0,
            Some(entry_id("entry:baseline")),
            Some(HistoryEntrySequence::new(1).unwrap()),
        ),
        vec![HistoryEntry::new(
            entry_id("entry:2"),
            metadata("Existing", "document:weighted"),
            HistoryEntrySequence::new(2).unwrap(),
            HistoryRevision::new(1),
            1,
            value(2, 1),
        )],
        Vec::new(),
    );
    let mut history = LinearHistory::from_state(limits, state).unwrap();
    let before = history.clone();
    assert!(matches!(
        history.record_applied(
            record(
                1,
                "entry:3",
                metadata("New", "document:weighted"),
                value(3, 1),
            ),
            &WeightedPolicy,
        ),
        Err(HistoryRecordError::Retention(
            longhorn_history::HistoryRetentionError::BaselineOverflow
        ))
    ));
    assert_eq!(history, before);
}
