use std::sync::Arc;

use longhorn_config::{
    BackupCreateCommand, BackupCreateOutcome, BackupExportCommand, BackupExportOutcome,
    BackupRetentionApplyCommand, BackupRetentionApplyOutcome, ConfigOperationsSnapshot,
    ConfigSnapshotCommand, RestoreAdapterExecuteCommand, RestoreAdapterExecuteOutcome,
    RestoreExecuteCommand, RestoreExecuteOutcome, RestoreInspectCommand, RestoreInspectOutcome,
    RestorePlanCommand, RestorePlanOutcome, RestoreRecoveryCommand,
    RestoreRecoveryOutcomeProjection, StorageCleanupCommand, StorageCleanupOutcome,
    StorageRecoveryCommand, StorageRecoveryOutcome, StorageTransitionExecuteCommand,
    StorageTransitionExecuteOutcome, StorageTransitionInspectCommand,
    StorageTransitionInspectOutcome,
};
use tauri::{Runtime, State, WebviewWindow};

use crate::{
    ConfigOperationsHandlerAssembly, ConfigOperationsHostAuthority, ConfigOperationsHostError,
};

/// Type-erased config operations installed once in Tauri managed state.
pub struct TauriConfigOperationsState {
    assembly: Arc<ConfigOperationsHandlerAssembly<dyn ConfigOperationsHostAuthority>>,
}

impl TauriConfigOperationsState {
    /// Wraps one explicitly injected command assembly.
    #[must_use]
    pub fn new(
        assembly: Arc<ConfigOperationsHandlerAssembly<dyn ConfigOperationsHostAuthority>>,
    ) -> Self {
        Self { assembly }
    }
}

async fn run_blocking<R: Runtime, T: Send + 'static>(
    state: State<'_, TauriConfigOperationsState>,
    window: WebviewWindow<R>,
    action: impl FnOnce(
        &ConfigOperationsHandlerAssembly<dyn ConfigOperationsHostAuthority>,
        &str,
    ) -> Result<T, ConfigOperationsHostError>
    + Send
    + 'static,
) -> Result<T, ConfigOperationsHostError> {
    let assembly = Arc::clone(&state.assembly);
    let label = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || action(&assembly, &label))
        .await
        .map_err(|_| ConfigOperationsHostError::state_unavailable())?
}

/// Returns one exact caller-authorized operations snapshot.
#[tauri::command]
pub async fn longhorn_config_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: ConfigSnapshotCommand,
) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.snapshot(label, command)
    })
    .await
}

/// Inspects one storage profile target without mutation.
#[tauri::command]
pub async fn longhorn_config_storage_inspect<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageTransitionInspectCommand,
) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.inspect_storage_transition(label, command)
    })
    .await
}

/// Executes one matching host-retained transition plan.
#[tauri::command]
pub async fn longhorn_config_storage_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageTransitionExecuteCommand,
) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.execute_storage_transition(label, command)
    })
    .await
}

/// Recovers one interrupted journaled storage transition.
#[tauri::command]
pub async fn longhorn_config_storage_recover<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageRecoveryCommand,
) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.recover_storage(label, command)
    })
    .await
}

/// Applies source cleanup authorized by one exact committed receipt.
#[tauri::command]
pub async fn longhorn_config_storage_cleanup<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageCleanupCommand,
) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.cleanup_storage(label, command)
    })
    .await
}

/// Captures and operationally publishes one backup.
#[tauri::command]
pub async fn longhorn_config_backup_create<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupCreateCommand,
) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.create_backup(label, command)
    })
    .await
}

/// Exports one proven archive to a host-selected target.
#[tauri::command]
pub async fn longhorn_config_backup_export<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupExportCommand,
) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.export_backup(label, command)
    })
    .await
}

/// Applies one confirmed host-owned retention plan.
#[tauri::command]
pub async fn longhorn_config_backup_retention<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupRetentionApplyCommand,
) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.apply_backup_retention(label, command)
    })
    .await
}

/// Selects, unlocks, and inspects one archive without mutation.
#[tauri::command]
pub async fn longhorn_config_restore_inspect<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreInspectCommand,
) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.inspect_restore(label, command)
    })
    .await
}

/// Binds explicit choices and fresh evidence into one restore plan.
#[tauri::command]
pub async fn longhorn_config_restore_plan<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestorePlanCommand,
) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.plan_restore(label, command)
    })
    .await
}

/// Stages and executes one matching host-retained restore plan.
#[tauri::command]
pub async fn longhorn_config_restore_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreExecuteCommand,
) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.execute_restore(label, command)
    })
    .await
}

/// Executes one explicitly confirmed custom-adapter restore.
#[tauri::command]
pub async fn longhorn_config_restore_adapter_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreAdapterExecuteCommand,
) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.execute_adapter_restore(label, command)
    })
    .await
}

/// Verifies rollback or cleans one terminal restore journal.
#[tauri::command]
pub async fn longhorn_config_restore_recover<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreRecoveryCommand,
) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
    run_blocking(state, window, move |assembly, label| {
        assembly.recover_restore(label, command)
    })
    .await
}
