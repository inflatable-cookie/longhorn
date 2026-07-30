//! Direct and serialized injected-handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_command_config::{
    CommandCatalogueSnapshot, CommandKeymapCommit, CommandKeymapLoadOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapReset,
};
use longhorn_tauri_command::{
    CommandHandlerAssembly, CommandHostAuthority, CommandHostError, CommandHostErrorCode,
    CommandHostService, keymap_changed_event,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Requests {
    preview: CommandKeymapPreview,
    commit: CommandKeymapCommit,
    reset: CommandKeymapReset,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    catalogue: CommandCatalogueSnapshot,
    requests: Requests,
    load_outcomes: Vec<CommandKeymapLoadOutcome>,
    preview_results: Vec<CommandKeymapPreviewResult>,
    mutation_results: Vec<CommandKeymapMutationResult>,
}

#[derive(Clone)]
struct FixtureAuthority {
    fixture: Fixture,
    calls: Arc<Mutex<Vec<String>>>,
}

impl CommandHostAuthority for FixtureAuthority {
    fn catalogue(&mut self, caller: &str) -> Result<CommandCatalogueSnapshot, CommandHostError> {
        self.record("catalogue", caller);
        self.authorize(caller)?;
        Ok(self.fixture.catalogue.clone())
    }

    fn keymap(&mut self, caller: &str) -> Result<CommandKeymapLoadOutcome, CommandHostError> {
        self.record("keymap", caller);
        self.authorize(caller)?;
        Ok(self.fixture.load_outcomes[0].clone())
    }

    fn preview(
        &mut self,
        caller: &str,
        _request: CommandKeymapPreview,
    ) -> Result<CommandKeymapPreviewResult, CommandHostError> {
        self.record("preview", caller);
        self.authorize(caller)?;
        Ok(self.fixture.preview_results[0].clone())
    }

    fn commit(
        &mut self,
        caller: &str,
        _request: CommandKeymapCommit,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.record("commit", caller);
        self.authorize(caller)?;
        Ok(self.fixture.mutation_results[0].clone())
    }

    fn reset(
        &mut self,
        caller: &str,
        _request: CommandKeymapReset,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.record("reset", caller);
        self.authorize(caller)?;
        Ok(self.fixture.mutation_results[1].clone())
    }
}

impl FixtureAuthority {
    fn record(&self, operation: &str, caller: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{operation}:{caller}"));
    }

    fn authorize(&self, caller: &str) -> Result<(), CommandHostError> {
        (caller == "settings").then_some(()).ok_or_else(|| {
            CommandHostError::authority("caller is not authorized for command keymaps", false)
        })
    }
}

#[test]
fn direct_and_serialized_requests_use_one_injected_assembly() {
    let fixture = fixture();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = CommandHandlerAssembly::new(FixtureAuthority {
        fixture: fixture.clone(),
        calls: calls.clone(),
    });

    assert_eq!(service.catalogue("settings").unwrap(), fixture.catalogue);
    assert_eq!(
        service.keymap("settings").unwrap(),
        fixture.load_outcomes[0]
    );
    assert_eq!(
        service
            .preview("settings", serialized(&fixture.requests.preview))
            .unwrap(),
        fixture.preview_results[0]
    );
    assert_eq!(
        service
            .commit("settings", serialized(&fixture.requests.commit))
            .unwrap(),
        fixture.mutation_results[0]
    );
    assert_eq!(
        service
            .reset("settings", serialized(&fixture.requests.reset))
            .unwrap(),
        fixture.mutation_results[1]
    );
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "catalogue:settings",
            "keymap:settings",
            "preview:settings",
            "commit:settings",
            "reset:settings",
        ]
    );
}

#[test]
fn caller_authorization_is_not_delegated_to_tauri_capabilities() {
    let service = CommandHandlerAssembly::new(FixtureAuthority {
        fixture: fixture(),
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let error = service.catalogue("main").unwrap_err();
    assert_eq!(error.code, CommandHostErrorCode::AuthorityUnavailable);
    assert!(!error.retryable);
}

#[test]
fn only_changed_applied_receipts_project_keymap_hints() {
    let fixture = fixture();
    let changed = keymap_changed_event(&fixture.mutation_results[0]).unwrap();
    assert_eq!(changed.registry_generation.get(), 0);
    assert_eq!(changed.keymap_revision.get(), 1);
    assert!(keymap_changed_event(&fixture.mutation_results[1]).is_none());
    assert!(keymap_changed_event(&fixture.mutation_results[2]).is_none());
}

#[test]
fn poisoned_handler_state_is_a_typed_retryable_failure() {
    let service = Arc::new(CommandHandlerAssembly::new(FixtureAuthority {
        fixture: fixture(),
        calls: Arc::new(Mutex::new(Vec::new())),
    }));
    let poison = service.clone();
    assert!(
        std::thread::spawn(move || poison.with_authority::<()>(|_| panic!("poison")))
            .join()
            .is_err()
    );
    let error = service.catalogue("settings").unwrap_err();
    assert_eq!(error.code, CommandHostErrorCode::StateUnavailable);
    assert!(error.retryable);
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../../fixtures/commands/protocol-v1.json")).unwrap()
}

fn serialized<Value>(value: &Value) -> Value
where
    Value: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}
