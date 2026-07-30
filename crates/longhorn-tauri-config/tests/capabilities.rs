//! Narrow Tauri capability and dependency audit.

use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn permission_examples_name_exact_command_groups() {
    let read_permission = read("examples/permissions/read-config-operations.toml");
    let storage = read("examples/permissions/mutate-storage.toml");
    let backup = read("examples/permissions/mutate-backups.toml");
    let restore = read("examples/permissions/mutate-restore.toml");
    assert_eq!(read_permission.matches("\"longhorn_config_").count(), 3);
    assert_eq!(storage.matches("\"longhorn_config_").count(), 3);
    assert_eq!(backup.matches("\"longhorn_config_").count(), 3);
    assert_eq!(restore.matches("\"longhorn_config_").count(), 4);
    assert!(read_permission.contains("\"longhorn_config_snapshot\""));
    assert!(read_permission.contains("\"longhorn_config_storage_inspect\""));
    assert!(read_permission.contains("\"longhorn_config_restore_inspect\""));
    for command in ["storage_execute", "storage_recover", "storage_cleanup"] {
        assert!(storage.contains(&format!("\"longhorn_config_{command}\"")));
    }
    for command in ["backup_create", "backup_export", "backup_retention"] {
        assert!(backup.contains(&format!("\"longhorn_config_{command}\"")));
    }
    for command in [
        "restore_plan",
        "restore_execute",
        "restore_adapter_execute",
        "restore_recover",
    ] {
        assert!(restore.contains(&format!("\"longhorn_config_{command}\"")));
    }
}

#[test]
fn capability_examples_add_only_selected_authority() {
    let diagnostics: Value =
        serde_json::from_str(&read("examples/capabilities/config-diagnostics.json")).unwrap();
    let full: Value =
        serde_json::from_str(&read("examples/capabilities/config-operations.json")).unwrap();
    assert_eq!(
        diagnostics["permissions"],
        serde_json::json!(["allow-longhorn-config-read"])
    );
    assert_eq!(
        full["permissions"],
        serde_json::json!([
            "allow-longhorn-config-read",
            "allow-longhorn-storage-mutate",
            "allow-longhorn-backup-mutate",
            "allow-longhorn-restore-mutate"
        ])
    );
    assert_eq!(
        diagnostics["windows"],
        serde_json::json!(["main", "settings"])
    );
    assert_eq!(full["windows"], serde_json::json!(["settings"]));
}

#[test]
fn crate_dependency_edge_stays_narrow_and_secret_free() {
    let manifest = read("Cargo.toml");
    for forbidden in [
        "longhorn-config-age",
        "longhorn-settings",
        "longhorn-layout",
        "longhorn-surfaces",
        "longhorn-transfer",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    let sources = [
        read("src/authority.rs"),
        read("src/commands.rs"),
        read("src/ports.rs"),
    ]
    .join("\n");
    for forbidden in [
        "AgeIdentity",
        "AgePassphrase",
        "archive_bytes",
        "payload_bytes",
    ] {
        assert!(!sources.contains(forbidden), "{forbidden}");
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
