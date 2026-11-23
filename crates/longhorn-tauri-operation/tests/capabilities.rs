//! Registered command dispatch and capability separation for operations.

use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use longhorn_core::OperationId;
use longhorn_operation::{
    OperationCancellationCommand, OperationCancellationResult, OperationMutationCommand,
    OperationMutationResult, OperationSnapshotQuery, OperationSnapshotResponse,
};
use longhorn_tauri_operation::{
    OperationExecutorError, OperationExecutorPort, OperationHandlerAssembly,
    OperationHostAuthority, OperationHostError, TauriOperationState,
};
use serde_json::Value;
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct StubAuthority;

impl OperationHostAuthority for StubAuthority {
    fn snapshot(
        &mut self,
        _caller: &str,
        _query: OperationSnapshotQuery,
    ) -> Result<OperationSnapshotResponse, OperationHostError> {
        Err(OperationHostError::authority("stub", false))
    }

    fn mutate(
        &mut self,
        _caller: &str,
        _command: OperationMutationCommand,
    ) -> Result<OperationMutationResult, OperationHostError> {
        Err(OperationHostError::authority("stub", false))
    }

    fn cancel(
        &mut self,
        _caller: &str,
        _command: OperationCancellationCommand,
    ) -> Result<OperationCancellationResult, OperationHostError> {
        Err(OperationHostError::authority("stub", false))
    }
}

struct StubExecutor;

impl OperationExecutorPort for StubExecutor {
    fn request_cancellation(
        &mut self,
        _operation_id: &OperationId,
    ) -> Result<(), OperationExecutorError> {
        Ok(())
    }
}

#[test]
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let assembly = Arc::new(OperationHandlerAssembly::new(StubAuthority, StubExecutor));
    let app = tauri::test::mock_builder()
        .manage(TauriOperationState::new(assembly))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_operation::longhorn_operation_snapshot,
            longhorn_tauri_operation::longhorn_operation_mutate,
            longhorn_tauri_operation::longhorn_operation_cancel,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();

    let expected_groups: &[(&str, &str, &[&str])] = &[
        (
            "examples/permissions/read-operation.toml",
            "read-operation",
            &["longhorn_operation_snapshot"],
        ),
        (
            "examples/permissions/cancel-operation.toml",
            "cancel-operation",
            &["longhorn_operation_cancel"],
        ),
        (
            "examples/permissions/manage-operation.toml",
            "manage-operation",
            &["longhorn_operation_mutate"],
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
            assert!(
                seen.insert(command.clone()),
                "duplicate command {command} across permissions"
            );
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
    assert_eq!(seen.len(), 3);

    let read_cap: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-operation.json")).unwrap();
    let cancel_cap: Value =
        serde_json::from_str(&read("examples/capabilities/cancellable-operation.json")).unwrap();
    let manage_cap: Value =
        serde_json::from_str(&read("examples/capabilities/managed-operation.json")).unwrap();
    assert_eq!(
        read_cap["permissions"],
        serde_json::json!(["read-operation"])
    );
    assert_eq!(
        cancel_cap["permissions"],
        serde_json::json!(["read-operation", "cancel-operation"])
    );
    assert_eq!(
        manage_cap["permissions"],
        serde_json::json!(["read-operation", "cancel-operation", "manage-operation"])
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
