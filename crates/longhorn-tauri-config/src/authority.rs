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

use crate::ConfigOperationsHostError;

/// Consumer-injected authorization, plan custody, policy, and execution edge.
///
/// Implementations must re-resolve current filesystem evidence before
/// confirmed transition, cleanup, export, or retention work. They also own
/// idempotency storage for request ids and committed receipts.
pub trait ConfigOperationsAuthority: Send {
    /// Returns one caller-authorized exact snapshot.
    fn snapshot(
        &mut self,
        caller: &str,
        command: ConfigSnapshotCommand,
    ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError>;

    /// Inspects one target profile and retains the executable plan in host state.
    fn inspect_storage_transition(
        &mut self,
        caller: &str,
        command: StorageTransitionInspectCommand,
    ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError>;

    /// Rechecks and executes one matching host-retained transition plan.
    fn execute_storage_transition(
        &mut self,
        caller: &str,
        command: StorageTransitionExecuteCommand,
    ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError>;

    /// Recovers an interrupted journaled transition.
    fn recover_storage(
        &mut self,
        caller: &str,
        command: StorageRecoveryCommand,
    ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError>;

    /// Reconstructs cleanup only from a matching committed receipt.
    fn cleanup_storage(
        &mut self,
        caller: &str,
        command: StorageCleanupCommand,
    ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError>;

    /// Flushes or refuses pending publication, captures, and publishes.
    fn create_backup(
        &mut self,
        caller: &str,
        command: BackupCreateCommand,
    ) -> Result<BackupCreateOutcome, ConfigOperationsHostError>;

    /// Exports a proven archive through a host-selected target.
    fn export_backup(
        &mut self,
        caller: &str,
        command: BackupExportCommand,
    ) -> Result<BackupExportOutcome, ConfigOperationsHostError>;

    /// Rechecks and applies one matching host-owned retention plan.
    fn apply_backup_retention(
        &mut self,
        caller: &str,
        command: BackupRetentionApplyCommand,
    ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError>;

    /// Selects, unlocks, and inspects one archive without mutation.
    fn inspect_restore(
        &mut self,
        caller: &str,
        command: RestoreInspectCommand,
    ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError>;

    /// Binds explicit conflict choices and fresh evidence into a retained plan.
    fn plan_restore(
        &mut self,
        caller: &str,
        command: RestorePlanCommand,
    ) -> Result<RestorePlanOutcome, ConfigOperationsHostError>;

    /// Stages and executes one matching retained ordinary restore plan.
    fn execute_restore(
        &mut self,
        caller: &str,
        command: RestoreExecuteCommand,
    ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError>;

    /// Executes one explicitly confirmed custom-adapter operation.
    fn execute_adapter_restore(
        &mut self,
        caller: &str,
        command: RestoreAdapterExecuteCommand,
    ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError>;

    /// Verifies rollback or cleans one terminal restore journal.
    fn recover_restore(
        &mut self,
        caller: &str,
        command: RestoreRecoveryCommand,
    ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError>;
}
