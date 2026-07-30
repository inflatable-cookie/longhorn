use std::sync::Mutex;

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

use crate::{ConfigOperationsAuthority, ConfigOperationsCommandService, ConfigOperationsHostError};

/// Shared command assembly used by Tauri and direct/serialized tests.
pub struct ConfigOperationsHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> ConfigOperationsHandlerAssembly<A> {
    /// Binds one explicitly injected authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            authority: Mutex::new(authority),
        }
    }

    /// Runs trusted host work against the injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, ConfigOperationsHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| ConfigOperationsHostError::state_unavailable())
    }
}

impl<A> ConfigOperationsCommandService for ConfigOperationsHandlerAssembly<A>
where
    A: ConfigOperationsAuthority,
{
    fn snapshot(
        &self,
        caller: &str,
        command: ConfigSnapshotCommand,
    ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.snapshot(caller, command))?
    }

    fn inspect_storage_transition(
        &self,
        caller: &str,
        command: StorageTransitionInspectCommand,
    ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.inspect_storage_transition(caller, command))?
    }

    fn execute_storage_transition(
        &self,
        caller: &str,
        command: StorageTransitionExecuteCommand,
    ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.execute_storage_transition(caller, command))?
    }

    fn recover_storage(
        &self,
        caller: &str,
        command: StorageRecoveryCommand,
    ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.recover_storage(caller, command))?
    }

    fn cleanup_storage(
        &self,
        caller: &str,
        command: StorageCleanupCommand,
    ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.cleanup_storage(caller, command))?
    }

    fn create_backup(
        &self,
        caller: &str,
        command: BackupCreateCommand,
    ) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.create_backup(caller, command))?
    }

    fn export_backup(
        &self,
        caller: &str,
        command: BackupExportCommand,
    ) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.export_backup(caller, command))?
    }

    fn apply_backup_retention(
        &self,
        caller: &str,
        command: BackupRetentionApplyCommand,
    ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.apply_backup_retention(caller, command))?
    }

    fn inspect_restore(
        &self,
        caller: &str,
        command: RestoreInspectCommand,
    ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.inspect_restore(caller, command))?
    }

    fn plan_restore(
        &self,
        caller: &str,
        command: RestorePlanCommand,
    ) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.plan_restore(caller, command))?
    }

    fn execute_restore(
        &self,
        caller: &str,
        command: RestoreExecuteCommand,
    ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.execute_restore(caller, command))?
    }

    fn execute_adapter_restore(
        &self,
        caller: &str,
        command: RestoreAdapterExecuteCommand,
    ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.execute_adapter_restore(caller, command))?
    }

    fn recover_restore(
        &self,
        caller: &str,
        command: RestoreRecoveryCommand,
    ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
        self.with_authority(|authority| authority.recover_restore(caller, command))?
    }
}
