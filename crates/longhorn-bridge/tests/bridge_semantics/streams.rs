use longhorn_bridge::{
    BridgeEventEnvelope, BridgeSnapshotDecision, BridgeSnapshotEnvelope, BridgeStreamDecision,
    BridgeStreamTracker,
};

use crate::support::{cursor, domain_id, session_id};

#[test]
fn snapshot_and_event_payloads_round_trip_as_domain_owned_types() {
    let snapshot = BridgeSnapshotEnvelope::new(cursor("session:current", 2, 8), vec![1_u32, 2, 3]);
    let event = BridgeEventEnvelope::new(cursor("session:current", 2, 9), 4_u32);

    let snapshot_json = serde_json::to_string(&snapshot).unwrap();
    let event_json = serde_json::to_string(&event).unwrap();
    assert_eq!(
        serde_json::from_str::<BridgeSnapshotEnvelope<Vec<u32>>>(&snapshot_json).unwrap(),
        snapshot
    );
    assert_eq!(
        serde_json::from_str::<BridgeEventEnvelope<u32>>(&event_json).unwrap(),
        event
    );
}

#[test]
fn listener_first_race_forces_refresh_when_snapshot_is_older() {
    let mut tracker = BridgeStreamTracker::new(
        session_id("session:current"),
        domain_id("example.workspace"),
    );

    assert_eq!(
        tracker.classify_event(&cursor("session:current", 1, 6)),
        BridgeStreamDecision::ResnapshotRequired
    );
    assert_eq!(
        tracker.accept_snapshot(cursor("session:current", 1, 5)),
        BridgeSnapshotDecision::AcceptedResnapshotRequired
    );
    assert!(tracker.requires_snapshot());
    assert_eq!(
        tracker.accept_snapshot(cursor("session:current", 1, 6)),
        BridgeSnapshotDecision::Accepted
    );
    assert!(!tracker.requires_snapshot());
}

#[test]
fn duplicate_stale_contiguous_gap_and_epoch_rules_are_deterministic() {
    let mut tracker = BridgeStreamTracker::new(
        session_id("session:current"),
        domain_id("example.workspace"),
    );
    assert_eq!(
        tracker.accept_snapshot(cursor("session:current", 3, 10)),
        BridgeSnapshotDecision::Accepted
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 3, 10)),
        BridgeStreamDecision::IgnoreDuplicate
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 3, 9)),
        BridgeStreamDecision::IgnoreStale
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 3, 11)),
        BridgeStreamDecision::Apply
    );
    assert_eq!(tracker.accepted_cursor().unwrap().sequence().get(), 11);
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 3, 13)),
        BridgeStreamDecision::ResnapshotGap
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 3, 14)),
        BridgeStreamDecision::ResnapshotRequired
    );
    assert_eq!(
        tracker.accept_snapshot(cursor("session:current", 3, 14)),
        BridgeSnapshotDecision::Accepted
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 2, 99)),
        BridgeStreamDecision::IgnoreStale
    );
    assert_eq!(
        tracker.classify_event(&cursor("session:current", 4, 0)),
        BridgeStreamDecision::ResnapshotNewEpoch
    );
}

#[test]
fn superseded_session_can_never_advance_current_authority() {
    let mut tracker =
        BridgeStreamTracker::new(session_id("session:old"), domain_id("example.workspace"));
    tracker.accept_snapshot(cursor("session:old", 1, 4));
    tracker.advance_session(session_id("session:new"));

    assert_eq!(
        tracker.classify_event(&cursor("session:old", 1, 5)),
        BridgeStreamDecision::IgnoreSupersededSession
    );
    assert_eq!(
        tracker.accept_snapshot(cursor("session:old", 1, 6)),
        BridgeSnapshotDecision::SupersededSession
    );
    assert!(tracker.accepted_cursor().is_none());
    assert!(tracker.requires_snapshot());
}
