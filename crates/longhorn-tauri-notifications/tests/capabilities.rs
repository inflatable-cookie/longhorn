//! Registered command dispatch and capability separation for notifications.

use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use longhorn_notifications::{
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};
use longhorn_tauri_notifications::{
    NotificationHandlerAssembly, NotificationHostAuthority, NotificationHostError,
    TauriNotificationState,
};
use serde_json::Value;
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct StubAuthority;

impl NotificationHostAuthority for StubAuthority {
    fn snapshot(
        &mut self,
        _caller: &str,
        _query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError> {
        Err(NotificationHostError::authority("stub", false))
    }

    fn mutate(
        &mut self,
        _caller: &str,
        _command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError> {
        Err(NotificationHostError::authority("stub", false))
    }
}

#[test]
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let assembly = Arc::new(NotificationHandlerAssembly::new(StubAuthority));
    let app = tauri::test::mock_builder()
        .manage(TauriNotificationState::new(assembly))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_notifications::longhorn_notifications_snapshot,
            longhorn_tauri_notifications::longhorn_notifications_mutate,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();

    let expected_groups: &[(&str, &str, &[&str])] = &[
        (
            "examples/permissions/read-notifications.toml",
            "read-notifications",
            &["longhorn_notifications_snapshot"],
        ),
        (
            "examples/permissions/manage-notifications.toml",
            "manage-notifications",
            &[
                "longhorn_notifications_snapshot",
                "longhorn_notifications_mutate",
            ],
        ),
    ];

    let mut seen = BTreeSet::new();
    let mut callback_id = 0_u32;
    for (path, expected_identifier, expected_commands) in expected_groups {
        let toml_text = read(path);
        assert_eq!(
            parse_permission_identifier(&toml_text).as_deref(),
            Some(*expected_identifier),
            "unexpected identifier in {path}"
        );
        let allowed = parse_allowed_commands(&toml_text);
        let expected: Vec<String> = expected_commands.iter().map(|c| (*c).to_owned()).collect();
        assert_eq!(allowed, expected, "unexpected commands in {path}");
        for command in allowed {
            seen.insert(command.clone());
            callback_id += 2;
            let result = tauri::test::get_ipc_response(
                &webview,
                tauri::webview::InvokeRequest {
                    cmd: command.clone(),
                    callback: tauri::ipc::CallbackFn(callback_id),
                    error: tauri::ipc::CallbackFn(callback_id + 1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(serde_json::json!({})),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            );
            let error = result.expect_err("stub authority or empty payload must reject");
            let text = error.to_string();
            assert!(
                !text.contains("not found"),
                "command {command} from {path} is not registered: {text}"
            );
        }
    }
    assert_eq!(seen.len(), 2);

    let read_cap: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-notifications.json")).unwrap();
    let manage_cap: Value =
        serde_json::from_str(&read("examples/capabilities/managed-notifications.json")).unwrap();
    assert_eq!(
        read_cap["permissions"],
        serde_json::json!(["read-notifications"])
    );
    assert_eq!(
        manage_cap["permissions"],
        serde_json::json!(["manage-notifications"])
    );
}

fn parse_allowed_commands(toml_text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_allow = false;
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("commands.allow") {
            in_allow = true;
        }
        if in_allow {
            let mut rest = trimmed;
            while let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                let Some(end) = after.find('"') else {
                    break;
                };
                commands.push(after[..end].to_owned());
                rest = &after[end + 1..];
            }
            if trimmed.contains(']') {
                in_allow = false;
            }
        }
    }
    commands
}

fn parse_permission_identifier(toml_text: &str) -> Option<String> {
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("identifier") {
            let start = rest.find('"')?;
            let after = &rest[start + 1..];
            let end = after.find('"')?;
            return Some(after[..end].to_owned());
        }
    }
    None
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}
