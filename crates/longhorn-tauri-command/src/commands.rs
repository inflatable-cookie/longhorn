use std::sync::Arc;

use longhorn_command_config::{
    CommandCatalogueChangedEvent, CommandCatalogueSnapshot, CommandKeymapChangedEvent,
    CommandKeymapCommit, CommandKeymapLoadOutcome, CommandKeymapMutationOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapProtocolVersion, CommandKeymapReset,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::CommandHostError;

/// Catalogue invalidation hint event.
pub const COMMAND_CATALOGUE_CHANGED_EVENT: &str = "longhorn://command/catalogue-changed";
/// Effective keymap invalidation hint event.
pub const COMMAND_KEYMAP_CHANGED_EVENT: &str = "longhorn://command/keymap-changed";

/// Object-safe command surface retained in Tauri managed state.
pub trait CommandHostService: Send + Sync {
    /// Returns one caller-authorized sealed catalogue.
    fn catalogue(&self, caller: &str) -> Result<CommandCatalogueSnapshot, CommandHostError>;

    /// Loads one caller-authorized effective keymap.
    fn keymap(&self, caller: &str) -> Result<CommandKeymapLoadOutcome, CommandHostError>;

    /// Previews one checked keymap patch.
    fn preview(
        &self,
        caller: &str,
        request: CommandKeymapPreview,
    ) -> Result<CommandKeymapPreviewResult, CommandHostError>;

    /// Commits one digest-bound keymap patch.
    fn commit(
        &self,
        caller: &str,
        request: CommandKeymapCommit,
    ) -> Result<CommandKeymapMutationResult, CommandHostError>;

    /// Resets caller-authorized keymap state.
    fn reset(
        &self,
        caller: &str,
        request: CommandKeymapReset,
    ) -> Result<CommandKeymapMutationResult, CommandHostError>;
}

/// Type-erased command host installed once in Tauri managed state.
pub struct TauriCommandState {
    service: Arc<dyn CommandHostService>,
}

impl TauriCommandState {
    /// Wraps one explicitly injected command assembly.
    #[must_use]
    pub fn new(service: Arc<dyn CommandHostService>) -> Self {
        Self { service }
    }
}

/// Returns the caller-authorized sealed command catalogue.
#[tauri::command]
pub async fn longhorn_command_catalogue<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
) -> Result<CommandCatalogueSnapshot, CommandHostError> {
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || service.catalogue(&label))
        .await
        .map_err(|_| CommandHostError::state_unavailable())?
}

/// Loads the checked effective keymap.
#[tauri::command]
pub async fn longhorn_command_keymap<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
) -> Result<CommandKeymapLoadOutcome, CommandHostError> {
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || service.keymap(&label))
        .await
        .map_err(|_| CommandHostError::state_unavailable())?
}

/// Previews one checked keymap patch.
#[tauri::command]
pub async fn longhorn_command_keymap_preview<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
    request: CommandKeymapPreview,
) -> Result<CommandKeymapPreviewResult, CommandHostError> {
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || service.preview(&label, request))
        .await
        .map_err(|_| CommandHostError::state_unavailable())?
}

/// Commits one exact accepted keymap preview.
#[tauri::command]
pub async fn longhorn_command_keymap_commit<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriCommandState>,
    request: CommandKeymapCommit,
) -> Result<CommandKeymapMutationResult, CommandHostError> {
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    let result = tauri::async_runtime::spawn_blocking(move || service.commit(&label, request))
        .await
        .map_err(|_| CommandHostError::state_unavailable())??;
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
    let service = Arc::clone(&state.service);
    let label = window.label().to_owned();
    let result = tauri::async_runtime::spawn_blocking(move || service.reset(&label, request))
        .await
        .map_err(|_| CommandHostError::state_unavailable())??;
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
