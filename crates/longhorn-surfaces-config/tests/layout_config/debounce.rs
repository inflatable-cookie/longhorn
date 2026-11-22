use std::{
    env, fs,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use longhorn_config::{
    ConfigStore, CoordinationFailureKind, DebounceFlushSet, DebouncePolicy, DebounceTerminal,
    FlushOutcome, LoadOutcome, MutationError, StageError,
};
use longhorn_config::{DomainDescriptor, DomainFilePath, StorageClass};
use longhorn_core::{
    DomainId, LayoutSchemaId, PanelDefinitionId, RegionFamilyId, RegionId, SchemaVersion,
    SizingSlotId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutRatio, LayoutSchemaDefinition,
    PanelDefinition, PanelInstancePolicy, PlacementSelector, RegionDefinition,
    SizingSlotDefinition, SurfaceLimits, validate_normalized_document,
};
use longhorn_surfaces_config::{
    LayoutPresentationIntent, LayoutPresentationIntentError, NoSurfaceMigration,
    RegisteredSurfaceDomain, SurfaceBackupPolicy, debounced_layout_presentation,
    publish_layout_mutation,
};

use crate::support::{
    FixedClock, Fixture, activate_request, collapse_request, document, domain, options, registry,
    sizing_request,
};

const HELPER_MODE: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_MODE";
const HELPER_ROOT: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_ROOT";
const HELPER_MARKER: &str = "LONGHORN_LAYOUT_CONFIG_HELPER_MARKER";
const HELPER_START_TIMEOUT: Duration = Duration::from_secs(60);

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
    let mut lane = debounced_layout_presentation(
        &store,
        &domain,
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
        loaded.value.surfaces()[0].sizing_slots()[0]
            .ratio()
            .millionths(),
        340_000
    );
    assert_eq!(
        loaded.value.surfaces()[0].regions()[0].collapsed(),
        Some(true)
    );
}

#[test]
fn pending_weight_rejection_preserves_existing_generation() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut lane = debounced_layout_presentation(
        &store,
        &domain,
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
    let mut lane = debounced_layout_presentation(
        &store,
        &domain,
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
    let secondary = RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            DomainId::new("layout.secondary").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/secondary-layout.json").unwrap()),
        )
        .unwrap(),
        document(),
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    let mut store = fixture.store();
    store.register(&primary).unwrap();
    store.register(&secondary).unwrap();
    let mut first = debounced_layout_presentation(
        &store,
        &primary,
        FixedClock,
        policy(1, Duration::from_secs(1)),
    )
    .unwrap();
    let mut second = debounced_layout_presentation(
        &store,
        &secondary,
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
    let mut lane = debounced_layout_presentation(
        &store,
        &domain,
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

#[test]
fn debounce_lanes_bind_each_domain_to_its_own_registry_order_and_sizing_bounds() {
    let fixture = Fixture::new();
    let primary = domain();
    let reversed_and_narrower_registry = LayoutDefinitionRegistry::new(
        SurfaceLimits::builder()
            .maximum_schemas(8)
            .maximum_regions_per_schema(8)
            .maximum_sizing_slots_per_schema(8)
            .maximum_panel_definitions(8)
            .maximum_surfaces(8)
            .maximum_windows(4)
            .maximum_host_preferences_per_surface(4)
            .maximum_label_bytes(64)
            .maximum_panel_instances(32)
            .maximum_panel_instances_per_region(16)
            .build()
            .unwrap(),
        [LayoutSchemaDefinition::new(
            LayoutSchemaId::new("schema:workspace").unwrap(),
            [
                // Intentionally opposite region order (`main` before `left`) from `registry()`.
                RegionDefinition::new(
                    RegionId::new("main").unwrap(),
                    RegionFamilyId::new("content").unwrap(),
                    10,
                    EmptyRegionPolicy::KeepVisible,
                    false,
                ),
                RegionDefinition::new(
                    RegionId::new("left").unwrap(),
                    RegionFamilyId::new("side").unwrap(),
                    20,
                    EmptyRegionPolicy::KeepVisible,
                    true,
                ),
            ],
            // Narrower sizing bounds [180_000, 350_000] than `registry()` [100_000, 500_000].
            [SizingSlotDefinition::new(
                SizingSlotId::new("left-width").unwrap(),
                10,
                LayoutRatio::from_millionths(180_000).unwrap(),
                LayoutRatio::from_millionths(200_000).unwrap(),
                LayoutRatio::from_millionths(350_000).unwrap(),
            )],
        )],
        [
            PanelDefinition::new(
                PanelDefinitionId::new("panel:activity").unwrap(),
                [PlacementSelector::Region(RegionId::new("left").unwrap())],
                [PlacementSelector::Region(RegionId::new("left").unwrap())],
                PanelInstancePolicy::Singleton,
                false,
                false,
            ),
            PanelDefinition::new(
                PanelDefinitionId::new("panel:tool").unwrap(),
                [PlacementSelector::Region(RegionId::new("main").unwrap())],
                [PlacementSelector::Region(RegionId::new("main").unwrap())],
                PanelInstancePolicy::Multiple,
                true,
                true,
            ),
        ],
    )
    .unwrap();
    let secondary = RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            DomainId::new("layout.reversed").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("workspace/reversed-layout.json").unwrap()),
        )
        .unwrap(),
        document(),
        reversed_and_narrower_registry,
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap();

    let mut store = fixture.store();
    store.register(&primary).unwrap();
    store.register(&secondary).unwrap();

    let mut primary_lane = debounced_layout_presentation(
        &store,
        &primary,
        FixedClock,
        policy(2, Duration::from_secs(1)),
    )
    .unwrap();
    let mut secondary_lane = debounced_layout_presentation(
        &store,
        &secondary,
        FixedClock,
        policy(2, Duration::from_secs(1)),
    )
    .unwrap();

    // 150_000 is valid in `primary` ([100_000, 500_000]) but out of bounds in `secondary` ([180_000, 350_000]).
    primary_lane
        .stage(LayoutPresentationIntent::new(sizing_request(7, 150_000)).unwrap())
        .unwrap();
    secondary_lane
        .stage(LayoutPresentationIntent::new(sizing_request(7, 150_000)).unwrap())
        .unwrap();

    assert!(matches!(
        primary_lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { .. })
    ));
    assert!(matches!(
        secondary_lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Failed {
            error: MutationError::Patch(_),
            ..
        })
    ));

    // A valid sizing update on `secondary` must flush in `secondary`'s canonical region order (`main`, `left`).
    let mut secondary_valid_lane = debounced_layout_presentation(
        &store,
        &secondary,
        FixedClock,
        policy(2, Duration::from_secs(1)),
    )
    .unwrap();
    secondary_valid_lane
        .stage(LayoutPresentationIntent::new(sizing_request(7, 300_000)).unwrap())
        .unwrap();
    assert!(matches!(
        secondary_valid_lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { .. })
    ));

    let LoadOutcome::Ready(primary_loaded) = store.load(&primary).unwrap() else {
        panic!("primary domain should load after debounced flush");
    };
    let LoadOutcome::Ready(secondary_loaded) = store.load(&secondary).unwrap() else {
        panic!("secondary domain should load after debounced flush");
    };

    validate_normalized_document(primary.registry(), &primary_loaded.value).unwrap();
    validate_normalized_document(secondary.registry(), &secondary_loaded.value).unwrap();
    assert_eq!(
        primary_loaded.value.surfaces()[0]
            .regions()
            .iter()
            .map(|region| region.region_id().as_str())
            .collect::<Vec<_>>(),
        vec!["left", "main"]
    );
    assert_eq!(
        secondary_loaded.value.surfaces()[0]
            .regions()
            .iter()
            .map(|region| region.region_id().as_str())
            .collect::<Vec<_>>(),
        vec!["main", "left"]
    );
}

fn external_fixture(root: &std::path::Path) -> ConfigStore {
    let data = root.join("data");
    let roots = longhorn_storage_layout::StorageRoots::new(
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
    let deadline = Instant::now() + HELPER_START_TIMEOUT;
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
    let _ = child.wait();
    panic!("coordination helper did not acquire the lock");
}
