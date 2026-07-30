//! Narrow command host capability, dependency, and execution-bus audit.

use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn permission_examples_name_exact_read_and_mutation_commands() {
    let read_permission = read("examples/permissions/read-command.toml");
    let mutate = read("examples/permissions/mutate-command.toml");
    assert_eq!(read_permission.matches("\"longhorn_command_").count(), 2);
    assert_eq!(mutate.matches("\"longhorn_command_").count(), 3);
    for command in ["catalogue", "keymap"] {
        let exact = format!("\"longhorn_command_{command}\"");
        assert!(read_permission.contains(&exact));
        assert!(!mutate.contains(&exact));
    }
    for command in ["keymap_preview", "keymap_commit", "keymap_reset"] {
        let exact = format!("\"longhorn_command_{command}\"");
        assert!(mutate.contains(&exact));
        assert!(!read_permission.contains(&exact));
    }
}

#[test]
fn capability_examples_add_only_selected_commands_and_event_lifetime() {
    let read_only: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-command.json")).unwrap();
    let mutable: Value =
        serde_json::from_str(&read("examples/capabilities/mutable-command.json")).unwrap();
    assert_eq!(
        read_only["permissions"],
        serde_json::json!([
            "allow-longhorn-command-read",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        mutable["permissions"],
        serde_json::json!([
            "allow-longhorn-command-read",
            "allow-longhorn-command-mutate",
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
fn crate_and_handler_sources_exclude_product_execution_and_unrelated_domains() {
    let manifest = read("Cargo.toml");
    for forbidden in [
        "longhorn-bridge",
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
        "execute_command",
        "executeCommand",
        "CommandExecutionRequest",
        "command_id:",
        "commandId:",
    ] {
        assert!(!authority.contains(forbidden), "{forbidden}");
        assert!(!commands.contains(forbidden), "{forbidden}");
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
