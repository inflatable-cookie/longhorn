use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use longhorn_config::{
    BackupCatalog, BackupScope, ConfigDomain, ConfigStore, CoordinationAuthority,
    CoordinationFailureKind, DebounceClock, DebouncePolicy, DebounceStrategy, DebouncedMutation,
    DomainDescriptor, DomainIssue, DurabilityRequirement, FlushOutcome, MigrationStep,
    MutationOptions, StorageRoots,
};
use longhorn_core::SchemaVersion;
use serde_json::{Value, json};

use crate::common::{Fixture, Preferences, PreferencesDomain, config_domain, document};

use super::capture;

const HELPER_MODE: &str = "LONGHORN_BACKUP_HELPER_MODE";
const HELPER_ROOT: &str = "LONGHORN_BACKUP_HELPER_ROOT";
const HELPER_MARKER: &str = "LONGHORN_BACKUP_HELPER_MARKER";
const HELPER_RELEASE: &str = "LONGHORN_BACKUP_HELPER_RELEASE";
const HELPER_START_TIMEOUT: Duration = Duration::from_secs(60);
const HELPER_RELEASE_TIMEOUT: Duration = Duration::from_secs(120);

struct EnabledStrategy;

impl DebounceStrategy<PreferencesDomain> for EnabledStrategy {
    type Intent = bool;

    fn coalesce(
        &self,
        _previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        Ok(next)
    }

    fn apply(&self, intent: &Self::Intent, value: &mut Preferences) -> Result<(), DomainIssue> {
        value.enabled = *intent;
        Ok(())
    }

    fn pending_weight(&self, _intent: &Self::Intent) -> usize {
        size_of::<bool>()
    }
}

struct FixedClock;

impl DebounceClock for FixedClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

#[test]
fn pending_debounce_is_absent_until_the_host_forces_flush() {
    let fixture = Fixture::new();
    let domain = config_domain();
    let bytes = document(
        "example.preferences",
        3,
        json!({"name": "published", "enabled": true}),
    );
    fixture.write(&domain, &bytes);
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();
    let debounce_policy = DebouncePolicy::new(
        Duration::from_secs(1),
        size_of::<bool>(),
        MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
    )
    .unwrap();
    let mut lane = DebouncedMutation::new(
        &store,
        &domain,
        EnabledStrategy,
        FixedClock,
        debounce_policy,
    )
    .unwrap();
    lane.stage(false).unwrap();

    let before = capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();
    assert_eq!(before.payloads()[0].bytes(), bytes);
    assert!(matches!(lane.flush_forced(), FlushOutcome::Terminal(_)));

    let after = capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();
    let parsed: Value = serde_json::from_slice(after.payloads()[0].bytes()).unwrap();
    assert_eq!(parsed["value"]["enabled"], false);
}

#[test]
fn capture_releases_the_guard_before_return() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "published", "enabled": true}),
        ),
    );
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();
    capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();

    store
        .mutate(
            &domain,
            MutationOptions::new(Duration::from_millis(50), DurabilityRequirement::Atomic),
            |value| {
                value.name = "after-capture".into();
                Ok(())
            },
        )
        .unwrap();
}

struct SlowDomain {
    inner: PreferencesDomain,
    marker: Option<PathBuf>,
    release: Option<PathBuf>,
}

impl SlowDomain {
    fn new(marker: Option<PathBuf>, release: Option<PathBuf>) -> Self {
        Self {
            inner: config_domain(),
            marker,
            release,
        }
    }
}

impl ConfigDomain for SlowDomain {
    type Value = Preferences;

    fn descriptor(&self) -> &DomainDescriptor {
        self.inner.descriptor()
    }

    fn default_value(&self) -> Self::Value {
        self.inner.default_value()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        self.inner.decode(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        self.inner.encode(value)
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        self.inner.validate(value)
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if let Some(marker) = &self.marker {
            fs::write(marker, b"capture-locked").unwrap();
            wait_for_release(self.release.as_deref().unwrap());
        }
        self.inner.validate_raw(schema_version, value)
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        self.inner.migrate_one(from, value)
    }
}

#[test]
fn capture_helper_process() {
    let Ok(mode) = env::var(HELPER_MODE) else {
        return;
    };
    assert_eq!(mode, "capture");
    let root = PathBuf::from(env::var_os(HELPER_ROOT).unwrap());
    let marker = PathBuf::from(env::var_os(HELPER_MARKER).unwrap());
    let release = PathBuf::from(env::var_os(HELPER_RELEASE).unwrap());
    let (roots, coordination) = authorities(&root);
    let domain = SlowDomain::new(Some(marker), Some(release));
    let mut store = ConfigStore::new(roots, coordination);
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();
    capture(&store, &catalog, &BackupScope::AllRegistered).unwrap();
}

#[test]
fn helper_process_mutation_waits_for_the_capture_cut() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "published", "enabled": true}),
        ),
    );
    let marker = fixture.temp.path().join("capture-locked");
    let release = fixture.temp.path().join("capture-release");
    let release_on_drop = ReleaseOnDrop(release.clone());
    let mut child = spawn_capture_helper(fixture.temp.path(), &marker, &release);
    wait_for_marker(&mut child, &marker);

    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let result = store.mutate(
        &domain,
        MutationOptions::new(Duration::from_millis(40), DurabilityRequirement::Atomic),
        |value| {
            value.name = "interleaved".into();
            Ok(())
        },
    );
    drop(release_on_drop);
    let error = result.unwrap_err();
    let longhorn_config::MutationError::Coordination(failure) = error else {
        panic!("expected coordination timeout");
    };
    assert_eq!(failure.kind, CoordinationFailureKind::Timeout);
    assert!(child.wait().unwrap().success());

    store
        .mutate(
            &domain,
            MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
            |value| {
                value.name = "after-release".into();
                Ok(())
            },
        )
        .unwrap();
}

fn authorities(root: &Path) -> (StorageRoots, CoordinationAuthority) {
    let data = root.join("data");
    let roots = StorageRoots::new(
        root.join("config"),
        &data,
        root.join("state"),
        root.join("cache"),
        root.join("runtime"),
        root.join("log"),
        root.join("backups"),
    )
    .unwrap()
    .with_policy(root.join("policy"))
    .unwrap()
    .with_workspace(root.join("workspace"))
    .unwrap()
    .with_project(root.join("project"))
    .unwrap();
    (roots, CoordinationAuthority::new(data).unwrap())
}

fn spawn_capture_helper(root: &Path, marker: &Path, release: &Path) -> Child {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("backup::coordination::capture_helper_process")
        .arg("--nocapture")
        .env(HELPER_MODE, "capture")
        .env(HELPER_ROOT, root)
        .env(HELPER_MARKER, marker)
        .env(HELPER_RELEASE, release)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_for_marker(child: &mut Child, marker: &Path) {
    let deadline = Instant::now() + HELPER_START_TIMEOUT;
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("capture helper exited before acquiring lock: {status}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("capture helper did not acquire lock");
}

struct ReleaseOnDrop(PathBuf);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        let _ = fs::write(&self.0, b"release");
    }
}

fn wait_for_release(release: &Path) {
    let deadline = Instant::now() + HELPER_RELEASE_TIMEOUT;
    while !release.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(release.exists(), "parent did not release capture helper");
}
