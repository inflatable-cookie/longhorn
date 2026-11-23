//! Registered command dispatch and capability separation for settings.

use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationResult,
    SettingsRegistrySnapshot, SettingsResetCommand,
};
use longhorn_tauri_settings::{
    SettingsHandlerAssembly, SettingsHostAuthority, SettingsHostError, TauriSettingsState,
};
use serde_json::Value;
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct StubAuthority;

impl SettingsHostAuthority for StubAuthority {
    fn registry(&mut self, _caller: &str) -> Result<SettingsRegistrySnapshot, SettingsHostError> {
        Err(SettingsHostError::authority("stub", false))
    }

    fn load(
        &mut self,
        _caller: &str,
        _command: SettingsLoadCommand,
    ) -> Result<SettingsLoadOutcome, SettingsHostError> {
        Err(SettingsHostError::authority("stub", false))
    }

    fn apply(
        &mut self,
        _caller: &str,
        _command: SettingsApplyCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        Err(SettingsHostError::authority("stub", false))
    }

    fn reset(
        &mut self,
        _caller: &str,
        _command: SettingsResetCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        Err(SettingsHostError::authority("stub", false))
    }
}

#[test]
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let assembly = Arc::new(SettingsHandlerAssembly::new(StubAuthority));
    let app = tauri::test::mock_builder()
        .manage(TauriSettingsState::new(assembly))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_settings::longhorn_settings_registry,
            longhorn_tauri_settings::longhorn_settings_load,
            longhorn_tauri_settings::longhorn_settings_apply,
            longhorn_tauri_settings::longhorn_settings_reset,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::default())
        .build()
        .unwrap();

    let expected_groups: &[(&str, &str, &[&str])] = &[
        (
            "examples/permissions/read-settings.toml",
            "allow-longhorn-settings-read",
            &["longhorn_settings_registry", "longhorn_settings_load"],
        ),
        (
            "examples/permissions/mutate-settings.toml",
            "allow-longhorn-settings-mutate",
            &["longhorn_settings_apply", "longhorn_settings_reset"],
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
    assert_eq!(seen.len(), 4);
}

#[test]
fn capability_examples_add_only_selected_commands_and_event_lifetime() {
    let read_only: Value =
        serde_json::from_str(&read("examples/capabilities/read-only-settings.json")).unwrap();
    let mutable: Value =
        serde_json::from_str(&read("examples/capabilities/mutable-settings.json")).unwrap();
    assert_eq!(
        read_only["permissions"],
        serde_json::json!([
            "allow-longhorn-settings-read",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
    );
    assert_eq!(
        mutable["permissions"],
        serde_json::json!([
            "allow-longhorn-settings-read",
            "allow-longhorn-settings-mutate",
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
