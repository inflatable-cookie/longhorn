use std::sync::Arc;

use longhorn_command_config::{
    CommandCatalogueChangedEvent, CommandCatalogueSnapshot, CommandKeymapChangedEvent,
    CommandKeymapCommit, CommandKeymapLoadOutcome, CommandKeymapMutationOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapProtocolVersion, CommandKeymapReset,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{CommandHandlerAssembly, CommandHostAuthority, CommandHostError};

/// Catalogue invalidation hint event.
pub const COMMAND_CATALOGUE_CHANGED_EVENT: &str = "longhorn://command/catalogue-changed";
/// Effective keymap invalidation hint event.
pub const COMMAND_KEYMAP_CHANGED_EVENT: &str = "longhorn://command/keymap-changed";

/// Type-erased command host installed once in Tauri managed state.
pub struct TauriCommandState {
    assembly: Arc<CommandHandlerAssembly<dyn CommandHostAuthority>>,
}

impl TauriCommandState {
    /// Wraps one explicitly injected command assembly.
    #[must_use]
    pub fn new(assembly: Arc<CommandHandlerAssembly<dyn CommandHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriCommandState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &CommandHandlerAssembly<dyn CommandHostAuthority>,
        &str,
    ) -> Result<T, CommandHostError>
    + Send
    + 'static,
) -> Result<T, CommandHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| CommandHostError::state_unavailable())?
}

/// Returns the caller-authorized sealed command catalogue.
#[tauri::command]
pub async fn longhorn_command_catalogue<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
) -> Result<CommandCatalogueSnapshot, CommandHostError> {
    run_blocking(state, &window, |assembly, label| assembly.catalogue(label)).await
}

/// Loads the checked effective keymap.
#[tauri::command]
pub async fn longhorn_command_keymap<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
) -> Result<CommandKeymapLoadOutcome, CommandHostError> {
    run_blocking(state, &window, |assembly, label| assembly.keymap(label)).await
}

/// Previews one checked keymap patch.
#[tauri::command]
pub async fn longhorn_command_keymap_preview<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
    request: CommandKeymapPreview,
) -> Result<CommandKeymapPreviewResult, CommandHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.preview(label, request)
    })
    .await
}

/// Commits one exact accepted keymap preview.
#[tauri::command]
pub async fn longhorn_command_keymap_commit<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
    request: CommandKeymapCommit,
) -> Result<CommandKeymapMutationResult, CommandHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.commit(label, request)
    })
    .await?;
    if let Err(error) = publish_mutation_hint(&window, &result) {
        longhorn_core::report_best_effort_failure("command.mutation-hint-emit", error);
    }
    Ok(result)
}

/// Resets the effective keymap to the compiled default.
#[tauri::command]
pub async fn longhorn_command_keymap_reset<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
    request: CommandKeymapReset,
) -> Result<CommandKeymapMutationResult, CommandHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.reset(label, request)
    })
    .await?;
    if let Err(error) = publish_mutation_hint(&window, &result) {
        longhorn_core::report_best_effort_failure("command.mutation-hint-emit", error);
    }
    Ok(result)
}

/// Publishes a trusted catalogue invalidation hint after host recomposition.
pub fn publish_catalogue_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: CommandCatalogueChangedEvent,
) -> Result<(), CommandHostError> {
    app.emit(COMMAND_CATALOGUE_CHANGED_EVENT, event)
        .map_err(|error| CommandHostError::event_publication(error.to_string()))
}

/// Publishes a trusted keymap invalidation hint after an external authority change.
pub fn publish_keymap_changed<R: Runtime>(
    app: &AppHandle<R>,
    event: CommandKeymapChangedEvent,
) -> Result<(), CommandHostError> {
    app.emit(COMMAND_KEYMAP_CHANGED_EVENT, event)
        .map_err(|error| CommandHostError::event_publication(error.to_string()))
}

fn publish_mutation_hint<R: Runtime>(
    window: &WebviewWindow<R>,
    result: &CommandKeymapMutationResult,
) -> Result<(), CommandHostError> {
    let Some(event) = keymap_changed_event(result) else {
        return Ok(());
    };
    window
        .emit(COMMAND_KEYMAP_CHANGED_EVENT, event)
        .map_err(|error| CommandHostError::event_publication(error.to_string()))
}

/// Projects a non-durable revision hint only for changed successful mutation.
#[must_use]
pub fn keymap_changed_event(
    result: &CommandKeymapMutationResult,
) -> Option<CommandKeymapChangedEvent> {
    let CommandKeymapMutationResult::Applied { snapshot, receipt } = result else {
        return None;
    };
    (receipt.outcome == CommandKeymapMutationOutcome::Changed).then_some(
        CommandKeymapChangedEvent {
            protocol_version: CommandKeymapProtocolVersion::CURRENT,
            registry_generation: snapshot.registry_generation,
            keymap_revision: snapshot.state.revision,
        },
    )
}
