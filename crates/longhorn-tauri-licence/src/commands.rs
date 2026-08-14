use std::sync::Arc;

use longhorn_licence::{
    LicenceActivateCommand, LicenceChangedEvent, LicenceChangedKind, LicenceDeactivateCommand,
    LicenceOutcomeProjection, LicenceProtocolVersion, LicenceRefreshCommand,
    LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::LicenceHostError;

/// Non-durable committed licence invalidation hint.
pub const LICENCE_CHANGED_EVENT: &str = "longhorn://licence/changed";

/// Object-safe licence surface retained in Tauri managed state.
pub trait LicenceHostService: Send + Sync {
    /// Returns the caller-authorized licence state.
    fn snapshot(&self, caller: &str) -> Result<LicenceSnapshot, LicenceHostError>;
    /// Presents a credential and asks for a licence.
    fn activate(
        &self,
        caller: &str,
        command: LicenceActivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
    /// Releases this machine's seat.
    fn deactivate(
        &self,
        caller: &str,
        command: LicenceDeactivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
    /// Re-checks the lease now.
    fn refresh(
        &self,
        caller: &str,
        command: LicenceRefreshCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
    /// Releases a named machine's seat.
    fn release_seat(
        &self,
        caller: &str,
        command: LicenceReleaseSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
    /// Renames a machine's seat.
    fn rename_seat(
        &self,
        caller: &str,
        command: LicenceRenameSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
}

/// Type-erased licence host installed once in Tauri managed state.
pub struct TauriLicenceState {
    service: Arc<dyn LicenceHostService>,
}

impl TauriLicenceState {
    /// Wraps one explicitly injected licence assembly.
    #[must_use]
    pub fn new(service: Arc<dyn LicenceHostService>) -> Self {
        Self { service }
    }
}

/// Returns caller-authorized licence state.
#[tauri::command]
pub fn longhorn_licence_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
) -> Result<LicenceSnapshot, LicenceHostError> {
    state.service.snapshot(window.label())
}

/// Presents a credential and asks for a licence.
#[tauri::command]
pub fn longhorn_licence_activate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceActivateCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    emitting(
        &window,
        state.service.activate(window.label(), command)?,
        LicenceChangedKind::Activated,
    )
}

/// Releases this machine's seat.
#[tauri::command]
pub fn longhorn_licence_deactivate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceDeactivateCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    emitting(
        &window,
        state.service.deactivate(window.label(), command)?,
        LicenceChangedKind::Deactivated,
    )
}

/// Re-checks the lease now.
#[tauri::command]
pub fn longhorn_licence_refresh<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceRefreshCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    emitting(
        &window,
        state.service.refresh(window.label(), command)?,
        LicenceChangedKind::Refreshed,
    )
}

/// Releases a named machine's seat.
#[tauri::command]
pub fn longhorn_licence_release_seat<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceReleaseSeatCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    emitting(
        &window,
        state.service.release_seat(window.label(), command)?,
        LicenceChangedKind::Deactivated,
    )
}

/// Renames a machine's seat.
#[tauri::command]
pub fn longhorn_licence_rename_seat<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceRenameSeatCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    emitting(
        &window,
        state.service.rename_seat(window.label(), command)?,
        LicenceChangedKind::SeatRelabelled,
    )
}

/// Publishes a trusted licence invalidation hint after an external commit.
///
/// The authority's state moves without a command in one case that matters:
/// the scheduled lease renewal. A consumer holding a snapshot needs to hear
/// about it, and has nothing to hear it from otherwise.
pub fn publish_licence_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: LicenceChangedEvent,
) -> Result<(), LicenceHostError> {
    app.emit(LICENCE_CHANGED_EVENT, event)
        .map_err(|error| LicenceHostError::event_publication(error.to_string()))
}

/// Projects a non-durable hint only for a committed outcome.
///
/// A rejection leaves the state as it was, so there is nothing to invalidate
/// and a consumer that refetched on one would be refetching for nothing.
#[must_use]
pub fn licence_changed_event(
    outcome: &LicenceOutcomeProjection,
    kind: LicenceChangedKind,
) -> Option<LicenceChangedEvent> {
    let LicenceOutcomeProjection::Committed { snapshot } = outcome else {
        return None;
    };
    Some(LicenceChangedEvent {
        protocol_version: LicenceProtocolVersion::CURRENT,
        authority_epoch: snapshot.authority_epoch,
        kind,
    })
}

/// Emits the hint for a committed outcome, then returns it.
fn emitting<R: Runtime>(
    window: &WebviewWindow<R>,
    outcome: LicenceOutcomeProjection,
    kind: LicenceChangedKind,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    if let Some(event) = licence_changed_event(&outcome, kind)
        && let Err(error) = window.emit(LICENCE_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("licence.changed-emit", error);
    }
    Ok(outcome)
}
