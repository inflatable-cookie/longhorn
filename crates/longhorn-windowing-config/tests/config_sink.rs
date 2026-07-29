//! Config-backed window placement sink integration tests.

use std::{collections::BTreeMap, fs, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, DurabilityRequirement, LoadOutcome, MigrationStep, MutationOptions, StorageClass,
    StorageRoots,
};
use longhorn_core::{DomainId, SchemaVersion, ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_tauri_windowing::{
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
                SchemaVersion::new(1).unwrap(),
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
        if schema_version.get() == 1 && value.is_object() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "invalid session document"))
        }
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
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
    WindowFlushRequest::new(
        vec![WindowFlushTarget::new(
            WindowId::new(window_id).unwrap(),
            None,
        )],
        longhorn_windowing::WindowLifecycleDuration::from_millis(2_000),
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
    WindowPlacementSink::request_flush(&sink, &request("window:main")).unwrap();

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

    WindowPlacementSink::request_flush(&sink, &request("window:b")).unwrap();

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
