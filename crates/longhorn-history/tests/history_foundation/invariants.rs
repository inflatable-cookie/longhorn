use std::convert::Infallible;

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
    OpaqueIdError,
};
use longhorn_history::{
    HistoryCoalesce, HistoryEntry, HistoryEntrySequence, HistoryLabel, HistoryLabelError,
    HistoryLimits, HistoryLimitsError, HistoryPolicy, HistoryRecordError, HistoryStateError,
    LinearHistory, LinearHistoryState, MAXIMUM_HISTORY_LABEL_BYTES,
};

use crate::support::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Value(u32);

struct SeparatePolicy;

impl HistoryPolicy<Value> for SeparatePolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &Value) -> Result<Value, Self::Error> {
        Ok(payload.clone())
    }

    fn is_noop(&self, _: &Value) -> bool {
        false
    }

    fn coalesce(&self, _: &Value, _: &Value) -> Result<HistoryCoalesce<Value>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[test]
fn identities_labels_and_limits_are_bounded_and_distinct() {
    assert!(HistoryId::new("history:document").is_ok());
    assert!(HistoryEntryId::new("entry:0001").is_ok());
    assert!(HistoryKindId::new("document:insert").is_ok());
    assert!(HistoryGroupId::new("gesture:title").is_ok());
    assert!(HistoryPlanId::new("plan:0001").is_ok());
    assert_eq!(
        HistoryEntryId::new("Entry"),
        Err(OpaqueIdError::InvalidCharacter { index: 0 })
    );
    assert_eq!(HistoryLabel::new(""), Err(HistoryLabelError::Empty));
    assert_eq!(
        HistoryLabel::new("x".repeat(MAXIMUM_HISTORY_LABEL_BYTES + 1)),
        Err(HistoryLabelError::TooLong {
            maximum: MAXIMUM_HISTORY_LABEL_BYTES,
            actual: MAXIMUM_HISTORY_LABEL_BYTES + 1,
        })
    );
    assert_eq!(HistoryLimits::new(0, 10), Err(HistoryLimitsError::Zero));
}

#[test]
fn imported_state_rejects_duplicate_ids_order_revision_and_next_sequence() {
    let limits = HistoryLimits::default();
    let duplicate = LinearHistoryState::new(
        history_id("history:test"),
        HistoryRevision::new(2),
        HistoryEntrySequence::new(3).unwrap(),
        vec![entry("entry:1", "One", "test:value", 1, 1, Value(1))],
        vec![entry("entry:1", "Two", "test:value", 2, 2, Value(2))],
    );
    assert_eq!(
        LinearHistory::from_state(limits, duplicate),
        Err(HistoryStateError::DuplicateEntryId(entry_id("entry:1")))
    );

    let wrong_order = LinearHistoryState::new(
        history_id("history:test"),
        HistoryRevision::new(2),
        HistoryEntrySequence::new(3).unwrap(),
        vec![entry("entry:2", "Two", "test:value", 2, 2, Value(2))],
        vec![entry("entry:1", "One", "test:value", 1, 1, Value(1))],
    );
    assert_eq!(
        LinearHistory::from_state(limits, wrong_order),
        Err(HistoryStateError::SequenceOrder)
    );

    let future_revision = LinearHistoryState::new(
        history_id("history:test"),
        HistoryRevision::new(1),
        HistoryEntrySequence::new(2).unwrap(),
        vec![entry("entry:1", "One", "test:value", 1, 2, Value(1))],
        Vec::new(),
    );
    assert!(matches!(
        LinearHistory::from_state(limits, future_revision),
        Err(HistoryStateError::InvalidEntryRevision { .. })
    ));

    let stale_next = LinearHistoryState::new(
        history_id("history:test"),
        HistoryRevision::new(1),
        HistoryEntrySequence::new(1).unwrap(),
        vec![entry("entry:1", "One", "test:value", 1, 1, Value(1))],
        Vec::new(),
    );
    assert_eq!(
        LinearHistory::from_state(limits, stale_next),
        Err(HistoryStateError::NextSequenceNotAfterEntries)
    );
}

#[test]
fn stale_duplicate_bound_and_overflow_rejections_are_failure_atomic() {
    let mut history = LinearHistory::new(
        history_id("history:test"),
        HistoryLimits::new(4, 3).unwrap(),
    );
    history
        .record_applied(
            record(0, "entry:1", metadata("One", "test:value"), Value(1)),
            &SeparatePolicy,
        )
        .unwrap();

    let before = history.clone();
    assert!(matches!(
        history.record_applied(
            record(0, "entry:2", metadata("Two", "test:value"), Value(2)),
            &SeparatePolicy,
        ),
        Err(HistoryRecordError::StaleRevision { .. })
    ));
    assert_eq!(history, before);

    assert_eq!(
        history.record_applied(
            record(1, "entry:1", metadata("Two", "test:value"), Value(2)),
            &SeparatePolicy,
        ),
        Err(HistoryRecordError::DuplicateEntryId(entry_id("entry:1")))
    );
    assert_eq!(history, before);

    assert_eq!(
        history.record_applied(
            record(1, "entry:2", metadata("Four", "test:value"), Value(2),),
            &SeparatePolicy,
        ),
        Err(HistoryRecordError::LabelTooLong {
            maximum: 3,
            actual: 4,
        })
    );
    assert_eq!(history, before);

    let revision_max = LinearHistoryState::new(
        history_id("history:max-revision"),
        HistoryRevision::new(u64::MAX),
        HistoryEntrySequence::new(2).unwrap(),
        vec![entry("entry:1", "One", "test:value", 1, u64::MAX, Value(1))],
        Vec::new(),
    );
    let mut revision_max =
        LinearHistory::from_state(HistoryLimits::default(), revision_max).unwrap();
    assert_eq!(
        revision_max.record_applied(
            record(u64::MAX, "entry:2", metadata("Two", "test:value"), Value(2),),
            &SeparatePolicy,
        ),
        Err(HistoryRecordError::RevisionOverflow)
    );

    let sequence_max = LinearHistoryState::new(
        history_id("history:max-sequence"),
        HistoryRevision::new(1),
        HistoryEntrySequence::new(u64::MAX).unwrap(),
        vec![HistoryEntry::new(
            entry_id("entry:1"),
            metadata("One", "test:value"),
            HistoryEntrySequence::new(u64::MAX - 1).unwrap(),
            HistoryRevision::new(1),
            Value(1),
        )],
        Vec::new(),
    );
    let mut sequence_max =
        LinearHistory::from_state(HistoryLimits::default(), sequence_max).unwrap();
    assert_eq!(
        sequence_max.record_applied(
            record(1, "entry:2", metadata("Two", "test:value"), Value(2)),
            &SeparatePolicy,
        ),
        Err(HistoryRecordError::SequenceOverflow)
    );
}

struct ReplacingPolicy;

impl HistoryPolicy<Value> for ReplacingPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &Value) -> Result<Value, Self::Error> {
        Ok(payload.clone())
    }

    fn is_noop(&self, _: &Value) -> bool {
        false
    }

    fn coalesce(&self, _: &Value, incoming: &Value) -> Result<HistoryCoalesce<Value>, Self::Error> {
        Ok(HistoryCoalesce::Replace(incoming.clone()))
    }
}

#[test]
fn committed_group_boundary_prevents_cross_group_coalescing() {
    let mut history = LinearHistory::new(history_id("history:groups"), HistoryLimits::default());
    history
        .record_applied(
            record(
                0,
                "entry:1",
                grouped_metadata("First", "test:value", "group:1"),
                Value(1),
            ),
            &ReplacingPolicy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                1,
                "entry:2",
                grouped_metadata("Second", "test:value", "group:2"),
                Value(2),
            ),
            &ReplacingPolicy,
        )
        .unwrap();

    assert_eq!(
        history
            .applied()
            .iter()
            .map(|entry| entry.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:1", "entry:2"]
    );
    assert_eq!(history.applied()[0].sequence().get(), 1);
    assert_eq!(history.applied()[1].sequence().get(), 2);
}

struct InvalidReplacementPolicy;

impl HistoryPolicy<Value> for InvalidReplacementPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &Value) -> Result<Value, Self::Error> {
        Ok(payload.clone())
    }

    fn is_noop(&self, payload: &Value) -> bool {
        payload.0 == 0
    }

    fn coalesce(&self, _: &Value, _: &Value) -> Result<HistoryCoalesce<Value>, Self::Error> {
        Ok(HistoryCoalesce::Replace(Value(0)))
    }
}

#[test]
fn no_op_replacement_must_be_an_explicit_removal() {
    let mut history = LinearHistory::new(history_id("history:test"), HistoryLimits::default());
    history
        .record_applied(
            record(0, "entry:1", metadata("One", "test:value"), Value(1)),
            &SeparatePolicy,
        )
        .unwrap();
    let before = history.clone();

    assert_eq!(
        history.record_applied(
            record(1, "entry:2", metadata("Zero", "test:value"), Value(2)),
            &InvalidReplacementPolicy,
        ),
        Err(HistoryRecordError::CoalescedPayloadIsNoOp)
    );
    assert_eq!(history, before);
}
