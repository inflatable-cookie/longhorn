//! Capability examples stay narrow and auditable.

use std::{fs, path::PathBuf};

use serde_json::Value;

const BASE_COMMANDS: [&str; 5] = [
    "longhorn_transfer_snapshot",
    "longhorn_transfer_start_panel",
    "longhorn_transfer_publish_lease",
    "longhorn_transfer_commit_panel",
    "longhorn_transfer_cancel",
];
const SURFACE_COMMANDS: [&str; 2] = [
    "longhorn_transfer_start_surface",
    "longhorn_transfer_commit_surface",
];

#[test]
fn permission_examples_name_only_the_public_commands() {
    let base = read("examples/permissions/base-transfer.toml");
    let surface = read("examples/permissions/surface-transfer.toml");

    assert_eq!(base.matches("\"longhorn_transfer_").count(), 5);
    assert_eq!(surface.matches("\"longhorn_transfer_").count(), 2);
    for command in BASE_COMMANDS {
        assert!(base.contains(&format!("\"{command}\"")));
        assert!(!surface.contains(&format!("\"{command}\"")));
    }
    for command in SURFACE_COMMANDS {
        assert!(surface.contains(&format!("\"{command}\"")));
        assert!(!base.contains(&format!("\"{command}\"")));
    }
}

#[test]
fn capability_examples_add_only_event_listen_and_selected_command_sets() {
    let base: Value =
        serde_json::from_str(&read("examples/capabilities/base-transfer.json")).unwrap();
    let surface: Value =
        serde_json::from_str(&read("examples/capabilities/surface-transfer.json")).unwrap();

    assert_eq!(
        base["permissions"],
        serde_json::json!([
            "allow-longhorn-transfer-base",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        surface["permissions"],
        serde_json::json!([
            "allow-longhorn-transfer-base",
            "allow-longhorn-transfer-surface",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
