//! Discovery lifecycle fixtures (Card 228): create, enumerate, stale-pid
//! detection, idempotent remove, and contract 004 path resolution.

use std::path::Path;

use longhorn_agent_control::{
    DISCOVERY_SCHEMA_VERSION, DiscoveryFile, InstanceToken, enumerate_discovery,
    enumerate_discovery_with, process_alive, publish_discovery, remove_discovery_file,
    resolve_discovery_dir, resolve_discovery_dir_with_state_override, sweep_stale_discovery,
};
use longhorn_config::{PlatformDirectoryFacts, TargetPlatform};
use tempfile::TempDir;

fn facts(platform: TargetPlatform, root: &Path) -> PlatformDirectoryFacts {
    PlatformDirectoryFacts::complete(
        platform,
        root.join("config"),
        root.join("data"),
        root.join("state"),
        root.join("cache"),
        root.join("log"),
        root.join("runtime"),
    )
}

#[test]
fn discovery_dir_follows_platform_native_state_root_rules() {
    let root = TempDir::new().unwrap();

    let linux = resolve_discovery_dir(&facts(TargetPlatform::Linux, root.path())).unwrap();
    assert_eq!(
        linux.strip_prefix(root.path()).unwrap(),
        Path::new("state/longhorn/agent-control")
    );

    let macos = resolve_discovery_dir(&facts(TargetPlatform::MacOs, root.path())).unwrap();
    assert_eq!(
        macos.strip_prefix(root.path()).unwrap(),
        Path::new("state/longhorn/state/agent-control")
    );

    let windows = resolve_discovery_dir(&facts(TargetPlatform::Windows, root.path())).unwrap();
    assert_eq!(
        windows.strip_prefix(root.path()).unwrap(),
        Path::new("state/longhorn/state/agent-control")
    );
}

#[test]
fn state_override_is_an_explicit_root_not_a_parallel_one() {
    let root = TempDir::new().unwrap();
    let state = root.path().join("injected-state");
    let dir = resolve_discovery_dir_with_state_override(
        &facts(TargetPlatform::Linux, root.path()),
        &state,
    )
    .unwrap();
    assert_eq!(dir, state.join("agent-control"));
}

#[test]
fn create_enumerate_and_remove_round_trip() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let token = InstanceToken::generate().unwrap();

    // A missing directory enumerates as empty.
    let scan = enumerate_discovery(&dir).unwrap();
    assert!(scan.instances.is_empty());
    assert!(scan.unreadable.is_empty());

    let instance = publish_discovery(&dir, "dev.example.soundcheck", 49152, token.clone()).unwrap();
    assert_eq!(
        instance.path().file_name().unwrap().to_str().unwrap(),
        format!("dev.example.soundcheck-{}.json", std::process::id())
    );

    let scan = enumerate_discovery(&dir).unwrap();
    assert_eq!(scan.instances.len(), 1);
    assert!(scan.unreadable.is_empty());
    let record = &scan.instances[0];
    assert!(record.is_live());
    assert!(!record.is_stale());
    assert_eq!(record.file().schema_version, DISCOVERY_SCHEMA_VERSION);
    assert_eq!(record.file().app_id, "dev.example.soundcheck");
    assert_eq!(record.file().pid, std::process::id());
    assert_eq!(record.file().port, 49152);
    assert_eq!(record.file().token, token);

    instance.remove().unwrap();
    assert!(enumerate_discovery(&dir).unwrap().instances.is_empty());
}

#[test]
fn stale_pid_is_detectable_by_the_enumerator() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let instance = publish_discovery(
        &dir,
        "dev.example.nucleus",
        49153,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();

    // Injected liveness proves the marking logic deterministically.
    let scan = enumerate_discovery_with(&dir, |_| false).unwrap();
    assert!(scan.instances[0].is_stale());
    let scan = enumerate_discovery_with(&dir, |pid| pid == std::process::id()).unwrap();
    assert!(scan.instances[0].is_live());

    // The real probe agrees: this process is alive.
    assert!(process_alive(std::process::id()));

    instance.remove().unwrap();
}

#[cfg(unix)]
#[test]
fn a_dead_pid_marks_the_file_stale_under_the_real_probe() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");

    let mut child = std::process::Command::new("sh")
        .args(["-c", "sleep 60"])
        .spawn()
        .unwrap();
    let dead_pid = child.id();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!process_alive(dead_pid));

    // Forge the file a crashed instance of that pid would have left.
    let file = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.loophole".to_owned(),
        pid: dead_pid,
        port: 49154,
        token: InstanceToken::generate().unwrap(),
    };
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("dev.example.loophole-{dead_pid}.json"));
    std::fs::write(&path, serde_json::to_string(&file).unwrap()).unwrap();

    let scan = enumerate_discovery(&dir).unwrap();
    assert_eq!(scan.instances.len(), 1);
    assert!(scan.instances[0].is_stale());
    assert_eq!(scan.instances[0].file().pid, dead_pid);
}

#[cfg(unix)]
#[test]
fn sweep_unlinks_discovery_files_with_a_dead_pid() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");

    let mut child = std::process::Command::new("sh")
        .args(["-c", "sleep 60"])
        .spawn()
        .unwrap();
    let dead_pid = child.id();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!process_alive(dead_pid));

    let live = publish_discovery(
        &dir,
        "dev.example.live",
        49160,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();
    let dead_path = dir.join(format!("dev.example.dead-{dead_pid}.json"));
    let dead = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.dead".to_owned(),
        pid: dead_pid,
        port: 49161,
        token: InstanceToken::generate().unwrap(),
    };
    std::fs::write(&dead_path, serde_json::to_string(&dead).unwrap()).unwrap();

    let removed = sweep_stale_discovery(&dir).unwrap();
    assert_eq!(removed, 1);
    assert!(!dead_path.exists());
    assert!(live.path().exists());

    let scan = enumerate_discovery(&dir).unwrap();
    assert_eq!(scan.instances.len(), 1);
    assert!(scan.instances[0].is_live());
    assert_eq!(scan.instances[0].file().pid, std::process::id());

    live.remove().unwrap();
}

#[test]
fn publish_sweeps_dead_pid_files_before_writing() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    std::fs::create_dir_all(&dir).unwrap();

    // A pid that is not this process and is overwhelmingly not running.
    let dead_pid = std::process::id().wrapping_add(100_000).max(2);
    assert_ne!(dead_pid, std::process::id());
    assert!(
        !process_alive(dead_pid),
        "fixture needs a dead pid; {dead_pid} is live"
    );

    let dead_path = dir.join(format!("dev.example.ghost-{dead_pid}.json"));
    let dead = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.ghost".to_owned(),
        pid: dead_pid,
        port: 49162,
        token: InstanceToken::generate().unwrap(),
    };
    std::fs::write(&dead_path, serde_json::to_string(&dead).unwrap()).unwrap();

    let live = publish_discovery(
        &dir,
        "dev.example.fresh",
        49163,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();

    assert!(!dead_path.exists());
    assert!(live.path().exists());
    let scan = enumerate_discovery_with(&dir, |pid| pid == std::process::id()).unwrap();
    assert_eq!(scan.instances.len(), 1);
    assert!(scan.instances[0].is_live());

    live.remove().unwrap();
}

#[test]
fn remove_is_idempotent() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let instance = publish_discovery(
        &dir,
        "dev.example.jetstream",
        49155,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();
    let path = instance.path().to_path_buf();

    remove_discovery_file(&path).unwrap();
    remove_discovery_file(&path).unwrap();
    instance.remove().unwrap();
}

#[test]
fn foreign_and_mismatched_files_never_block_enumeration() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let instance = publish_discovery(
        &dir,
        "dev.example.finch",
        49156,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();

    // Garbage, a wrong-schema file, and a name/pid mismatch all sort into
    // `unreadable` while the real record still enumerates.
    std::fs::write(dir.join("notes.txt"), b"not a discovery file").unwrap();
    std::fs::write(dir.join("junk.json"), b"{ not json").unwrap();
    let mut future = serde_json::to_value(instance.file()).unwrap();
    future["schemaVersion"] = serde_json::json!(DISCOVERY_SCHEMA_VERSION + 1);
    std::fs::write(
        dir.join(format!("dev.example.finch-{}.json", std::process::id() + 1)),
        serde_json::to_string(&future).unwrap(),
    )
    .unwrap();
    let mismatched = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.finch".to_owned(),
        pid: std::process::id() + 2,
        port: 49157,
        token: InstanceToken::generate().unwrap(),
    };
    std::fs::write(
        dir.join(format!("dev.example.finch-{}.json", std::process::id() + 3)),
        serde_json::to_string(&mismatched).unwrap(),
    )
    .unwrap();

    let scan = enumerate_discovery_with(&dir, |_| true).unwrap();
    assert_eq!(scan.instances.len(), 1);
    assert_eq!(scan.instances[0].file().port, 49156);
    assert_eq!(scan.unreadable.len(), 3);
}

#[test]
fn unsafe_app_ids_cannot_escape_the_directory() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    for app_id in ["../escape", "a/b", "", ".."] {
        assert!(
            publish_discovery(&dir, app_id, 49158, InstanceToken::generate().unwrap()).is_err(),
            "app id {app_id:?} must be rejected"
        );
    }
}

#[cfg(unix)]
#[test]
fn discovery_file_permissions_match_credential_class() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new().unwrap();
    let dir = root.path().join("agent-control");
    let instance = publish_discovery(
        &dir,
        "dev.example.split-shell",
        49159,
        InstanceToken::generate().unwrap(),
    )
    .unwrap();

    let file_mode = std::fs::metadata(instance.path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(file_mode, 0o600);
    let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700);

    instance.remove().unwrap();
}
