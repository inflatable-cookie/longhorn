//! Narrow operation capability example audit.

use std::{fs, path::PathBuf};

#[test]
fn capability_examples_keep_read_cancel_and_manage_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let read = fs::read_to_string(root.join("permissions/read-operation.toml")).unwrap();
    let cancel = fs::read_to_string(root.join("permissions/cancel-operation.toml")).unwrap();
    let manage = fs::read_to_string(root.join("permissions/manage-operation.toml")).unwrap();
    assert!(read.contains("longhorn_operation_snapshot"));
    assert!(!read.contains("longhorn_operation_cancel"));
    assert!(cancel.contains("longhorn_operation_cancel"));
    assert!(!cancel.contains("longhorn_operation_mutate"));
    assert!(manage.contains("longhorn_operation_mutate"));

    for capability in [
        "read-only-operation",
        "cancellable-operation",
        "managed-operation",
    ] {
        let value =
            fs::read_to_string(root.join(format!("capabilities/{capability}.json"))).unwrap();
        serde_json::from_str::<serde_json::Value>(&value).unwrap();
    }
}
