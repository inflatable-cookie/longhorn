//! Direct, serialized, and Tauri mock-runtime handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_licence::{
    LicenceActivateCommand, LicenceChangedKind, LicenceCredentialProjection,
    LicenceDeactivateCommand, LicenceOutcomeProjection, LicenceProtocolVersion,
    LicenceRefreshCommand, LicenceRejectionCode, LicenceReleaseSeatCommand,
    LicenceRenameSeatCommand, LicenceSnapshot,
};
use longhorn_tauri_licence::{
    LicenceHandlerAssembly, LicenceHostAuthority, LicenceHostError, LicenceHostService,
    TauriLicenceState, licence_changed_event, longhorn_licence_activate,
    longhorn_licence_deactivate, longhorn_licence_refresh, longhorn_licence_release_seat,
    longhorn_licence_rename_seat, longhorn_licence_snapshot,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct Authority {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Authority {
    fn record(&self, what: &str, caller: &str) {
        self.calls.lock().unwrap().push(format!("{what}:{caller}"));
    }
}

impl LicenceHostAuthority for Authority {
    fn snapshot(&mut self, caller: &str) -> Result<LicenceSnapshot, LicenceHostError> {
        self.record("snapshot", caller);
        Ok(snapshot())
    }

    fn activate(
        &mut self,
        caller: &str,
        _: LicenceActivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.record("activate", caller);
        Ok(committed())
    }

    fn deactivate(
        &mut self,
        caller: &str,
        _: LicenceDeactivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.record("deactivate", caller);
        Ok(committed())
    }

    fn refresh(
        &mut self,
        caller: &str,
        _: LicenceRefreshCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.record("refresh", caller);
        Ok(committed())
    }

    fn release_seat(
        &mut self,
        caller: &str,
        _: LicenceReleaseSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.record("release_seat", caller);
        Ok(rejected())
    }

    fn rename_seat(
        &mut self,
        caller: &str,
        _: LicenceRenameSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.record("rename_seat", caller);
        Ok(committed())
    }
}

fn snapshot() -> LicenceSnapshot {
    LicenceSnapshot::unlicensed(4)
}

fn committed() -> LicenceOutcomeProjection {
    LicenceOutcomeProjection::Committed {
        snapshot: snapshot(),
    }
}

fn rejected() -> LicenceOutcomeProjection {
    LicenceOutcomeProjection::Rejected {
        code: LicenceRejectionCode::SeatNotFound,
        snapshot: snapshot(),
    }
}

fn envelope() -> (LicenceProtocolVersion, u64) {
    (LicenceProtocolVersion::CURRENT, 4)
}

#[test]
fn mock_runtime_uses_one_injected_caller_aware_assembly() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = Arc::new(LicenceHandlerAssembly::new(Authority {
        calls: calls.clone(),
    }));
    let app = tauri::test::mock_builder()
        .manage(TauriLicenceState::new(service))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::default())
        .build()
        .unwrap();
    let (protocol_version, authority_epoch) = envelope();

    assert_eq!(
        longhorn_licence_snapshot(window.clone(), app.state()).unwrap(),
        snapshot()
    );
    assert_eq!(
        longhorn_licence_activate(
            window.clone(),
            app.state(),
            LicenceActivateCommand {
                protocol_version,
                authority_epoch,
                credential: LicenceCredentialProjection::Key {
                    key: "ABCDE12345FGHJK67894".to_owned(),
                },
                label: Some("Studio".to_owned()),
            }
        )
        .unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_licence_deactivate(
            window.clone(),
            app.state(),
            LicenceDeactivateCommand {
                protocol_version,
                authority_epoch,
            }
        )
        .unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_licence_refresh(
            window.clone(),
            app.state(),
            LicenceRefreshCommand {
                protocol_version,
                authority_epoch,
            }
        )
        .unwrap(),
        committed()
    );
    // A refused release still answers: the caller gets the reason and the
    // state as it remains, not an adapter error.
    assert_eq!(
        longhorn_licence_release_seat(
            window.clone(),
            app.state(),
            LicenceReleaseSeatCommand {
                protocol_version,
                authority_epoch,
                machine_id: "m-the-old-macbook-16".to_owned(),
            }
        )
        .unwrap(),
        rejected()
    );
    assert_eq!(
        longhorn_licence_rename_seat(
            window,
            app.state(),
            LicenceRenameSeatCommand {
                protocol_version,
                authority_epoch,
                machine_id: "m-the-old-macbook-16".to_owned(),
                label: None,
            }
        )
        .unwrap(),
        committed()
    );

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:settings",
            "activate:settings",
            "deactivate:settings",
            "refresh:settings",
            "release_seat:settings",
            "rename_seat:settings",
        ]
    );
}

#[test]
fn serialized_service_and_committed_event_remain_credential_free() {
    let service = LicenceHandlerAssembly::new(Authority {
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let (protocol_version, authority_epoch) = envelope();

    let outcome = service
        .refresh(
            "settings",
            LicenceRefreshCommand {
                protocol_version,
                authority_epoch,
            },
        )
        .unwrap();
    let encoded = serde_json::to_string(&outcome).unwrap();
    for forbidden in ["credential", "secret", "signature", "token", "keyId"] {
        assert!(
            !encoded.contains(forbidden),
            "`{forbidden}` reached a serialized outcome"
        );
    }

    let event = licence_changed_event(&outcome, LicenceChangedKind::Refreshed).unwrap();
    assert_eq!(event.authority_epoch, 4);
    assert_eq!(event.kind, LicenceChangedKind::Refreshed);
}

/// A rejection leaves the state as it was, so there is nothing to invalidate.
#[test]
fn a_rejected_outcome_publishes_no_invalidation_hint() {
    assert!(licence_changed_event(&rejected(), LicenceChangedKind::Deactivated).is_none());
}
