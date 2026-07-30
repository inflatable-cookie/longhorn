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

use crate::ConfigOperationsHostError;

/// Object-safe commands retained in Tauri managed state.
pub trait ConfigOperationsCommandService: Send + Sync {
    /// Returns one caller-authorized snapshot.
    fn snapshot(
        &self,
        caller: &str,
        command: ConfigSnapshotCommand,
    ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError>;

    /// Inspects one target profile.
    fn inspect_storage_transition(
        &self,
        caller: &str,
        command: StorageTransitionInspectCommand,
    ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError>;

    /// Executes one matching confirmed transition.
    fn execute_storage_transition(
        &self,
        caller: &str,
        command: StorageTransitionExecuteCommand,
    ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError>;

    /// Recovers interrupted transition state.
    fn recover_storage(
        &self,
        caller: &str,
        command: StorageRecoveryCommand,
    ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError>;

    /// Applies receipt-bound source cleanup.
    fn cleanup_storage(
        &self,
        caller: &str,
        command: StorageCleanupCommand,
    ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError>;

    /// Creates and operationally publishes a backup.
    fn create_backup(
        &self,
        caller: &str,
        command: BackupCreateCommand,
    ) -> Result<BackupCreateOutcome, ConfigOperationsHostError>;

    /// Exports a proven archive through host selection.
    fn export_backup(
        &self,
        caller: &str,
        command: BackupExportCommand,
    ) -> Result<BackupExportOutcome, ConfigOperationsHostError>;

    /// Applies one matching host-owned retention plan.
    fn apply_backup_retention(
        &self,
        caller: &str,
        command: BackupRetentionApplyCommand,
    ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError>;

    /// Selects, unlocks, and inspects one restore archive.
    fn inspect_restore(
        &self,
        caller: &str,
        command: RestoreInspectCommand,
    ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError>;

    /// Plans explicit choices against fresh current evidence.
    fn plan_restore(
        &self,
        caller: &str,
        command: RestorePlanCommand,
    ) -> Result<RestorePlanOutcome, ConfigOperationsHostError>;

    /// Executes one matching ordinary restore plan.
    fn execute_restore(
        &self,
        caller: &str,
        command: RestoreExecuteCommand,
    ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError>;

    /// Executes one matching custom-adapter restore.
    fn execute_adapter_restore(
        &self,
        caller: &str,
        command: RestoreAdapterExecuteCommand,
    ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError>;

    /// Recovers or cleans one restore journal.
    fn recover_restore(
        &self,
        caller: &str,
        command: RestoreRecoveryCommand,
    ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError>;
}

/// Type-erased config operations installed once in Tauri managed state.
pub struct TauriConfigOperationsState {
    service: Arc<dyn ConfigOperationsCommandService>,
}

impl TauriConfigOperationsState {
    /// Wraps one explicitly injected command assembly.
    #[must_use]
    pub fn new(service: Arc<dyn ConfigOperationsCommandService>) -> Self {
        Self { service }
    }
}

/// Returns one exact caller-authorized operations snapshot.
#[tauri::command]
pub fn longhorn_config_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: ConfigSnapshotCommand,
) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
    state.service.snapshot(window.label(), command)
}

/// Inspects one storage profile target without mutation.
#[tauri::command]
pub fn longhorn_config_storage_inspect<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageTransitionInspectCommand,
) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
    state
        .service
        .inspect_storage_transition(window.label(), command)
}

/// Executes one matching host-retained transition plan.
#[tauri::command]
pub fn longhorn_config_storage_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageTransitionExecuteCommand,
) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
    state
        .service
        .execute_storage_transition(window.label(), command)
}

/// Recovers one interrupted journaled storage transition.
#[tauri::command]
pub fn longhorn_config_storage_recover<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageRecoveryCommand,
) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
    state.service.recover_storage(window.label(), command)
}

/// Applies source cleanup authorized by one exact committed receipt.
#[tauri::command]
pub fn longhorn_config_storage_cleanup<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: StorageCleanupCommand,
) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
    state.service.cleanup_storage(window.label(), command)
}

/// Captures and operationally publishes one backup.
#[tauri::command]
pub fn longhorn_config_backup_create<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupCreateCommand,
) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
    state.service.create_backup(window.label(), command)
}

/// Exports one proven archive to a host-selected target.
#[tauri::command]
pub fn longhorn_config_backup_export<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupExportCommand,
) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
    state.service.export_backup(window.label(), command)
}

/// Applies one confirmed host-owned retention plan.
#[tauri::command]
pub fn longhorn_config_backup_retention<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: BackupRetentionApplyCommand,
) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
    state
        .service
        .apply_backup_retention(window.label(), command)
}

/// Selects, unlocks, and inspects one archive without mutation.
#[tauri::command]
pub fn longhorn_config_restore_inspect<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreInspectCommand,
) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
    state.service.inspect_restore(window.label(), command)
}

/// Binds explicit choices and fresh evidence into one restore plan.
#[tauri::command]
pub fn longhorn_config_restore_plan<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestorePlanCommand,
) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
    state.service.plan_restore(window.label(), command)
}

/// Stages and executes one matching host-retained restore plan.
#[tauri::command]
pub fn longhorn_config_restore_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreExecuteCommand,
) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
    state.service.execute_restore(window.label(), command)
}

/// Executes one explicitly confirmed custom-adapter restore.
#[tauri::command]
pub fn longhorn_config_restore_adapter_execute<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreAdapterExecuteCommand,
) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
    state
        .service
        .execute_adapter_restore(window.label(), command)
}

/// Verifies rollback or cleans one terminal restore journal.
#[tauri::command]
pub fn longhorn_config_restore_recover<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriConfigOperationsState>,
    command: RestoreRecoveryCommand,
) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
    state.service.recover_restore(window.label(), command)
}
