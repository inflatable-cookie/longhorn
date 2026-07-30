//! Narrow Tauri capability and dependency audit.

use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn permission_examples_name_exact_read_and_mutation_commands() {
    let read_permission = read("examples/permissions/read-settings.toml");
    let mutate = read("examples/permissions/mutate-settings.toml");
    assert_eq!(read_permission.matches("\"longhorn_settings_").count(), 2);
    assert_eq!(mutate.matches("\"longhorn_settings_").count(), 2);
    for command in ["registry", "load"] {
        assert!(read_permission.contains(&format!("\"longhorn_settings_{command}\"")));
        assert!(!mutate.contains(&format!("\"longhorn_settings_{command}\"")));
    }
    for command in ["apply", "reset"] {
        assert!(mutate.contains(&format!("\"longhorn_settings_{command}\"")));
        assert!(!read_permission.contains(&format!("\"longhorn_settings_{command}\"")));
    }
}

#[test]
fn capability_examples_add_only_selected_commands_and_event_lifetime() {
    let read_only: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-settings.json")).unwrap();
    let mutable: Value =
        serde_json::from_str(&read("examples/capabilities/mutable-settings.json")).unwrap();
    assert_eq!(
        read_only["permissions"],
        serde_json::json!([
            "allow-longhorn-settings-read",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        mutable["permissions"],
        serde_json::json!([
            "allow-longhorn-settings-read",
            "allow-longhorn-settings-mutate",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        read_only["windows"],
        serde_json::json!(["main", "settings"])
    );
    assert_eq!(mutable["windows"], serde_json::json!(["settings"]));
}

#[test]
fn crate_dependency_edge_stays_narrow() {
    let manifest = read("Cargo.toml");
    for forbidden in [
        "longhorn-layout",
        "longhorn-surfaces",
        "longhorn-transfer",
        "longhorn-settings-config",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
