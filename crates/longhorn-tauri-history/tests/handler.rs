//! Direct and serialized injected-history handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_history::{
    HistoryNavigationCommand, HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection,
    HistoryNavigationResult, HistoryPageCommand, HistoryPageSnapshot, HistorySnapshot,
};
use longhorn_tauri_history::{
    HistoryHandlerAssembly, HistoryHostAuthority, HistoryHostError, HistoryHostErrorCode,
    HistoryHostService, history_changed_event,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    snapshot: HistorySnapshot,
    page_request: HistoryPageCommand,
    page: HistoryPageSnapshot,
    navigation_command: HistoryNavigationCommand,
    navigation_results: Vec<HistoryNavigationResult>,
}

#[derive(Clone)]
struct FixtureAuthority {
    fixture: Fixture,
    calls: Arc<Mutex<Vec<String>>>,
}

impl HistoryHostAuthority for FixtureAuthority {
    fn snapshot(&mut self, caller: &str) -> Result<HistorySnapshot, HistoryHostError> {
        self.record("snapshot", caller);
        Ok(self.fixture.snapshot.clone())
    }

    fn page(
        &mut self,
        caller: &str,
        _command: HistoryPageCommand,
    ) -> Result<HistoryPageSnapshot, HistoryHostError> {
        self.record("page", caller);
        Ok(self.fixture.page.clone())
    }

    fn navigate(
        &mut self,
        caller: &str,
        _command: HistoryNavigationCommand,
    ) -> Result<HistoryNavigationResult, HistoryHostError> {
        self.record("navigate", caller);
        if caller == "history" {
            Ok(self.fixture.navigation_results[0].clone())
        } else {
            Ok(HistoryNavigationResult::Rejected {
                snapshot: self.fixture.snapshot.clone(),
                rejection: HistoryNavigationRejectionProjection {
                    code: HistoryNavigationRejectionCode::Unauthorized,
                    detail: "caller lacks product history authorization".into(),
                    refresh_required: false,
                },
            })
        }
    }
}

impl FixtureAuthority {
    fn record(&self, operation: &str, caller: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{operation}:{caller}"));
    }
}

#[test]
fn direct_and_serialized_requests_use_one_injected_assembly() {
    let fixture = fixture();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = HistoryHandlerAssembly::new(FixtureAuthority {
        fixture: fixture.clone(),
        calls: calls.clone(),
    });

    assert_eq!(service.snapshot("history").unwrap(), fixture.snapshot);
    assert_eq!(
        service
            .page("history", serialized(&fixture.page_request))
            .unwrap(),
        fixture.page
    );
    assert_eq!(
        service
            .navigate("history", serialized(&fixture.navigation_command))
            .unwrap(),
        fixture.navigation_results[0]
    );
    assert_eq!(
        *calls.lock().unwrap(),
        ["snapshot:history", "page:history", "navigate:history"]
    );
}

#[test]
fn tauri_reachability_does_not_grant_product_navigation_authority() {
    let fixture = fixture();
    let service = HistoryHandlerAssembly::new(FixtureAuthority {
        fixture: fixture.clone(),
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let result = service
        .navigate("main", fixture.navigation_command)
        .unwrap();
    let HistoryNavigationResult::Rejected {
        rejection,
        snapshot,
    } = result
    else {
        panic!("unauthorized caller must be rejected");
    };
    assert_eq!(rejection.code, HistoryNavigationRejectionCode::Unauthorized);
    assert_eq!(snapshot, fixture.snapshot);
}

#[test]
fn only_committed_navigation_projects_a_live_hint() {
    let fixture = fixture();
    let event = history_changed_event(&fixture.navigation_results[0]).unwrap();
    assert_eq!(event.previous_revision.unwrap().get(), 11);
    assert_eq!(event.committed_revision.get(), 12);
    assert!(history_changed_event(&fixture.navigation_results[1]).is_none());
}

#[test]
fn renderer_fixture_contains_no_product_payload() {
    let source = include_str!("../../../fixtures/history/protocol-v1.json");
    let value: serde_json::Value = serde_json::from_str(source).unwrap();
    assert!(!contains_key(&value, "payload"));
    assert!(!source.contains("PulseHistoryMutation"));
}

#[test]
fn poisoned_handler_state_is_a_typed_retryable_failure() {
    let service = Arc::new(HistoryHandlerAssembly::new(FixtureAuthority {
        fixture: fixture(),
        calls: Arc::new(Mutex::new(Vec::new())),
    }));
    let poison = service.clone();
    assert!(
        std::thread::spawn(move || poison.with_authority::<()>(|_| panic!("poison")))
            .join()
            .is_err()
    );
    let error = service.snapshot("history").unwrap_err();
    assert_eq!(error.code, HistoryHostErrorCode::StateUnavailable);
    assert!(error.retryable);
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../../fixtures/history/protocol-v1.json")).unwrap()
}

fn serialized<Value>(value: &Value) -> Value
where
    Value: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn contains_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(values) => {
            values.contains_key(key) || values.values().any(|value| contains_key(value, key))
        }
        serde_json::Value::Array(values) => values.iter().any(|value| contains_key(value, key)),
        _ => false,
    }
}
