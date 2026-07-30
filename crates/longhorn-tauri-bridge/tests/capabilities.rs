//! Minimal query-only and subscription Tauri capability inventory.

use std::{fs, path::Path};

use serde_json::{Value, json};

#[test]
fn permissions_admit_only_the_stable_generic_command_inventory() {
    let query = read("examples/permissions/query-bridge.toml");
    let mutate = read("examples/permissions/mutate-bridge.toml");

    for command in [
        "longhorn_bridge_hello",
        "longhorn_bridge_authority",
        "longhorn_bridge_query",
        "longhorn_bridge_resync",
    ] {
        assert!(query.contains(command));
    }
    for command in ["longhorn_bridge_command", "longhorn_bridge_cancel"] {
        assert!(!query.contains(command));
        assert!(mutate.contains(command));
    }
    assert!(!query.contains("workspace"));
    assert!(!mutate.contains("workspace"));
}

#[test]
fn query_only_capability_has_no_event_or_mutation_admission() {
    let query_only = json_file("examples/capabilities/query-only.json");
    let subscription = json_file("examples/capabilities/subscription.json");

    assert_eq!(
        query_only["permissions"],
        json!(["allow-longhorn-bridge-query"])
    );
    assert_eq!(
        subscription["permissions"],
        json!([
            "allow-longhorn-bridge-query",
            "allow-longhorn-bridge-mutate",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative)).unwrap()
}

fn json_file(relative: &str) -> Value {
    serde_json::from_str(&read(relative)).unwrap()
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
