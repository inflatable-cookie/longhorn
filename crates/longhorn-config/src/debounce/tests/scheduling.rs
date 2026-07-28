use std::{fs, time::Duration};

use serde_json::{Value, json};

use crate::{
    DebouncePolicy, DebouncePolicyError, DebounceTerminal, DebouncedMutation,
    DurabilityRequirement, FlushOutcome, MutationOptions, StageDisposition, StageError,
};

use super::support::{
    FakeClock, PatchIntent, PatchStrategy, TestDomain, fixture, policy, target_path,
};

#[test]
fn fake_clock_resets_trailing_deadline_and_coalesces_ordered_patches() {
    let (temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let clock = FakeClock::default();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        PatchStrategy::default(),
        clock.clone(),
        policy(32),
    )
    .unwrap();

    let first = lane.stage(PatchIntent::name("first")).unwrap();
    assert_eq!(first.due_at, Duration::from_millis(200));
    assert_eq!(first.disposition, StageDisposition::Opened);

    clock.set_millis(150);
    let second = lane.stage(PatchIntent::enabled(false)).unwrap();
    assert_eq!(second.due_at, Duration::from_millis(350));
    assert_eq!(
        second.disposition,
        StageDisposition::Coalesced {
            previous_generation: 1
        }
    );

    clock.set_millis(349);
    assert!(matches!(lane.flush_due(), FlushOutcome::NotDue { .. }));
    assert!(!target_path(&temp, "example/settings.json").exists());

    clock.set_millis(350);
    let FlushOutcome::Terminal(DebounceTerminal::Published {
        generation,
        receipt,
        ..
    }) = lane.flush_due()
    else {
        panic!("expected published terminal");
    };
    assert_eq!(generation, 2);
    let document: Value = serde_json::from_slice(&fs::read(receipt.path).unwrap()).unwrap();
    assert_eq!(
        document["value"],
        json!({"name": "first", "enabled": false})
    );
    assert!(lane.snapshot().pending.is_none());
}

#[test]
fn rejected_candidate_preserves_pending_generation_and_deadline() {
    let (_temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let clock = FakeClock::default();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        PatchStrategy::default(),
        clock.clone(),
        policy(5),
    )
    .unwrap();
    lane.stage(PatchIntent::name("ok")).unwrap();
    let before = lane.snapshot();

    clock.set_millis(50);
    assert!(matches!(
        lane.stage(PatchIntent::name("reject")),
        Err(StageError::Coalescing { .. })
    ));
    assert_eq!(lane.snapshot(), before);

    assert!(matches!(
        lane.stage(PatchIntent::name("too-long")),
        Err(StageError::PendingWeightExceeded { .. })
    ));
    assert_eq!(lane.snapshot(), before);
}

#[test]
fn unchanged_flush_and_explicit_discard_do_not_create_a_file() {
    let (temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        PatchStrategy::default(),
        FakeClock::default(),
        policy(32),
    )
    .unwrap();
    lane.stage(PatchIntent::name("default")).unwrap();
    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Unchanged { generation: 1, .. })
    ));
    assert!(!target_path(&temp, "example/settings.json").exists());

    lane.stage(PatchIntent::enabled(false)).unwrap();
    assert!(matches!(
        lane.discard(),
        FlushOutcome::Terminal(DebounceTerminal::Discarded { generation: 2, .. })
    ));
    assert!(!target_path(&temp, "example/settings.json").exists());
}

#[test]
fn policy_and_deadline_bounds_are_typed() {
    assert_eq!(
        DebouncePolicy::new(
            Duration::ZERO,
            0,
            MutationOptions::new(Duration::ZERO, DurabilityRequirement::Atomic)
        ),
        Err(DebouncePolicyError::ZeroPendingWeight)
    );

    let (_temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let clock = FakeClock::default();
    clock.set_millis(u64::MAX);
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        PatchStrategy::default(),
        clock,
        DebouncePolicy::new(
            Duration::MAX,
            32,
            MutationOptions::new(Duration::ZERO, DurabilityRequirement::Atomic),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        lane.stage(PatchIntent::name("value")),
        Err(StageError::DeadlineOverflow { .. })
    ));
}
