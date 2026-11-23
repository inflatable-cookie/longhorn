//! Registered command dispatch and capability separation for bridge.

use std::{collections::BTreeSet, fs, path::Path, sync::Arc};

use longhorn_bridge::{BridgeHelloRequest, BridgeNegotiationReceipt, DomainCapabilityDescriptor};
use longhorn_tauri_bridge::{
    BridgeAuthorityProvider, BridgeDomainRegistry, BridgeHandlerAssembly, BridgeHostError,
    bridge_plugin,
};
use serde_json::{Value, json};
use tauri::{WebviewUrl, WebviewWindowBuilder};

struct StubProvider;

impl BridgeAuthorityProvider for StubProvider {
    fn negotiate(
        &mut self,
        _caller: &str,
        _request: &BridgeHelloRequest,
        _registered_domains: &[DomainCapabilityDescriptor],
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Err(BridgeHostError::authority("stub", false))
    }

    fn refresh(
        &mut self,
        _caller: &str,
        _current: &BridgeNegotiationReceipt,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Err(BridgeHostError::authority("stub", false))
    }
}

#[test]
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let assembly = Arc::new(BridgeHandlerAssembly::new(
        StubProvider,
        BridgeDomainRegistry::new(),
    ));
    let app = tauri::test::mock_builder()
        .plugin(bridge_plugin(assembly))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_bridge::longhorn_bridge_hello,
            longhorn_tauri_bridge::longhorn_bridge_authority,
            longhorn_tauri_bridge::longhorn_bridge_query,
            longhorn_tauri_bridge::longhorn_bridge_command,
            longhorn_tauri_bridge::longhorn_bridge_cancel,
            longhorn_tauri_bridge::longhorn_bridge_resync,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();

    let expected_groups: &[(&str, &str, &[&str])] = &[
        (
            "examples/permissions/query-bridge.toml",
            "allow-longhorn-bridge-query",
            &[
                "longhorn_bridge_hello",
                "longhorn_bridge_authority",
                "longhorn_bridge_query",
                "longhorn_bridge_resync",
            ],
        ),
        (
            "examples/permissions/mutate-bridge.toml",
            "allow-longhorn-bridge-mutate",
            &["longhorn_bridge_command", "longhorn_bridge_cancel"],
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
                    body: tauri::ipc::InvokeBody::Json(json!({})),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            );
            let error = result.expect_err("stub provider or empty payload must reject");
            let text = error.to_string();
            assert!(
                !text.contains("not found"),
                "command {command} from {path} is not registered: {text}"
            );
        }
    }
    assert_eq!(seen.len(), 6);
}

#[test]
fn query_only_capability_has_no_event_or_mutation_admission() {
    let query_only = json_file("examples/capabilities/query-only.json");
    let subscription = json_file("examples/capabilities/subscription.json");

    assert_eq!(
        query_only["permissions"],
        json!(["allow-longhorn-bridge-query"])
    );
    assert_eq!(
        subscription["permissions"],
        json!([
            "allow-longhorn-bridge-query",
            "allow-longhorn-bridge-mutate",
            "core:event:allow-listen",
            "core:event:allow-unlisten"
        ])
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
    fs::read_to_string(root().join(relative)).unwrap()
}

fn json_file(relative: &str) -> Value {
    serde_json::from_str(&read(relative)).unwrap()
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
