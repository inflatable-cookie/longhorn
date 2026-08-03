//! Exact Tauri capability and dependency boundary evidence.

use std::{fs, path::PathBuf};
#[test]
fn permissions_split_three_reads_from_navigation() {
    let read = file("examples/permissions/read-history-tree.toml");
    let mutate = file("examples/permissions/mutate-history-tree.toml");
    assert_eq!(read.matches("\"longhorn_history_tree_").count(), 3);
    assert_eq!(mutate.matches("\"longhorn_history_tree_").count(), 1);
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
