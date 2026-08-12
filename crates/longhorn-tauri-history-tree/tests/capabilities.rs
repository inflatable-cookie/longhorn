//! Exact Tauri capability and dependency boundary evidence.

use std::{fs, path::PathBuf};
/// The count is pinned so a command added to the crate cannot quietly miss the
/// capability a host copies. Card 183 added `continuations` and did exactly
/// that: the command shipped, the example did not list it, and a host copying
/// the example would have been denied at runtime by a message naming a
/// permission rather than a missing command. Four reads since.
#[test]
fn permissions_split_reads_from_navigation() {
    let read = file("examples/permissions/read-history-tree.toml");
    let mutate = file("examples/permissions/mutate-history-tree.toml");
    assert_eq!(read.matches("\"longhorn_history_tree_").count(), 4);
    assert_eq!(mutate.matches("\"longhorn_history_tree_").count(), 1);
    for command in [
        "longhorn_history_tree_snapshot",
        "longhorn_history_tree_path",
        "longhorn_history_tree_branches",
        "longhorn_history_tree_continuations",
    ] {
        assert!(
            read.contains(command),
            "{command} is not in the read example"
        );
    }
    assert!(!read.contains("navigate"));
    assert!(mutate.contains("longhorn_history_tree_navigate"));
}

/// Card 185. Deletion is irreversible, so permission to move through history
/// must not carry permission to destroy it. A host grants this one to the
/// window that confirms the deletion, and to no other.
#[test]
fn deletion_is_its_own_capability() {
    let delete = file("examples/permissions/delete-history-tree.toml");
    let mutate = file("examples/permissions/mutate-history-tree.toml");
    let read = file("examples/permissions/read-history-tree.toml");
    assert_eq!(delete.matches("\"longhorn_history_tree_").count(), 2);
    assert!(delete.contains("longhorn_history_tree_delete_continuation"));
    // Card 186: pruning removes entries too, so it sits with deletion rather
    // than with navigation.
    assert!(delete.contains("longhorn_history_tree_prune"));
    for other in [&mutate, &read] {
        assert!(
            !other.contains("delete") && !other.contains("prune"),
            "destructive commands must not ride along with another permission"
        );
    }
    let capability = file("examples/capabilities/destructive-history-tree.json");
    assert!(capability.contains("allow-longhorn-history-tree-delete"));
    assert_eq!(
        capability.matches("allow-longhorn-history-tree-").count(),
        1,
        "the destructive capability grants nothing else"
    );
}
#[test]
fn adapter_manifest_excludes_payload_and_unrelated_domains() {
    let manifest = file("Cargo.toml");
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
    let source = format!("{}{}", file("src/authority.rs"), file("src/commands.rs"));
    assert!(!source.to_ascii_lowercase().contains("payload"));
}
fn file(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
