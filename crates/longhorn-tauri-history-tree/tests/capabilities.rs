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
