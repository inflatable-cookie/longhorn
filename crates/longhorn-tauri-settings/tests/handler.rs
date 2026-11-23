//! Direct and serialized injected-handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_core::SettingsApplyUnitId;
use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationResult,
    SettingsRegistrySnapshot, SettingsResetCommand,
};
use longhorn_tauri_settings::{
    ConsumerSettingsTransactionAuthority, ConsumerSettingsTransactionOutcome,
    SettingsHandlerAssembly, SettingsHostAuthority, SettingsHostError, SettingsHostErrorCode,
    mutation_changed_event,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    registry: SettingsRegistrySnapshot,
    load_commands: Vec<SettingsLoadCommand>,
    apply_commands: Vec<SettingsApplyCommand>,
    reset_commands: Vec<SettingsResetCommand>,
    load_outcomes: Vec<SettingsLoadOutcome>,
    mutation_results: Vec<SettingsMutationResult>,
}

#[derive(Clone)]
struct FixtureAuthority {
    fixture: Fixture,
    calls: Arc<Mutex<Vec<String>>>,
}

impl SettingsHostAuthority for FixtureAuthority {
    fn registry(&mut self, caller: &str) -> Result<SettingsRegistrySnapshot, SettingsHostError> {
        self.record("registry", caller);
        self.authorize(caller)?;
        Ok(self.fixture.registry.clone())
    }

    fn load(
        &mut self,
        caller: &str,
        _command: SettingsLoadCommand,
    ) -> Result<SettingsLoadOutcome, SettingsHostError> {
        self.record("load", caller);
        self.authorize(caller)?;
        Ok(self.fixture.load_outcomes[0].clone())
    }

    fn apply(
        &mut self,
        caller: &str,
        _command: SettingsApplyCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        self.record("apply", caller);
        self.authorize(caller)?;
        Ok(self.fixture.mutation_results[0].clone())
    }

    fn reset(
        &mut self,
        caller: &str,
        _command: SettingsResetCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        self.record("reset", caller);
        self.authorize(caller)?;
        Ok(self.fixture.mutation_results[1].clone())
    }

    fn authorize_transaction(
        &mut self,
        caller: &str,
        _apply_unit_id: &SettingsApplyUnitId,
    ) -> Result<(), SettingsHostError> {
        self.record("authorize-tx", caller);
        self.authorize(caller)
    }
}

impl FixtureAuthority {
    fn record(&self, operation: &str, caller: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{operation}:{caller}"));
    }

    fn authorize(&self, caller: &str) -> Result<(), SettingsHostError> {
        if caller == "settings" {
            Ok(())
        } else {
            Err(SettingsHostError::authority(
                "caller is not authorized",
                false,
            ))
        }
    }
}

#[test]
fn direct_and_serialized_commands_use_one_injected_assembly() {
    let fixture = fixture();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = SettingsHandlerAssembly::new(FixtureAuthority {
        fixture: fixture.clone(),
        calls: calls.clone(),
    });

    assert_eq!(service.registry("settings").unwrap(), fixture.registry);
    let load = serialized(&fixture.load_commands[0]);
    assert_eq!(
        service.load("settings", load).unwrap(),
        fixture.load_outcomes[0]
    );
    let apply = serialized(&fixture.apply_commands[0]);
    let applied = service.apply("settings", apply).unwrap();
    assert_eq!(serialized(&applied), fixture.mutation_results[0]);
    let reset = serialized(&fixture.reset_commands[0]);
    let reset_result = service.reset("settings", reset).unwrap();
    assert_eq!(serialized(&reset_result), fixture.mutation_results[1]);
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "registry:settings",
            "load:settings",
            "apply:settings",
            "reset:settings"
        ]
    );
}

#[test]
fn caller_authorization_is_not_delegated_to_tauri_capabilities() {
    let fixture = fixture();
    let service = SettingsHandlerAssembly::new(FixtureAuthority {
        fixture,
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let error = service.registry("main").unwrap_err();
    assert_eq!(error.code, SettingsHostErrorCode::AuthorityUnavailable);
    assert!(!error.retryable);
}

#[test]
fn changed_receipts_project_hints_but_unchanged_and_failed_results_do_not() {
    let fixture = fixture();
    let changed = mutation_changed_event(&fixture.mutation_results[0]).unwrap();
    assert_eq!(changed.registry_generation.get(), 7);
    assert_eq!(changed.scope_revision.get(), 3);
    assert_eq!(changed.scope_id.as_str(), "app:preferences");
    assert!(mutation_changed_event(&fixture.mutation_results[1]).is_none());
    assert!(mutation_changed_event(&fixture.mutation_results[2]).is_none());
}

#[test]
fn poisoned_handler_state_is_a_typed_non_retryable_failure() {
    let fixture = fixture();
    let service = Arc::new(SettingsHandlerAssembly::new(FixtureAuthority {
        fixture,
        calls: Arc::new(Mutex::new(Vec::new())),
    }));
    let poison = service.clone();
    assert!(
        std::thread::spawn(move || poison.with_authority::<()>(|_| panic!("poison")))
            .join()
            .is_err()
    );
    let error = service.registry("settings").unwrap_err();
    assert_eq!(error.code, SettingsHostErrorCode::StateUnavailable);
    assert!(!error.retryable);
}

struct RecordingTransaction {
    fixture: Fixture,
    calls: Arc<Mutex<Vec<String>>>,
}

impl ConsumerSettingsTransactionAuthority for RecordingTransaction {
    type Error = SettingsHostError;
    type Receipt = String;

    fn apply_transaction(
        &self,
        command: &SettingsApplyCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("tx-apply:{}", command.apply_unit_id.as_str()));
        Ok(ConsumerSettingsTransactionOutcome {
            result: self.fixture.mutation_results[0].clone(),
            authority_receipt: "tx:apply:1".into(),
        })
    }

    fn reset_transaction(
        &self,
        command: &SettingsResetCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("tx-reset:{}", command.apply_unit_id.as_str()));
        Ok(ConsumerSettingsTransactionOutcome {
            result: self.fixture.mutation_results[1].clone(),
            authority_receipt: "tx:reset:1".into(),
        })
    }
}

#[test]
fn injected_transaction_authority_handles_declared_units_after_caller_authorization() {
    let fixture = fixture();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tx_unit = fixture.apply_commands[0].apply_unit_id.clone();
    let assembly = SettingsHandlerAssembly::with_transaction_authority(
        FixtureAuthority {
            fixture: fixture.clone(),
            calls: Arc::clone(&calls),
        },
        [tx_unit.clone()],
        RecordingTransaction {
            fixture: fixture.clone(),
            calls: Arc::clone(&calls),
        },
    );

    // Unauthorized caller is rejected before the transaction authority runs.
    let unauthorized = assembly
        .apply("main", fixture.apply_commands[0].clone())
        .unwrap_err();
    assert_eq!(
        unauthorized.code,
        SettingsHostErrorCode::AuthorityUnavailable
    );
    assert!(!unauthorized.retryable);

    // Authorized caller routes through the transaction seam.
    let applied = assembly
        .apply("settings", fixture.apply_commands[0].clone())
        .unwrap();
    assert_eq!(applied, fixture.mutation_results[0]);
    let reset = assembly
        .reset("settings", fixture.reset_commands[0].clone())
        .unwrap();
    assert_eq!(reset, fixture.mutation_results[1]);

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "authorize-tx:main",
            "authorize-tx:settings",
            &format!("tx-apply:{}", tx_unit.as_str()),
            "authorize-tx:settings",
            &format!("tx-reset:{}", tx_unit.as_str()),
        ]
    );
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../../fixtures/settings/protocol-v1.json")).unwrap()
}

fn serialized<Value>(value: &Value) -> Value
where
    Value: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}
