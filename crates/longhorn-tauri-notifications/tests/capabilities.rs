//! Example capability separation audits.

use std::{fs, path::PathBuf};

#[test]
fn example_capabilities_keep_read_and_manage_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let read = fs::read_to_string(root.join("examples/capabilities/read-only-notifications.json"))
        .expect("read capability");
    let manage = fs::read_to_string(root.join("examples/capabilities/managed-notifications.json"))
        .expect("manage capability");
    assert!(read.contains("read-notifications"));
    assert!(!read.contains("manage-notifications"));
    assert!(manage.contains("manage-notifications"));
}
