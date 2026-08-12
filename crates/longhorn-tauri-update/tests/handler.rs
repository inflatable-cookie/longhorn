//! Direct, serialized, and Tauri mock-runtime handler conformance.

use std::sync::{Arc, Mutex};

use longhorn_tauri_update::{
    TauriUpdateState, UpdateHandlerAssembly, UpdateHostAuthority, UpdateHostError,
    UpdateHostService, longhorn_update_check, longhorn_update_defer, longhorn_update_install,
    longhorn_update_select_channel, longhorn_update_snapshot, update_changed_event,
};
use longhorn_update::{
    Channel, DeferralCause, UpdateAvailabilityProjection, UpdateChangedKind, UpdateCheckCommand,
    UpdateDeferCommand, UpdateInstallCommand, UpdateOutcomeProjection, UpdateProgressProjection,
    UpdateProtocolVersion, UpdateRejectionCode, UpdateSelectChannelCommand, UpdateSnapshot,
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

impl UpdateHostAuthority for Authority {
    fn snapshot(&mut self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError> {
        self.record("snapshot", caller);
        Ok(snapshot())
    }

    fn check(
        &mut self,
        caller: &str,
        _: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("check", caller);
        Ok(committed())
    }

    fn select_channel(
        &mut self,
        caller: &str,
        _: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("select_channel", caller);
        Ok(committed())
    }

    fn defer(
        &mut self,
        caller: &str,
        _: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("defer", caller);
        Ok(committed())
    }

    fn install(
        &mut self,
        caller: &str,
        _: UpdateInstallCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.record("install", caller);
        Ok(rejected())
    }
}

fn snapshot() -> UpdateSnapshot {
    UpdateSnapshot {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 4,
        channel: Channel::Production,
        installed_version: "1.3.0".to_owned(),
        availability: UpdateAvailabilityProjection::UpToDate,
        deferral: None,
        progress: UpdateProgressProjection::Idle,
    }
}

fn committed() -> UpdateOutcomeProjection {
    UpdateOutcomeProjection::Committed {
        snapshot: snapshot(),
    }
}

fn rejected() -> UpdateOutcomeProjection {
    UpdateOutcomeProjection::Rejected {
        code: UpdateRejectionCode::NotWritable,
        snapshot: snapshot(),
    }
}

fn check_command() -> UpdateCheckCommand {
    UpdateCheckCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 4,
    }
}

#[test]
fn mock_runtime_uses_one_injected_caller_aware_assembly() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = Arc::new(UpdateHandlerAssembly::new(Authority {
        calls: calls.clone(),
    }));
    let app = tauri::test::mock_builder()
        .manage(TauriUpdateState::new(service))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::default())
        .build()
        .unwrap();

    assert_eq!(
        longhorn_update_snapshot(window.clone(), app.state()).unwrap(),
        snapshot()
    );
    assert_eq!(
        longhorn_update_check(window.clone(), app.state(), check_command()).unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_update_select_channel(
            window.clone(),
            app.state(),
            UpdateSelectChannelCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                channel: Channel::Beta,
            }
        )
        .unwrap(),
        committed()
    );
    assert_eq!(
        longhorn_update_defer(
            window.clone(),
            app.state(),
            UpdateDeferCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                version: "1.4.0".to_owned(),
                cause: DeferralCause::UserPostponed,
            }
        )
        .unwrap(),
        committed()
    );
    // A refused install still answers. The caller gets the reason and the
    // state as it remains, not an adapter error.
    assert_eq!(
        longhorn_update_install(
            window,
            app.state(),
            UpdateInstallCommand {
                protocol_version: UpdateProtocolVersion::CURRENT,
                authority_epoch: 4,
                version: "1.4.0".to_owned(),
            }
        )
        .unwrap(),
        rejected()
    );

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:settings",
            "check:settings",
            "select_channel:settings",
            "defer:settings",
            "install:settings",
        ]
    );
}

#[test]
fn serialized_service_and_committed_event_remain_payload_free() {
    let service = UpdateHandlerAssembly::new(Authority {
        calls: Arc::new(Mutex::new(Vec::new())),
    });

    let outcome: UpdateOutcomeProjection = serde_json::from_slice(
        &serde_json::to_vec(&service.check("settings", check_command()).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(outcome, committed());

    let event = update_changed_event(&committed(), UpdateChangedKind::Checked).unwrap();
    assert_eq!(event.authority_epoch, 4);
    assert_eq!(event.kind, UpdateChangedKind::Checked);
}

/// A rejection leaves the state as it was, so there is nothing to invalidate
/// and a consumer that refetched on one would be refetching for nothing.
#[test]
fn a_rejected_outcome_publishes_no_invalidation_hint() {
    assert!(update_changed_event(&rejected(), UpdateChangedKind::Progressed).is_none());
}

/// The adapter never widens a caller. Every command reads the window label
/// and passes it through, so a window's own identity is what authorizes it.
#[test]
fn every_command_passes_the_caller_through_unchanged() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = UpdateHandlerAssembly::new(Authority {
        calls: calls.clone(),
    });

    drop(service.snapshot("main"));
    drop(service.install(
        "main",
        UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 4,
            version: "1.4.0".to_owned(),
        },
    ));

    assert_eq!(*calls.lock().unwrap(), ["snapshot:main", "install:main"]);
}
