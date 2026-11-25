//! Registered command dispatch and capability separation for fork history.

use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkContinuationPageCommand,
    ForkContinuationPageSnapshot, ForkDeleteContinuationCommand, ForkDeleteResult,
    ForkNavigationCommand, ForkNavigationResult, ForkPathPageCommand, ForkPathPageSnapshot,
    ForkPruneCommand, ForkPruneResult, ForkSnapshot,
};
use longhorn_tauri_history_tree::{
    ForkHistoryHandlerAssembly, ForkHistoryHostAuthority, ForkHistoryHostError,
    TauriForkHistoryState,
};
use serde_json::Value;
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct StubAuthority;

impl ForkHistoryHostAuthority for StubAuthority {
    fn snapshot(&mut self, _caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn path(
        &mut self,
        _caller: &str,
        _command: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn branches(
        &mut self,
        _caller: &str,
        _command: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn continuations(
        &mut self,
        _caller: &str,
        _command: ForkContinuationPageCommand,
    ) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn delete_continuation(
        &mut self,
        _caller: &str,
        _command: ForkDeleteContinuationCommand,
    ) -> Result<ForkDeleteResult, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn prune(
        &mut self,
        _caller: &str,
        _command: ForkPruneCommand,
    ) -> Result<ForkPruneResult, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }

    fn navigate(
        &mut self,
        _caller: &str,
        _command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError> {
        Err(ForkHistoryHostError::authority("stub", false))
    }
}

#[test]
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let assembly = Arc::new(ForkHistoryHandlerAssembly::new(StubAuthority));
    let app = tauri::test::mock_builder()
        .manage(TauriForkHistoryState::new(assembly))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_history_tree::longhorn_history_tree_snapshot,
            longhorn_tauri_history_tree::longhorn_history_tree_path,
            longhorn_tauri_history_tree::longhorn_history_tree_branches,
            longhorn_tauri_history_tree::longhorn_history_tree_continuations,
            longhorn_tauri_history_tree::longhorn_history_tree_delete_continuation,
            longhorn_tauri_history_tree::longhorn_history_tree_prune,
            longhorn_tauri_history_tree::longhorn_history_tree_navigate,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "history", WebviewUrl::default())
        .build()
        .unwrap();

    let expected_groups: &[(&str, &str, &[&str])] = &[
        (
            "examples/permissions/read-history-tree.toml",
            "allow-longhorn-history-tree-read",
            &[
                "longhorn_history_tree_snapshot",
                "longhorn_history_tree_path",
                "longhorn_history_tree_branches",
                "longhorn_history_tree_continuations",
            ],
        ),
        (
            "examples/permissions/mutate-history-tree.toml",
            "allow-longhorn-history-tree-mutate",
            &["longhorn_history_tree_navigate"],
        ),
        (
            "examples/permissions/delete-history-tree.toml",
            "allow-longhorn-history-tree-delete",
            &[
                "longhorn_history_tree_delete_continuation",
                "longhorn_history_tree_prune",
            ],
        ),
    ];

    let mut seen = BTreeSet::new();
    let mut callback_id = 0_u32;
    for (path, expected_identifier, expected_commands) in expected_groups {
        let toml_text = file(path);
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
    assert_eq!(seen.len(), 7);

    let destructive: Value =
        serde_json::from_str(&file("examples/capabilities/destructive-history-tree.json")).unwrap();
    assert_eq!(
        destructive["permissions"],
        serde_json::json!(["allow-longhorn-history-tree-delete"])
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

fn file(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
