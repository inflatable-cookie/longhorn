use std::sync::Arc;

use longhorn_update::{
    UpdateApplyCommand, UpdateCancelCommand, UpdateChangedEvent, UpdateChangedKind,
    UpdateCheckCommand, UpdateDeferCommand, UpdateOutcomeProjection, UpdatePrepareCommand,
    UpdateProgressEvent, UpdateProtocolVersion, UpdateSelectChannelCommand, UpdateSnapshot,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{UpdateHandlerAssembly, UpdateHostAuthority, UpdateHostError};

/// Non-durable committed update invalidation hint.
pub const UPDATE_CHANGED_EVENT: &str = "longhorn://update/changed";

/// Live byte progress, published while a transfer runs.
///
/// Separate from `UPDATE_CHANGED_EVENT` because it carries a value rather than
/// an invalidation: a snapshot read during the transfer still shows the
/// retained state, so this is the only channel that reports bytes as they
/// arrive.
pub const UPDATE_PROGRESS_EVENT: &str = "longhorn://update/progress";

/// Type-erased update host installed once in Tauri managed state.
pub struct TauriUpdateState {
    assembly: Arc<UpdateHandlerAssembly<dyn UpdateHostAuthority>>,
}

impl TauriUpdateState {
    /// Wraps one explicitly injected update assembly.
    #[must_use]
    pub fn new(assembly: Arc<UpdateHandlerAssembly<dyn UpdateHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriUpdateState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &UpdateHandlerAssembly<dyn UpdateHostAuthority>,
        &str,
    ) -> Result<T, UpdateHostError>
    + Send
    + 'static,
) -> Result<T, UpdateHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| UpdateHostError::state_unavailable())?
}

/// Returns caller-authorized update state.
#[tauri::command]
pub async fn longhorn_update_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
) -> Result<UpdateSnapshot, UpdateHostError> {
    run_blocking(state, &window, |assembly, label| assembly.snapshot(label)).await
}

/// Asks the source for the channel's current manifest.
#[tauri::command]
pub async fn longhorn_update_check<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateCheckCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.check(label, command)
    })
    .await?;
    emitting(&window, outcome, UpdateChangedKind::Checked)
}

/// Follows a different channel from now on.
#[tauri::command]
pub async fn longhorn_update_select_channel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateSelectChannelCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.select_channel(label, command)
    })
    .await?;
    emitting(&window, outcome, UpdateChangedKind::ChannelSelected)
}

/// Declines a version for now.
#[tauri::command]
pub async fn longhorn_update_defer<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateDeferCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.defer(label, command)
    })
    .await?;
    emitting(&window, outcome, UpdateChangedKind::Deferred)
}

/// Fetches, verifies and retains an update for a later apply.
///
/// Runs on a blocking thread so the transfer does not hold the main thread,
/// and byte progress leaves through `UPDATE_PROGRESS_EVENT` while it runs.
pub async fn longhorn_update_prepare<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdatePrepareCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    prepare(window, state, command, None).await
}

// Tauri's handler macro dispatches through the exported command wrapper. Keep
// the three-argument Rust entry point while accepting an optional IPC channel.
#[doc(hidden)]
#[macro_export]
macro_rules! __cmd__longhorn_update_prepare {
    ($path:path, $invoke:ident) => {
        $crate::dispatch_prepare($invoke)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tauri_command_name_longhorn_update_prepare {
    () => {
        "longhorn_update_prepare"
    };
}

pub use prepare_ipc::dispatch_prepare;

mod prepare_ipc {
    use tauri::ipc::{Invoke, JavaScriptChannelId};
    use tauri::{Runtime, State, WebviewWindow};

    use super::{TauriUpdateState, UpdateHostError, UpdateOutcomeProjection, UpdatePrepareCommand};

    #[doc(hidden)]
    /// Dispatches the prepare IPC command through Tauri's argument parser.
    pub fn dispatch_prepare<R: Runtime>(invoke: Invoke<R>) -> bool {
        let handle: fn(Invoke<R>) -> bool = tauri::generate_handler![self::longhorn_update_prepare];
        handle(invoke)
    }

    #[tauri::command]
    async fn longhorn_update_prepare<R: Runtime>(
        window: WebviewWindow<R>,
        state: State<'_, TauriUpdateState>,
        command: UpdatePrepareCommand,
        on_progress: Option<JavaScriptChannelId>,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        let progress = on_progress.map(|id| id.channel_on(window.as_ref().clone()));
        super::prepare(window, state, command, progress).await
    }
}

async fn prepare<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdatePrepareCommand,
    progress: Option<Channel<UpdateProgressEvent>>,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    let emit_window = window.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        assembly.prepare(&label, command, &mut |event| {
            let result = match &progress {
                Some(channel) => channel.send(event),
                None => emit_window.emit(UPDATE_PROGRESS_EVENT, event),
            };
            if let Err(error) = result {
                longhorn_core::report_best_effort_failure("update.progress-emit", error);
            }
        })
    })
    .await
    .map_err(|_| UpdateHostError::state_unavailable())??;
    emitting(&window, outcome, UpdateChangedKind::Progressed)
}

/// Applies the retained staged artifact, replacing the application.
///
/// The whole call runs under the consumer's exclusive admission lease; a
/// refused lease returns a deferred outcome rather than an adapter error.
#[tauri::command]
pub async fn longhorn_update_apply<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateApplyCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.apply(label, command)
    })
    .await?;
    let kind = if matches!(&outcome, UpdateOutcomeProjection::Committed { snapshot } if snapshot.deferral.is_some())
    {
        UpdateChangedKind::Deferred
    } else {
        UpdateChangedKind::Progressed
    };
    emitting(&window, outcome, kind)
}

/// Discards the retained staged artifact.
#[tauri::command]
pub async fn longhorn_update_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriUpdateState>,
    command: UpdateCancelCommand,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    let outcome = run_blocking(state, &window, move |assembly, label| {
        assembly.cancel(label, command)
    })
    .await?;
    emitting(&window, outcome, UpdateChangedKind::Progressed)
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
    kind: UpdateChangedKind,
) -> Result<UpdateOutcomeProjection, UpdateHostError> {
    if let Some(event) = update_changed_event(&outcome, kind)
        && let Err(error) = window.emit(UPDATE_CHANGED_EVENT, event)
    {
        longhorn_core::report_best_effort_failure("update.changed-emit", error);
    }
    Ok(outcome)
}
