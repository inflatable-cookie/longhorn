use std::sync::Arc;

use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationOutcome,
    SettingsMutationResult, SettingsRegistryChangedEvent, SettingsRegistryGeneration,
    SettingsRegistrySnapshot, SettingsResetCommand, SettingsScopeChangedEvent,
    SettingsScopeRevision,
};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};

use crate::{SettingsHandlerAssembly, SettingsHostAuthority, SettingsHostError};

/// Registry invalidation hint event.
pub const SETTINGS_REGISTRY_CHANGED_EVENT: &str = "longhorn://settings/registry-changed";
/// Scope invalidation hint event.
pub const SETTINGS_SCOPE_CHANGED_EVENT: &str = "longhorn://settings/scope-changed";

/// Type-erased settings commands installed once in Tauri managed state.
pub struct TauriSettingsState {
    assembly: Arc<SettingsHandlerAssembly<dyn SettingsHostAuthority>>,
}

impl TauriSettingsState {
    /// Wraps one explicitly injected command assembly.
    #[must_use]
    pub fn new(assembly: Arc<SettingsHandlerAssembly<dyn SettingsHostAuthority>>) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriSettingsState>,
    window: &WebviewWindow<R>,
    action: impl FnOnce(
        &SettingsHandlerAssembly<dyn SettingsHostAuthority>,
        &str,
    ) -> Result<T, SettingsHostError>
    + Send
    + 'static,
) -> Result<T, SettingsHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| SettingsHostError::state_unavailable())?
}

/// Returns the caller-authorized sealed settings registry.
#[tauri::command]
pub async fn longhorn_settings_registry<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSettingsState>,
) -> Result<SettingsRegistrySnapshot, SettingsHostError> {
    run_blocking(state, &window, |assembly, label| assembly.registry(label)).await
}

/// Loads one checked settings scope.
#[tauri::command]
pub async fn longhorn_settings_load<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSettingsState>,
    command: SettingsLoadCommand,
) -> Result<SettingsLoadOutcome, SettingsHostError> {
    run_blocking(state, &window, move |assembly, label| {
        assembly.load(label, command)
    })
    .await
}

/// Applies one checked failure-atomic settings intent.
#[tauri::command]
pub async fn longhorn_settings_apply<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSettingsState>,
    command: SettingsApplyCommand,
) -> Result<SettingsMutationResult, SettingsHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.apply(label, command)
    })
    .await?;
    // Events are non-durable hints. A failed hint must not erase a durable
    // mutation receipt returned to the invoking client.
    if let Err(error) = publish_mutation_hint(&window, &result) {
        longhorn_core::report_best_effort_failure("settings.mutation-hint-emit", error);
    }
    Ok(result)
}

/// Resets selected user overrides through one checked apply unit.
#[tauri::command]
pub async fn longhorn_settings_reset<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriSettingsState>,
    command: SettingsResetCommand,
) -> Result<SettingsMutationResult, SettingsHostError> {
    let result = run_blocking(state, &window, move |assembly, label| {
        assembly.reset(label, command)
    })
    .await?;
    // Events are non-durable hints. A failed hint must not erase a durable
    // mutation receipt returned to the invoking client.
    if let Err(error) = publish_mutation_hint(&window, &result) {
        longhorn_core::report_best_effort_failure("settings.mutation-hint-emit", error);
    }
    Ok(result)
}

/// Publishes a trusted registry invalidation hint after host recomposition.
pub fn publish_registry_changed<R: Runtime>(
    app: &AppHandle<R>,
    generation: SettingsRegistryGeneration,
) -> Result<(), SettingsHostError> {
    app.emit(
        SETTINGS_REGISTRY_CHANGED_EVENT,
        SettingsRegistryChangedEvent {
            protocol_version: longhorn_settings::SettingsProtocolVersion::CURRENT,
            registry_generation: generation,
        },
    )
    .map_err(|error| SettingsHostError::event_publication(error.to_string()))
}

/// Publishes a trusted scope invalidation hint after external authority change.
pub fn publish_scope_changed<R: Runtime>(
    app: &AppHandle<R>,
    registry_generation: SettingsRegistryGeneration,
    scope_id: longhorn_core::SettingsScopeId,
    scope_revision: SettingsScopeRevision,
) -> Result<(), SettingsHostError> {
    app.emit(
        SETTINGS_SCOPE_CHANGED_EVENT,
        SettingsScopeChangedEvent {
            protocol_version: longhorn_settings::SettingsProtocolVersion::CURRENT,
            registry_generation,
            scope_id,
            scope_revision,
        },
    )
    .map_err(|error| SettingsHostError::event_publication(error.to_string()))
}

fn publish_mutation_hint<R: Runtime>(
    window: &WebviewWindow<R>,
    result: &SettingsMutationResult,
) -> Result<(), SettingsHostError> {
    let Some(event) = mutation_changed_event(result) else {
        return Ok(());
    };
    window
        .emit(SETTINGS_SCOPE_CHANGED_EVENT, event)
        .map_err(|error| SettingsHostError::event_publication(error.to_string()))
}

/// Projects a non-durable revision hint only for a changed successful mutation.
#[must_use]
pub fn mutation_changed_event(
    result: &SettingsMutationResult,
) -> Option<SettingsScopeChangedEvent> {
    let SettingsMutationResult::Applied { snapshot, receipt } = result else {
        return None;
    };
    (receipt.outcome == SettingsMutationOutcome::Changed)
        .then(|| SettingsScopeChangedEvent::from(snapshot))
}
