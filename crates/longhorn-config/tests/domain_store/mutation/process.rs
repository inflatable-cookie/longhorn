use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use longhorn_config::{
    ConfigStore, CoordinationAuthority, CoordinationFailureKind, DebounceClock, DebouncePolicy,
    DebounceStrategy, DebouncedMutation, DomainIssue, DurabilityRequirement, LoadOutcome,
    MutationError, MutationOptions, StorageRoots,
};
use serde_json::json;

use crate::common::{Fixture, config_domain, document};

const HELPER_MODE: &str = "LONGHORN_CONFIG_HELPER_MODE";
const HELPER_ROOT: &str = "LONGHORN_CONFIG_HELPER_ROOT";
const HELPER_MARKER: &str = "LONGHORN_CONFIG_HELPER_MARKER";
const HELPER_DELAY_MS: &str = "LONGHORN_CONFIG_HELPER_DELAY_MS";

fn options(timeout: Duration) -> MutationOptions {
    MutationOptions::new(timeout, DurabilityRequirement::Atomic)
}

struct EnabledStrategy;

impl DebounceStrategy<crate::common::PreferencesDomain> for EnabledStrategy {
    type Intent = bool;

    fn coalesce(
        &self,
        _previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        Ok(next)
    }

    fn apply(
        &self,
        intent: &Self::Intent,
        value: &mut crate::common::Preferences,
    ) -> Result<(), DomainIssue> {
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
fn coordination_helper_process() {
    let Ok(mode) = env::var(HELPER_MODE) else {
        return;
    };
    let root = PathBuf::from(env::var_os(HELPER_ROOT).unwrap());
    let (roots, coordination) = authorities(&root);
    let domain = config_domain();
    let mut store = ConfigStore::new(roots, coordination);
    store.register(&domain).unwrap();
    let marker = env::var_os(HELPER_MARKER).map(PathBuf::from);
    let delay = env::var(HELPER_DELAY_MS)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    store
        .mutate(&domain, options(Duration::from_secs(10)), |value| {
            if let Some(marker) = &marker {
                fs::write(marker, b"locked").unwrap();
            }
            if mode == "hold" {
                thread::sleep(Duration::from_secs(30));
            } else if delay > 0 {
                thread::sleep(Duration::from_millis(delay));
            }
            value.name.push('x');
            Ok(())
        })
        .unwrap();
}

#[test]
fn helper_process_timeout_crash_release_and_persistent_lock_file() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "base", "enabled": true}),
        ),
    );
    let marker = fixture.temp.path().join("holder-ready");
    let mut child = spawn_helper(fixture.temp.path(), "hold", Some(&marker), 0);
    wait_for_marker(&mut child, &marker);

    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let error = store
        .mutate(&domain, options(Duration::from_millis(50)), |_value| Ok(()))
        .unwrap_err();
    let MutationError::Coordination(failure) = error else {
        panic!("expected coordination timeout");
    };
    assert_eq!(failure.kind, CoordinationFailureKind::Timeout);
    assert!(fixture.coordination.lock_path().exists());

    child.kill().unwrap();
    child.wait().unwrap();
    store
        .mutate(&domain, options(Duration::from_secs(2)), |value| {
            value.enabled = false;
            Ok(())
        })
        .unwrap();
    assert!(fixture.coordination.lock_path().exists());
}

#[test]
fn two_helper_processes_serialize_patch_updates() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "base", "enabled": true}),
        ),
    );
    let marker = fixture.temp.path().join("first-ready");
    let mut first = spawn_helper(fixture.temp.path(), "append", Some(&marker), 150);
    wait_for_marker(&mut first, &marker);
    let mut second = spawn_helper(fixture.temp.path(), "append", None, 0);

    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());

    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected current value");
    };
    assert_eq!(loaded.value.name, "basexx");
}

#[test]
fn debounced_flush_reconciles_with_an_intervening_process_mutation() {
    let fixture = Fixture::new();
    let domain = config_domain();
    fixture.write(
        &domain,
        &document(
            "example.preferences",
            3,
            json!({"name": "base", "enabled": true}),
        ),
    );
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let policy = DebouncePolicy::new(
        Duration::from_millis(200),
        size_of::<bool>(),
        options(Duration::from_secs(2)),
    )
    .unwrap();
    let mut lane =
        DebouncedMutation::new(&store, &domain, EnabledStrategy, FixedClock, policy).unwrap();
    lane.stage(false).unwrap();

    let mut helper = spawn_helper(fixture.temp.path(), "append", None, 0);
    assert!(helper.wait().unwrap().success());
    lane.flush_forced();

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected current value");
    };
    assert_eq!(loaded.value.name, "basex");
    assert!(!loaded.value.enabled);
}

fn authorities(root: &Path) -> (StorageRoots, CoordinationAuthority) {
    let config = root.join("config");
    let data = root.join("data");
    let roots = StorageRoots::new(
        config,
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
    let coordination = CoordinationAuthority::new(data).unwrap();
    (roots, coordination)
}

fn spawn_helper(root: &Path, mode: &str, marker: Option<&Path>, delay_ms: u64) -> Child {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("mutation::process::coordination_helper_process")
        .arg("--nocapture")
        .env(HELPER_MODE, mode)
        .env(HELPER_ROOT, root)
        .env(HELPER_DELAY_MS, delay_ms.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(marker) = marker {
        command.env(HELPER_MARKER, marker);
    }
    command.spawn().unwrap()
}

fn wait_for_marker(child: &mut Child, marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("helper exited before acquiring lock: {status}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    panic!("helper did not acquire lock");
}
