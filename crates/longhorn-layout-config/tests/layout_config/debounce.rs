use std::{
    env, fs,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use longhorn_config::{
    ConfigStore, CoordinationFailureKind, DebounceFlushSet, DebouncePolicy, DebounceTerminal,
    DebouncedMutation, FlushOutcome, LoadOutcome, MutationError, StageError,
};
use longhorn_config::{DomainDescriptor, DomainFilePath, StorageClass};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_layout_config::{
    LayoutBackupPolicy, LayoutPresentationIntent, LayoutPresentationIntentError,
    LayoutPresentationStrategy, NoLayoutMigration, RegisteredLayoutDomain, publish_layout_mutation,
};

use crate::support::{
    FixedClock, Fixture, activate_request, collapse_request, document, domain, options, registry,
    sizing_request,
};

const HELPER_MODE: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_MODE";
const HELPER_ROOT: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_ROOT";
const HELPER_MARKER: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_MARKER";

fn policy(maximum: usize, timeout: Duration) -> DebouncePolicy {
    DebouncePolicy::new(Duration::from_millis(200), maximum, options(timeout)).unwrap()
}

#[test]
fn presentation_lane_rejects_structural_commands() {
    assert_eq!(
        LayoutPresentationIntent::new(activate_request(7)),
        Err(LayoutPresentationIntentError::StructuralCommand)
    );
}

#[test]
fn ordered_presentation_requests_publish_once_over_fresh_state() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        LayoutPresentationStrategy::new(domain.registry()),
        FixedClock,
        policy(2, Duration::from_secs(1)),
    )
    .unwrap();
    lane.stage(LayoutPresentationIntent::new(sizing_request(7, 340_000)).unwrap())
        .unwrap();
    lane.stage(LayoutPresentationIntent::new(collapse_request(8, true)).unwrap())
        .unwrap();

    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { generation: 2, .. })
    ));
    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("flushed presentation state should load");
    };
    assert_eq!(loaded.value.revision().get(), 9);
    assert_eq!(
        loaded.value.containers()[0].sizing_slots()[0]
            .ratio()
            .millionths(),
        340_000
    );
    assert_eq!(
        loaded.value.containers()[0].regions()[0].collapsed(),
        Some(true)
    );
}

#[test]
fn pending_weight_rejection_preserves_existing_generation() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        LayoutPresentationStrategy::new(domain.registry()),
        FixedClock,
        policy(1, Duration::from_secs(1)),
    )
    .unwrap();
    let opened = lane
        .stage(LayoutPresentationIntent::new(sizing_request(7, 340_000)).unwrap())
        .unwrap();
    let error = lane
        .stage(LayoutPresentationIntent::new(collapse_request(8, true)).unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        StageError::PendingWeightExceeded {
            attempted: 2,
            maximum: 1,
            ..
        }
    ));
    assert_eq!(
        lane.snapshot().pending.unwrap().generation,
        opened.generation
    );
}

#[test]
fn structural_publication_does_not_wait_behind_presentation_debounce() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        LayoutPresentationStrategy::new(domain.registry()),
        FixedClock,
        policy(1, Duration::from_secs(1)),
    )
    .unwrap();
    lane.stage(LayoutPresentationIntent::new(sizing_request(8, 360_000)).unwrap())
        .unwrap();

    let structural = publish_layout_mutation(
        &store,
        &domain,
        options(Duration::from_secs(1)),
        &activate_request(7),
    )
    .unwrap();
    assert_eq!(structural.layout().committed_revision().get(), 8);
    assert!(lane.snapshot().pending.is_some());
    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { generation: 1, .. })
    ));

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("structural then presentation state should load");
    };
    assert_eq!(loaded.value.revision().get(), 9);
}

#[test]
fn aggregate_flush_accepts_multiple_explicit_layout_domains() {
    let fixture = Fixture::new();
    let primary = domain();
    let secondary = RegisteredLayoutDomain::new(
        DomainDescriptor::new(
            DomainId::new("layout.secondary").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/secondary-layout.json").unwrap()),
        )
        .unwrap(),
        document(),
        registry(),
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap();
    let mut store = fixture.store();
    store.register(&primary).unwrap();
    store.register(&secondary).unwrap();
    let mut first = DebouncedMutation::new(
        &store,
        &primary,
        LayoutPresentationStrategy::new(primary.registry()),
        FixedClock,
        policy(1, Duration::from_secs(1)),
    )
    .unwrap();
    let mut second = DebouncedMutation::new(
        &store,
        &secondary,
        LayoutPresentationStrategy::new(secondary.registry()),
        FixedClock,
        policy(1, Duration::from_secs(1)),
    )
    .unwrap();
    first
        .stage(LayoutPresentationIntent::new(sizing_request(7, 310_000)).unwrap())
        .unwrap();
    second
        .stage(LayoutPresentationIntent::new(collapse_request(7, true)).unwrap())
        .unwrap();

    let outcomes = {
        let mut set = DebounceFlushSet::new(&store);
        set.insert(&mut first).unwrap();
        set.insert(&mut second).unwrap();
        set.flush_all()
    };
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|outcome| matches!(
        outcome,
        FlushOutcome::Terminal(DebounceTerminal::Published { .. })
    )));
}

#[test]
fn coordination_helper_process() {
    if env::var(HELPER_MODE).as_deref() != Ok("hold") {
        return;
    }
    let root = std::path::PathBuf::from(env::var_os(HELPER_ROOT).unwrap());
    let fixture = external_fixture(&root);
    let domain = domain();
    let mut store = fixture;
    store.register(&domain).unwrap();
    let marker = std::path::PathBuf::from(env::var_os(HELPER_MARKER).unwrap());
    store
        .mutate(&domain, options(Duration::from_secs(10)), |_value| {
            fs::write(marker, b"locked").unwrap();
            thread::sleep(Duration::from_secs(30));
            Ok(())
        })
        .unwrap();
}

#[test]
fn timeout_retains_same_pending_generation_for_explicit_retry() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        LayoutPresentationStrategy::new(domain.registry()),
        FixedClock,
        policy(1, Duration::from_millis(50)),
    )
    .unwrap();
    let staged = lane
        .stage(LayoutPresentationIntent::new(sizing_request(7, 370_000)).unwrap())
        .unwrap();
    let marker = fixture.temp.path().join("layout-holder-ready");
    let mut helper = spawn_helper(fixture.temp.path(), &marker);
    wait_for_marker(&mut helper, &marker);

    let outcome = lane.flush_forced();
    let FlushOutcome::Terminal(DebounceTerminal::Failed {
        generation,
        error: MutationError::Coordination(failure),
        ..
    }) = outcome
    else {
        let _ = helper.kill();
        panic!("lock timeout should retain the pending layout intent");
    };
    assert_eq!(generation, staged.generation);
    assert_eq!(failure.kind, CoordinationFailureKind::Timeout);
    let pending = lane.snapshot().pending.unwrap();
    assert_eq!(pending.generation, staged.generation);
    assert!(pending.retry_required);

    helper.kill().unwrap();
    helper.wait().unwrap();
    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published {
            generation,
            ..
        }) if generation == staged.generation
    ));
}

fn external_fixture(root: &std::path::Path) -> ConfigStore {
    let data = root.join("data");
    let roots = longhorn_config::StorageRoots::new(
        root.join("config"),
        &data,
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("log"),
        root.join("backups"),
    )
    .unwrap();
    ConfigStore::new(
        roots,
        longhorn_config::CoordinationAuthority::new(data).unwrap(),
    )
}

fn spawn_helper(root: &std::path::Path, marker: &std::path::Path) -> Child {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("debounce::coordination_helper_process")
        .arg("--nocapture")
        .env(HELPER_MODE, "hold")
        .env(HELPER_ROOT, root)
        .env(HELPER_MARKER, marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_for_marker(child: &mut Child, marker: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("coordination helper exited before acquiring lock: {status}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    panic!("coordination helper did not acquire the lock");
}
