use std::sync::atomic::Ordering;

use crate::{DebounceFlushSet, DebounceTerminal, DebouncedMutation, FlushOutcome, FlushSetError};

use super::support::{
    FakeClock, PatchIntent, PatchStrategy, TestDomain, fixture, policy, target_path,
};

#[test]
fn aggregate_flush_is_stable_complete_and_rejects_invalid_membership() {
    let (_temp, mut store) = fixture();
    let domain_a = TestDomain::new("example.alpha", "example/alpha.json");
    let domain_b = TestDomain::new("example.beta", "example/beta.json");
    store.register(&domain_a).unwrap();
    store.register(&domain_b).unwrap();
    let failed_strategy = PatchStrategy::default();
    failed_strategy.fail_once.store(true, Ordering::SeqCst);
    let mut lane_b = DebouncedMutation::new(
        &store,
        &domain_b,
        PatchStrategy::default(),
        FakeClock::default(),
        policy(32),
    )
    .unwrap();
    let mut lane_a = DebouncedMutation::new(
        &store,
        &domain_a,
        failed_strategy,
        FakeClock::default(),
        policy(32),
    )
    .unwrap();
    lane_a.stage(PatchIntent::name("alpha")).unwrap();
    lane_b.stage(PatchIntent::name("beta")).unwrap();

    let mut set = DebounceFlushSet::new(&store);
    set.insert(&mut lane_b).unwrap();
    set.insert(&mut lane_a).unwrap();
    let outcomes = set.flush_all();
    assert_eq!(outcomes.len(), 2);
    assert!(matches!(
        &outcomes[0],
        FlushOutcome::Terminal(DebounceTerminal::Failed { domain, .. })
            if domain.as_str() == "example.alpha"
    ));
    assert!(matches!(
        &outcomes[1],
        FlushOutcome::Terminal(DebounceTerminal::Published { domain, .. })
            if domain.as_str() == "example.beta"
    ));
    drop(set);

    let mut duplicate = DebouncedMutation::new(
        &store,
        &domain_a,
        PatchStrategy::default(),
        FakeClock::default(),
        policy(32),
    )
    .unwrap();
    let mut set = DebounceFlushSet::new(&store);
    set.insert(&mut lane_a).unwrap();
    assert!(matches!(
        set.insert(&mut duplicate),
        Err(FlushSetError::DuplicateDomain { .. })
    ));
}

#[test]
fn wrong_store_is_rejected_and_drop_performs_no_io() {
    let (temp_a, mut store_a) = fixture();
    let (_temp_b, mut store_b) = fixture();
    let domain_a = TestDomain::new("example.settings", "example/settings.json");
    let domain_b = TestDomain::new("example.settings", "example/settings.json");
    store_a.register(&domain_a).unwrap();
    store_b.register(&domain_b).unwrap();
    let mut lane = DebouncedMutation::new(
        &store_a,
        &domain_a,
        PatchStrategy::default(),
        FakeClock::default(),
        policy(32),
    )
    .unwrap();
    lane.stage(PatchIntent::name("pending")).unwrap();

    let mut set = DebounceFlushSet::new(&store_b);
    assert!(matches!(
        set.insert(&mut lane),
        Err(FlushSetError::WrongStore { .. })
    ));
    drop(set);
    drop(lane);
    assert!(!target_path(&temp_a, "example/settings.json").exists());
}
