use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use longhorn_core::DomainId;

use crate::{
    CoordinationFailure, CoordinationFailureKind, DebounceTerminal, DebouncedMutation, DomainIssue,
    DomainLocation, FlushOutcome, MutationError, MutationRefusal, PublicationFailure,
    PublicationStage, RetryDisposition, StoreError, store::mutation::MutationOutcome,
};

use super::support::{FakeClock, PatchIntent, PatchStrategy, TestDomain, fixture, policy};

#[test]
fn failed_due_flush_requires_explicit_retry_and_retains_intent() {
    let (_temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let clock = FakeClock::default();
    let strategy = PatchStrategy::default();
    strategy.fail_once.store(true, Ordering::SeqCst);
    let applications = Arc::clone(&strategy.applications);
    let mut lane =
        DebouncedMutation::new(&store, &domain, strategy, clock.clone(), policy(32)).unwrap();
    lane.stage(PatchIntent::name("saved")).unwrap();

    clock.set_millis(200);
    assert!(matches!(
        lane.flush_due(),
        FlushOutcome::Terminal(DebounceTerminal::Failed {
            retry: RetryDisposition::RequiresIntervention,
            ..
        })
    ));
    assert_eq!(applications.load(Ordering::SeqCst), 1);
    assert!(lane.snapshot().pending.unwrap().retry_required);
    assert_eq!(lane.next_deadline(), None);

    assert!(matches!(
        lane.flush_due(),
        FlushOutcome::RetryRequired { generation: 1, .. }
    ));
    assert_eq!(applications.load(Ordering::SeqCst), 1);

    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { generation: 1, .. })
    ));
    assert_eq!(applications.load(Ordering::SeqCst), 2);
}

#[test]
fn new_stage_after_failure_coalesces_and_starts_a_new_deadline() {
    let (_temp, mut store) = fixture();
    let domain = TestDomain::new("example.settings", "example/settings.json");
    store.register(&domain).unwrap();
    let clock = FakeClock::default();
    let strategy = PatchStrategy::default();
    strategy.fail_once.store(true, Ordering::SeqCst);
    let mut lane =
        DebouncedMutation::new(&store, &domain, strategy, clock.clone(), policy(32)).unwrap();
    lane.stage(PatchIntent::name("saved")).unwrap();
    clock.set_millis(200);
    lane.flush_due();

    clock.set_millis(250);
    let receipt = lane.stage(PatchIntent::enabled(false)).unwrap();
    assert_eq!(receipt.generation, 2);
    assert_eq!(receipt.due_at, Duration::from_millis(450));
    assert!(!lane.snapshot().pending.unwrap().retry_required);
    clock.set_millis(449);
    assert!(matches!(lane.flush_due(), FlushOutcome::NotDue { .. }));
    clock.set_millis(450);
    assert!(matches!(
        lane.flush_due(),
        FlushOutcome::Terminal(DebounceTerminal::Published { generation: 2, .. })
    ));
}

#[test]
fn every_uncommitted_mutation_error_retains_the_same_generation() {
    let errors = vec![
        MutationError::Store(StoreError::NotRegistered {
            id: DomainId::new("example.settings").unwrap(),
        }),
        MutationError::Coordination(CoordinationFailure {
            kind: CoordinationFailureKind::Timeout,
            lock_path: PathBuf::from("config.lock"),
            detail: "injected".to_owned(),
        }),
        MutationError::Refused(MutationRefusal::Unavailable {
            location: DomainLocation::DefaultsOnly,
        }),
        MutationError::Patch(DomainIssue::new("patch", "injected")),
        MutationError::Validation(DomainIssue::new("validation", "injected")),
        MutationError::Encode(DomainIssue::new("encode", "injected")),
        MutationError::EncodedValueInvalid(DomainIssue::new("raw", "injected")),
        MutationError::Serialization {
            detail: "injected".to_owned(),
        },
        MutationError::Publication(PublicationFailure {
            stage: PublicationStage::Rename,
            path: PathBuf::from("settings.json"),
            published: false,
            detail: "injected".to_owned(),
        }),
    ];

    for error in errors {
        let (_temp, mut store) = fixture();
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
        lane.stage(PatchIntent::name("pending")).unwrap();

        assert!(matches!(
            lane.finish_flush(1, Err(error) as Result<MutationOutcome, MutationError>),
            FlushOutcome::Terminal(DebounceTerminal::Failed { generation: 1, .. })
        ));
        let pending = lane.snapshot().pending.unwrap();
        assert_eq!(pending.generation, 1);
        assert!(pending.retry_required);
    }
}

#[test]
fn known_post_publication_failure_clears_non_idempotent_intent() {
    let (_temp, mut store) = fixture();
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
    lane.stage(PatchIntent::name("published")).unwrap();
    let failure = PublicationFailure {
        stage: PublicationStage::SyncDirectory,
        path: PathBuf::from("settings.json"),
        published: true,
        detail: "injected".to_owned(),
    };

    assert!(matches!(
        lane.finish_flush(
            1,
            Err(MutationError::Publication(failure.clone()))
                as Result<MutationOutcome, MutationError>
        ),
        FlushOutcome::Terminal(DebounceTerminal::PublishedWithDurabilityFailure {
            failure: actual,
            ..
        }) if actual == failure
    ));
    assert!(lane.snapshot().pending.is_none());
}
