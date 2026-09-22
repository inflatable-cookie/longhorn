use std::sync::Arc;

use longhorn_update::{
    UpdateApplyCommand, UpdateCancelCommand, UpdateChangedEvent, UpdateChangedKind,
    UpdateCheckCommand, UpdateDeferCommand, UpdateOutcomeProjection, UpdatePrepareCommand,
    UpdateProgressEvent, UpdateProtocolVersion, UpdateSelectChannelCommand, UpdateSnapshot,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::UpdateHostError;

/// Non-durable committed update invalidation hint.
pub const UPDATE_CHANGED_EVENT: &str = "longhorn://update/changed";

/// Live byte progress, published while a transfer runs.
///
/// Separate from `UPDATE_CHANGED_EVENT` because it carries a value rather than
/// an invalidation: a snapshot read during the transfer still shows the
/// retained state, so this is the only channel that reports bytes as they
/// arrive.
pub const UPDATE_PROGRESS_EVENT: &str = "longhorn://update/progress";

/// Object-safe update surface retained in Tauri managed state.
pub trait UpdateHostService: Send + Sync {
    /// Returns the caller-authorized update state.
    fn snapshot(&self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError>;
    /// Asks the source for the channel's current manifest and records it.
    fn check(
        &self,
        caller: &str,
        command: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
    /// Follows a different channel from now on.
    fn select_channel(
        &self,
        caller: &str,
        command: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
    /// Declines a version for now.
    fn defer(
        &self,
        caller: &str,
        command: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
    /// Fetches, verifies and retains, reporting progress as bytes arrive.
    ///
    /// The authority lock is not held across the transfer; the report is the
    /// only thing the transfer touches.
    fn prepare(
        &self,
        caller: &str,
        command: UpdatePrepareCommand,
        progress: &mut dyn FnMut(UpdateProgressEvent),
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
    /// Applies the retained staged artifact.
    fn apply(
        &self,
        caller: &str,
        command: UpdateApplyCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
    /// Discards the retained staged artifact.
    fn cancel(
        &self,
        caller: &str,
        command: UpdateCancelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
}

/// Type-erased update host installed once in Tauri managed state.
pub struct TauriUpdateState {
    service: Arc<dyn UpdateHostService>,
}

impl TauriUpdateState {
    /// Wraps one explicitly injected update assembly.
    #[must_use]
    pub fn new(service: Arc<dyn UpdateHostService>) -> Self {
        Self { service }
    }
}

/// Returns caller-authorized update state.
#[tauri::command]
pub fn longhorn_update_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
) -> Result<UpdateSnapshot, UpdateHostError> {
    state.service.snapshot(window.label())
}

/// Asks the source for the channel's current manifest.
#[tauri::command]
pub fn longhorn_update_check<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateCheckCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    emitting(&window, state.service.check(window.label(), command)?)
}

/// Follows a different channel from now on.
#[tauri::command]
pub fn longhorn_update_select_channel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateSelectChannelCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    emitting(
        &window,
        state.service.select_channel(window.label(), command)?,
    )
}

/// Declines a version for now.
#[tauri::command]
pub fn longhorn_update_defer<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateDeferCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    emitting(&window, state.service.defer(window.label(), command)?)
}

/// Fetches, verifies and retains an update for a later apply.
///
/// Runs on a blocking thread so the transfer does not hold the main thread,
/// and byte progress leaves through `UPDATE_PROGRESS_EVENT` while it runs.
#[tauri::command]
pub async fn longhorn_update_prepare<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdatePrepareCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let outcome = service.prepare(&label, command, &mut |event| {
            emitting_progress(&window, event);
        })?;
        emitting(&window, outcome)
    })
    .await
    .map_err(|_| UpdateHostError::state_unavailable())?
}

/// Applies the retained staged artifact, replacing the application.
#[tauri::command]
pub fn longhorn_update_apply<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateApplyCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    emitting(&window, state.service.apply(window.label(), command)?)
}

/// Discards the retained staged artifact.
#[tauri::command]
pub fn longhorn_update_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateCancelCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    emitting(&window, state.service.cancel(window.label(), command)?)
}

/// Publishes a trusted update invalidation hint after an external commit.
///
/// The controller's state moves without a command in one case: a check the
/// application ran on a timer. A consumer holding a snapshot needs to hear
/// about it, and has nothing to hear it from otherwise.
pub fn publish_update_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: UpdateChangedEvent,
) -> Result<(), UpdateHostError> {
    app.emit(UPDATE_CHANGED_EVENT, event)
        .map_err(|error| UpdateHostError::event_publication(error.to_string()))
}

/// Projects a non-durable hint only for a committed outcome.
///
/// A rejection leaves the state as it was, so there is nothing to invalidate.
/// The kind is `Progressed` for everything the controller commits that is not
/// one of the three specific causes: progress is what moved, and inventing a
/// finer kind here would be guessing at a distinction the snapshot already
/// carries.
#[must_use]
pub fn update_changed_event(
    outcome: &UpdateOutcomeProjection,
    kind: UpdateChangedKind,
) -> Option<UpdateChangedEvent> {
    let UpdateOutcomeProjection::Committed { snapshot } = outcome else {
        return None;
    };
    Some(UpdateChangedEvent {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: snapshot.authority_epoch,
        kind,
    })
}

/// Emits the hint for a committed outcome, then returns it.
fn emitting<R: Runtime>(
    window: &WebviewWindow<R>,
    outcome: UpdateOutcomeProjection,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    if let Some(event) = update_changed_event(&outcome, UpdateChangedKind::Progressed)
        && let Err(error) = window.emit(UPDATE_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("update.changed-emit", error);
    }
    Ok(outcome)
}

/// Publishes one live progress report. A failed render is informational, not a
/// reason for the transfer to stop reporting.
fn emitting_progress<R: Runtime>(window: &WebviewWindow<R>, event: UpdateProgressEvent) {
    if let Err(error) = window.emit(UPDATE_PROGRESS_EVENT, event) {
        longhorn_core::report_best_effort_failure("update.progress-emit", error);
    }
}
