//! Config-backed window placement sink integration tests.

use std::{collections::BTreeMap, fs, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, DurabilityRequirement, LoadOutcome, MigrationStep, MutationOptions, StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion, ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_storage_layout::StorageRoots;
use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, WindowFlushRequest, WindowFlushScope,
    WindowFlushTarget, WindowPlacementSink,
};
use longhorn_windowing_config::ConfigWindowPlacementSink;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
struct Session {
    placements: BTreeMap<String, CapturedWindowPlacement>,
    panel_ratio: f64,
}

struct SessionDomain {
    descriptor: DomainDescriptor,
}

impl SessionDomain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("test.window-session").unwrap(),
                SchemaVersion::new(2).unwrap(),
                StorageClass::MachineState,
                Some(DomainFilePath::new("window-session.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for SessionDomain {
    type Value = Session;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        Session::default()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        serde_json::from_value(value).map_err(|error| DomainIssue::new("decode", error.to_string()))
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        serde_json::to_value(value).map_err(|error| DomainIssue::new("encode", error.to_string()))
    }

    fn validate(&self, _value: &Self::Value) -> Result<(), DomainIssue> {
        Ok(())
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if matches!(schema_version.get(), 1 | 2) && value.is_object() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "invalid session document"))
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() != 1 {
            return Ok(None);
        }
        let mut value = value;
        value["panel_ratio"] = serde_json::json!(0.5);
        Ok(Some(MigrationStep {
            schema_version: SchemaVersion::new(2).unwrap(),
            value,
        }))
    }
}

fn store() -> (TempDir, ConfigStore) {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let paths = [
        root.join("config"),
        root.join("data"),
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("log"),
        root.join("backup"),
    ];
    for path in &paths {
        fs::create_dir_all(path).unwrap();
    }
    let roots = StorageRoots::new(
        paths[0].clone(),
        paths[1].clone(),
        paths[2].clone(),
        paths[3].clone(),
        paths[4].clone(),
        paths[5].clone(),
        paths[6].clone(),
    )
    .unwrap();
    let authority = CoordinationAuthority::new(&paths[1]).unwrap();
    (temp, ConfigStore::new(roots, authority))
}

fn placement(window_id: &str, width: u32) -> CapturedWindowPlacement {
    CapturedWindowPlacement::new(
        WindowId::new(window_id).unwrap(),
        WindowPlacement::new(ScreenPoint::new(10, 20), ScreenSize::new(width, 700)),
        false,
        CapturedDisplayAssociation::Unresolved,
    )
}

fn request(window_id: &str) -> WindowFlushRequest {
    request_within(window_id, 2_000)
}

fn request_within(window_id: &str, timeout_millis: u64) -> WindowFlushRequest {
    WindowFlushRequest::new(
        vec![WindowFlushTarget::new(
            WindowId::new(window_id).unwrap(),
            None,
        )],
        longhorn_windowing::WindowLifecycleDuration::from_millis(timeout_millis),
        WindowFlushScope::ApplicationShutdown,
    )
}

#[test]
fn config_sink_coalesces_and_preserves_other_domain_fields() {
    let (_temp, store) = store();
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        |session: &mut Session, placement| {
            session
                .placements
                .insert(placement.window_id().as_str().to_owned(), placement.clone());
            Ok(())
        },
    )
    .unwrap();
    sink.mutate(|session| {
        session.panel_ratio = 0.34;
        Ok(())
    })
    .unwrap();

    WindowPlacementSink::stage(&sink, &placement("window:main", 800)).unwrap();
    WindowPlacementSink::stage(&sink, &placement("window:main", 1_200)).unwrap();
    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request("window:main")),
        longhorn_windowing::WindowFlushOutcome::Succeeded
    );

    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load");
    };
    assert_eq!(loaded.value.panel_ratio, 0.34);
    assert_eq!(
        loaded.value.placements["window:main"]
            .normal_placement()
            .inner_size()
            .width(),
        1_200
    );
}

#[test]
fn flush_only_publishes_requested_windows() {
    let (_temp, store) = store();
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        |session: &mut Session, placement| {
            session
                .placements
                .insert(placement.window_id().as_str().to_owned(), placement.clone());
            Ok(())
        },
    )
    .unwrap();
    WindowPlacementSink::stage(&sink, &placement("window:a", 800)).unwrap();
    WindowPlacementSink::stage(&sink, &placement("window:b", 900)).unwrap();

    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request("window:b")),
        longhorn_windowing::WindowFlushOutcome::Succeeded
    );

    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load");
    };
    assert_eq!(
        loaded
            .value
            .placements
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["window:b"]
    );
}

#[test]
fn failed_mutation_keeps_the_staged_placement_for_retry() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let (_temp, store) = store();
    let fail_once = AtomicBool::new(true);
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        move |session: &mut Session, captured| {
            session
                .placements
                .insert("window:main".to_owned(), captured.clone());
            if fail_once.swap(false, Ordering::SeqCst) {
                return Err(DomainIssue::new("injected", "placement patch failed"));
            }
            Ok(())
        },
    )
    .unwrap();
    let captured = placement("window:main", 800);
    let request = request("window:main");
    sink.stage(&captured).unwrap();

    assert!(matches!(
        longhorn_windowing::flush_window_placements(&sink, &request),
        longhorn_windowing::WindowFlushOutcome::SinkFailed { .. }
    ));
    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request),
        longhorn_windowing::WindowFlushOutcome::Succeeded
    );
    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load after retry");
    };
    assert_eq!(loaded.value.placements["window:main"], captured);
}

#[test]
fn placement_staged_during_flush_is_published_by_the_next_flush() {
    use std::sync::{Mutex, mpsc};

    let (_temp, store) = store();
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (staged_tx, staged_rx) = mpsc::sync_channel(1);
    let gate = Mutex::new(Some(staged_rx));
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        move |session: &mut Session, captured| {
            if let Some(staged) = gate.lock().unwrap().take() {
                started_tx.send(()).unwrap();
                staged.recv_timeout(Duration::from_secs(2)).unwrap();
            }
            session
                .placements
                .insert("window:main".to_owned(), captured.clone());
            Ok(())
        },
    )
    .unwrap();

    let original = placement("window:main", 800);
    let newer = placement("window:main", 1_200);
    let request = request("window:main");
    sink.stage(&original).unwrap();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| longhorn_windowing::flush_window_placements(&sink, &request));
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        sink.stage(&newer).unwrap();
        staged_tx.send(()).unwrap();
        assert_eq!(
            first.join().unwrap(),
            longhorn_windowing::WindowFlushOutcome::Succeeded
        );
    });
    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load after the first flush");
    };
    assert_eq!(loaded.value.placements["window:main"], original);

    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request),
        longhorn_windowing::WindowFlushOutcome::Succeeded
    );
    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load after the second flush");
    };
    assert_eq!(loaded.value.placements["window:main"], newer);
}

#[test]
fn a_flush_whose_publication_outlasts_its_bound_reports_timed_out() {
    use std::sync::{Mutex, mpsc};

    use longhorn_windowing::WindowFlushOutcome;

    let (_temp, store) = store();
    // The first publication blocks inside the patch until the test releases
    // it. The gate is the only thing that holds it up.
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let gate = Mutex::new(Some(release_rx));
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        move |session: &mut Session, captured| {
            if let Some(release) = gate.lock().unwrap().take() {
                release.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            session
                .placements
                .insert("window:main".to_owned(), captured.clone());
            Ok(())
        },
    )
    .unwrap();
    let captured = placement("window:main", 800);
    sink.stage(&captured).unwrap();

    // The explicit bound is enforced, so the caller stops waiting after it.
    let started = std::time::Instant::now();
    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request_within("window:main", 50)),
        WindowFlushOutcome::TimedOut
    );
    assert!(started.elapsed() < Duration::from_secs(2));

    // While that publication still holds the flush, a second flush is bounded
    // too rather than queuing behind it.
    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request_within("window:main", 50)),
        WindowFlushOutcome::TimedOut
    );

    // Releasing the publication lets it finish. Placements staged before the
    // timeout are then durable, and a flush with room to wait confirms it.
    release_tx.send(()).unwrap();
    assert_eq!(
        longhorn_windowing::flush_window_placements(&sink, &request("window:main")),
        WindowFlushOutcome::Succeeded
    );
    let LoadOutcome::Ready(loaded) = sink.load().unwrap() else {
        panic!("session should load after the delayed publication");
    };
    assert_eq!(loaded.value.placements["window:main"], captured);
}

#[test]
fn migrated_domain_rewrite_publishes_the_new_schema_with_a_safety_backup() {
    use longhorn_config::{
        BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupCaptureOptions,
        BackupKind, BackupLimits, BackupMetadata, BackupOperationalRoot, BackupProducer,
        MigrationRewriteOptions, RestoreSafetyBackupOptions,
    };

    let (temp, store) = store();
    let file = temp.path().join("state/window-session.json");
    let original = serde_json::json!({
        "domain": "test.window-session",
        "schemaVersion": 1,
        "value": { "placements": {}, "panel_ratio": 0.25 }
    });
    fs::write(&file, serde_json::to_vec(&original).unwrap()).unwrap();
    let sink = ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        |_, _| Ok(()),
    )
    .unwrap();
    let backup = RestoreSafetyBackupOptions::new(
        BackupMetadata::new(
            "before-placement-migration",
            BackupKind::PreMigration,
            "2026-11-22T12:00:00Z",
            BackupApplication::new("com.example.desktop", "2").unwrap(),
            BackupProducer::new("longhorn-config", "2").unwrap(),
        )
        .unwrap(),
        BackupOperationalRoot::new(temp.path().join("backup")).unwrap(),
        BackupArchiveFileName::new("before-placement-migration.longhorn-backup").unwrap(),
        BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        BackupArchiveLimits::default(),
    );

    let receipt = sink
        .rewrite_migrated_domain(MigrationRewriteOptions::new(Duration::from_secs(2), backup))
        .unwrap();

    assert_eq!(receipt.from().get(), 1);
    assert_eq!(receipt.to().get(), 2);
    assert!(receipt.safety_backup().path.is_file());
    let rewritten: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
    assert_eq!(rewritten["schemaVersion"], 2);
    assert_eq!(rewritten["value"]["panel_ratio"], 0.5);
}
