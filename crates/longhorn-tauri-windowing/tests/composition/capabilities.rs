use std::{fs, path::PathBuf};

use tauri::utils::acl::capability::Capability;

fn capability(name: &str) -> Capability {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/capabilities")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn capability_examples_parse_and_grant_only_titlebar_drag() {
    for (name, expected_windows) in [
        ("protected-main.json", vec!["main"]),
        (
            "protected-main-and-workspaces.json",
            vec!["main", "workspace-*"],
        ),
    ] {
        let capability = capability(name);
        assert_eq!(capability.windows, expected_windows);
        assert_eq!(capability.permissions.len(), 1);
        assert_eq!(
            serde_json::to_value(&capability.permissions).unwrap(),
            serde_json::json!(["core:window:allow-start-dragging"])
        );
        assert!(capability.remote.is_none());
        assert!(capability.local);
    }
}
