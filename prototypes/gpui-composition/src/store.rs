//! A real placement store, so a flush takes real time.
//!
//! Card 176's first step. Every teardown proof so far has used an in-memory
//! sink that answers synchronously, which means the whole class of "the answer
//! arrives after the window is gone" has never been exercised.
//!
//! This is `ConfigWindowPlacementSink` over a real `ConfigStore`: coordinated
//! mutation, atomic publish, a file on disk. `longhorn-windowing-config` takes
//! no host adapter, so a GPUI application uses it unchanged — which is worth
//! saying, because until this example its doc comment claimed it was for Tauri.
//!
//! Rooted under a directory this example owns rather than a temporary one:
//! the question Card 176 asks is whether a placement survives a *restart*, and
//! a store that evaporates on drop cannot answer it.

use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, DurabilityRequirement, LoadOutcome, MigrationStep, MutationOptions, StorageClass,
    StorageRoots,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_windowing::CapturedWindowPlacement;
use longhorn_windowing_config::ConfigWindowPlacementSink;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What this example persists: one placement per window.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Session {
    /// Captured placements, keyed by window id.
    pub placements: BTreeMap<String, CapturedWindowPlacement>,
}

/// The example's one configuration domain.
pub struct SessionDomain {
    descriptor: DomainDescriptor,
}

impl SessionDomain {
    /// Registers the domain under a stable id.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("example.gpui-session").expect("domain id"),
                SchemaVersion::new(1).expect("schema version"),
                // Machine state, not config: where a window sat is a property
                // of this machine and does not belong in anything synced.
                StorageClass::MachineState,
                Some(DomainFilePath::new("gpui-session.json").expect("file path")),
            )
            .expect("descriptor"),
        }
    }
}

impl Default for SessionDomain {
    fn default() -> Self {
        Self::new()
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

/// Where this example keeps its roots.
///
/// Under the target directory so a `cargo clean` disposes of it, and stable
/// across runs so a restart reads what the previous run wrote.
#[must_use]
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("example-store")
}

/// Builds the placement sink over a real store.
///
/// The patch is the application's, as the contract intends: Longhorn stages
/// and coalesces, the product decides what its own document looks like.
#[must_use]
pub fn sink() -> ConfigWindowPlacementSink<SessionDomain> {
    let root = root();
    let names = [
        "config", "data", "state", "cache", "runtime", "logs", "backups",
    ];
    let paths: Vec<PathBuf> = names.iter().map(|name| root.join(name)).collect();
    for path in &paths {
        fs::create_dir_all(path).expect("storage root");
    }

    let roots = StorageRoots::new(
        &paths[0], &paths[1], &paths[2], &paths[3], &paths[4], &paths[5], &paths[6],
    )
    .expect("storage roots");
    let coordination = CoordinationAuthority::new(&paths[1]).expect("coordination authority");
    let store = ConfigStore::new(roots, coordination);

    ConfigWindowPlacementSink::new(
        store,
        SessionDomain::new(),
        // A real durability requirement. `Atomic` is what makes the write take
        // measurable time, which is the entire point of this module.
        MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic),
        |session: &mut Session, placement: &CapturedWindowPlacement| {
            session
                .placements
                .insert(placement.window_id().as_str().to_owned(), placement.clone());
            Ok(())
        },
    )
    .expect("sink registers its domain")
}

/// Reads back what the last run persisted.
///
/// The restart half of Card 176: a placement that reached the file is a
/// placement that survived, and one that did not is the defect the card is
/// looking for.
#[must_use]
pub fn persisted() -> Session {
    let root = root();
    let names = [
        "config", "data", "state", "cache", "runtime", "logs", "backups",
    ];
    let paths: Vec<PathBuf> = names.iter().map(|name| root.join(name)).collect();
    if paths.iter().any(|path| !path.exists()) {
        return Session::default();
    }

    let Ok(roots) = StorageRoots::new(
        &paths[0], &paths[1], &paths[2], &paths[3], &paths[4], &paths[5], &paths[6],
    ) else {
        return Session::default();
    };
    let Ok(coordination) = CoordinationAuthority::new(&paths[1]) else {
        return Session::default();
    };

    let mut store = ConfigStore::new(roots, coordination);
    let domain = SessionDomain::new();
    if store.register(&domain).is_err() {
        return Session::default();
    }

    match store.load(&domain) {
        Ok(LoadOutcome::Ready(loaded)) => loaded.value,
        _ => Session::default(),
    }
}

/// Persists one window's current placement and returns how long the store took.
///
/// The measurement Card 176 asks for. Every teardown proof before this used a
/// sink that answered synchronously; this is what a real coordinated,
/// atomically published write actually costs, which is the number that decides
/// whether "the answer arrives after the window is gone" is a real risk.
pub fn persist_now(
    sink: &ConfigWindowPlacementSink<SessionDomain>,
    placement: &CapturedWindowPlacement,
) -> Result<Duration, String> {
    use longhorn_windowing::{
        WindowFlushRequest, WindowFlushScope, WindowFlushTarget, WindowLifecycleDuration,
        WindowPlacementSink as _,
    };

    let started = std::time::Instant::now();
    sink.stage(placement)?;
    let request = WindowFlushRequest::new(
        vec![WindowFlushTarget::new(placement.window_id().clone(), None)],
        WindowLifecycleDuration::from_millis(2_000),
        // Shutdown scope: this is the write a teardown depends on.
        WindowFlushScope::ApplicationShutdown,
    );
    let ticket = sink.request_flush(&request)?;
    drop(ticket);
    Ok(started.elapsed())
}
