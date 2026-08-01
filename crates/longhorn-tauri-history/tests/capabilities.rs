//! Narrow history host capability and dependency audit.

use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn permission_examples_name_exact_read_and_navigation_commands() {
    let read_permission = read("examples/permissions/read-history.toml");
    let mutate = read("examples/permissions/mutate-history.toml");
    assert_eq!(read_permission.matches("\"longhorn_history_").count(), 2);
    assert_eq!(mutate.matches("\"longhorn_history_").count(), 1);
    for command in ["snapshot", "page"] {
        let exact = format!("\"longhorn_history_{command}\"");
        assert!(read_permission.contains(&exact));
        assert!(!mutate.contains(&exact));
    }
    assert!(mutate.contains("\"longhorn_history_navigate\""));
    assert!(!read_permission.contains("\"longhorn_history_navigate\""));
}

#[test]
fn capability_examples_add_only_selected_commands_and_event_lifetime() {
    let read_only: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-history.json")).unwrap();
    let mutable: Value =
        serde_json::from_str(&read("examples/capabilities/mutable-history.json")).unwrap();
    assert_eq!(
        read_only["permissions"],
        serde_json::json!([
            "allow-longhorn-history-read",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        mutable["permissions"],
        serde_json::json!([
            "allow-longhorn-history-read",
            "allow-longhorn-history-mutate",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(read_only["windows"], serde_json::json!(["main", "history"]));
    assert_eq!(mutable["windows"], serde_json::json!(["history"]));
}

#[test]
fn crate_excludes_product_payload_and_unrelated_domains() {
    let manifest = read("Cargo.toml");
    for forbidden in [
        "longhorn-bridge",
        "longhorn-command",
        "longhorn-config",
        "longhorn-layout",
        "longhorn-settings",
        "longhorn-surfaces",
        "longhorn-transfer",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    let authority = read("src/authority.rs");
    let commands = read("src/commands.rs");
    for forbidden in [
        "payload:",
        "payload,",
        "Pulse",
        "project_revision",
        "projectRevision",
    ] {
        assert!(!authority.contains(forbidden), "{forbidden}");
        assert!(!commands.contains(forbidden), "{forbidden}");
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
