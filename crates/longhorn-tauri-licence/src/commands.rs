use std::sync::Arc;

use longhorn_licence::{
    LicenceActivateCommand, LicenceChangedEvent, LicenceChangedKind, LicenceDeactivateCommand,
    LicenceOutcomeProjection, LicenceProtocolVersion, LicenceRefreshCommand,
    LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{LicenceHandlerAssembly, LicenceHostAuthority, LicenceHostError};

/// Non-durable committed licence invalidation hint.
pub const LICENCE_CHANGED_EVENT: &str = "longhorn://licence/changed";

/// Type-erased licence host installed once in Tauri managed state.
pub struct TauriLicenceState {
    assembly: Arc<LicenceHandlerAssembly<dyn LicenceHostAuthority>>,
}

impl TauriLicenceState {
    /// Wraps one explicitly injected licence assembly.
    #[must_use]
    pub fn new(assembly: Arc<LicenceHandlerAssembly<dyn LicenceHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriLicenceState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &LicenceHandlerAssembly<dyn LicenceHostAuthority>,
        &str,
    ) -> Result<T, LicenceHostError>
    + Send
    + 'static,
) -> Result<T, LicenceHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| LicenceHostError::state_unavailable())?
}

/// Returns caller-authorized licence state.
#[tauri::command]
pub async fn longhorn_licence_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
) -> Result<LicenceSnapshot, LicenceHostError> {
    run_blocking(state, &window, |assembly, label| assembly.snapshot(label)).await
}

/// Presents a credential and asks for a licence.
#[tauri::command]
pub async fn longhorn_licence_activate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceActivateCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.activate(label, command)
    })
    .await?;
    emitting(&window, outcome, LicenceChangedKind::Activated)
}

/// Releases this machine's seat.
#[tauri::command]
pub async fn longhorn_licence_deactivate<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceDeactivateCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.deactivate(label, command)
    })
    .await?;
    emitting(&window, outcome, LicenceChangedKind::Deactivated)
}

/// Re-checks the lease now.
#[tauri::command]
pub async fn longhorn_licence_refresh<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceRefreshCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.refresh(label, command)
    })
    .await?;
    emitting(&window, outcome, LicenceChangedKind::Refreshed)
}

/// Releases a named machine's seat.
#[tauri::command]
pub async fn longhorn_licence_release_seat<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceReleaseSeatCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.release_seat(label, command)
    })
    .await?;
    emitting(&window, outcome, LicenceChangedKind::Deactivated)
}

/// Renames a machine's seat.
#[tauri::command]
pub async fn longhorn_licence_rename_seat<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriLicenceState>,
    command: LicenceRenameSeatCommand,
) -> Result<LicenceOutcomeProjection, LicenceHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.rename_seat(label, command)
    })
    .await?;
    emitting(&window, outcome, LicenceChangedKind::SeatRelabelled)
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
