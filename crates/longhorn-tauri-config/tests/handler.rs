//! Direct and serialized injected-handler conformance.

use std::sync::{Arc, Mutex};

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
use longhorn_core::ConfigRequestId;
use longhorn_tauri_config::{
    ConfigOperationsAuthority, ConfigOperationsCommandService, ConfigOperationsHandlerAssembly,
    ConfigOperationsHostError, ConfigOperationsHostErrorCode,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    snapshot: ConfigOperationsSnapshot,
    commands: Commands,
    outcomes: Outcomes,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commands {
    snapshot: ConfigSnapshotCommand,
    inspect_transition: StorageTransitionInspectCommand,
    execute_transition: StorageTransitionExecuteCommand,
    recover_storage: StorageRecoveryCommand,
    cleanup_storage: StorageCleanupCommand,
    create_backup: BackupCreateCommand,
    export_backup: BackupExportCommand,
    apply_retention: BackupRetentionApplyCommand,
    inspect_restore: RestoreInspectCommand,
    plan_restore: RestorePlanCommand,
    execute_restore: RestoreExecuteCommand,
    execute_adapter_restore: RestoreAdapterExecuteCommand,
    recover_restore: RestoreRecoveryCommand,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Outcomes {
    inspect_transition: StorageTransitionInspectOutcome,
    execute_transition: StorageTransitionExecuteOutcome,
    recover_storage: StorageRecoveryOutcome,
    cleanup_storage: StorageCleanupOutcome,
    create_backup: BackupCreateOutcome,
    export_backup: BackupExportOutcome,
    apply_retention: BackupRetentionApplyOutcome,
    inspect_restore: RestoreInspectOutcome,
    plan_restore: RestorePlanOutcome,
    execute_restore: RestoreExecuteOutcome,
    execute_adapter_restore: RestoreAdapterExecuteOutcome,
    recover_restore: RestoreRecoveryOutcomeProjection,
}

struct FixtureAuthority {
    fixture: Fixture,
    calls: Arc<Mutex<Vec<String>>>,
}

impl ConfigOperationsAuthority for FixtureAuthority {
    fn snapshot(
        &mut self,
        caller: &str,
        _command: ConfigSnapshotCommand,
    ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
        self.record("snapshot", caller)?;
        Ok(self.fixture.snapshot.clone())
    }

    fn inspect_storage_transition(
        &mut self,
        caller: &str,
        _command: StorageTransitionInspectCommand,
    ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
        self.record("storage-inspect", caller)?;
        Ok(self.fixture.outcomes.inspect_transition.clone())
    }

    fn execute_storage_transition(
        &mut self,
        caller: &str,
        _command: StorageTransitionExecuteCommand,
    ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
        self.record("storage-execute", caller)?;
        Ok(self.fixture.outcomes.execute_transition.clone())
    }

    fn recover_storage(
        &mut self,
        caller: &str,
        _command: StorageRecoveryCommand,
    ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
        self.record("storage-recover", caller)?;
        Ok(self.fixture.outcomes.recover_storage.clone())
    }

    fn cleanup_storage(
        &mut self,
        caller: &str,
        _command: StorageCleanupCommand,
    ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
        self.record("storage-cleanup", caller)?;
        Ok(self.fixture.outcomes.cleanup_storage.clone())
    }

    fn create_backup(
        &mut self,
        caller: &str,
        _command: BackupCreateCommand,
    ) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
        self.record("backup-create", caller)?;
        Ok(self.fixture.outcomes.create_backup.clone())
    }

    fn export_backup(
        &mut self,
        caller: &str,
        _command: BackupExportCommand,
    ) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
        self.record("backup-export", caller)?;
        Ok(self.fixture.outcomes.export_backup.clone())
    }

    fn apply_backup_retention(
        &mut self,
        caller: &str,
        _command: BackupRetentionApplyCommand,
    ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
        self.record("backup-retention", caller)?;
        Ok(self.fixture.outcomes.apply_retention.clone())
    }

    fn inspect_restore(
        &mut self,
        caller: &str,
        _command: RestoreInspectCommand,
    ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
        self.record("restore-inspect", caller)?;
        Ok(self.fixture.outcomes.inspect_restore.clone())
    }

    fn plan_restore(
        &mut self,
        caller: &str,
        _command: RestorePlanCommand,
    ) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
        self.record("restore-plan", caller)?;
        Ok(self.fixture.outcomes.plan_restore.clone())
    }

    fn execute_restore(
        &mut self,
        caller: &str,
        _command: RestoreExecuteCommand,
    ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
        self.record("restore-execute", caller)?;
        Ok(self.fixture.outcomes.execute_restore.clone())
    }

    fn execute_adapter_restore(
        &mut self,
        caller: &str,
        _command: RestoreAdapterExecuteCommand,
    ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
        self.record("restore-adapter", caller)?;
        Ok(self.fixture.outcomes.execute_adapter_restore.clone())
    }

    fn recover_restore(
        &mut self,
        caller: &str,
        _command: RestoreRecoveryCommand,
    ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
        self.record("restore-recover", caller)?;
        Ok(self.fixture.outcomes.recover_restore.clone())
    }
}

impl FixtureAuthority {
    fn record(&self, operation: &str, caller: &str) -> Result<(), ConfigOperationsHostError> {
        if caller != "settings" {
            return Err(ConfigOperationsHostError::authority(
                "caller is not authorized",
                false,
            ));
        }
        self.calls
            .lock()
            .unwrap()
            .push(format!("{operation}:{caller}"));
        Ok(())
    }
}

#[test]
fn direct_and_serialized_commands_use_one_injected_assembly() {
    let fixture = fixture();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = ConfigOperationsHandlerAssembly::new(FixtureAuthority {
        fixture: fixture.clone(),
        calls: calls.clone(),
    });

    assert_eq!(
        service
            .snapshot("settings", serialized(&fixture.commands.snapshot))
            .unwrap(),
        fixture.snapshot
    );
    assert_eq!(
        service
            .inspect_storage_transition(
                "settings",
                serialized(&fixture.commands.inspect_transition),
            )
            .unwrap(),
        fixture.outcomes.inspect_transition
    );
    assert_eq!(
        service
            .execute_storage_transition(
                "settings",
                serialized(&fixture.commands.execute_transition),
            )
            .unwrap(),
        fixture.outcomes.execute_transition
    );
    assert_eq!(
        service
            .recover_storage("settings", serialized(&fixture.commands.recover_storage))
            .unwrap(),
        fixture.outcomes.recover_storage
    );
    assert_eq!(
        service
            .cleanup_storage("settings", serialized(&fixture.commands.cleanup_storage))
            .unwrap(),
        fixture.outcomes.cleanup_storage
    );
    assert_eq!(
        service
            .create_backup("settings", serialized(&fixture.commands.create_backup))
            .unwrap(),
        fixture.outcomes.create_backup
    );
    assert_eq!(
        service
            .export_backup("settings", serialized(&fixture.commands.export_backup))
            .unwrap(),
        fixture.outcomes.export_backup
    );
    assert_eq!(
        service
            .apply_backup_retention("settings", serialized(&fixture.commands.apply_retention),)
            .unwrap(),
        fixture.outcomes.apply_retention
    );
    assert_eq!(
        service
            .inspect_restore("settings", serialized(&fixture.commands.inspect_restore))
            .unwrap(),
        fixture.outcomes.inspect_restore
    );
    assert_eq!(
        service
            .plan_restore("settings", serialized(&fixture.commands.plan_restore))
            .unwrap(),
        fixture.outcomes.plan_restore
    );
    assert_eq!(
        service
            .execute_restore("settings", serialized(&fixture.commands.execute_restore))
            .unwrap(),
        fixture.outcomes.execute_restore
    );
    assert_eq!(
        service
            .execute_adapter_restore(
                "settings",
                serialized(&fixture.commands.execute_adapter_restore),
            )
            .unwrap(),
        fixture.outcomes.execute_adapter_restore
    );
    assert_eq!(
        service
            .recover_restore("settings", serialized(&fixture.commands.recover_restore))
            .unwrap(),
        fixture.outcomes.recover_restore
    );
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "snapshot:settings",
            "storage-inspect:settings",
            "storage-execute:settings",
            "storage-recover:settings",
            "storage-cleanup:settings",
            "backup-create:settings",
            "backup-export:settings",
            "backup-retention:settings",
            "restore-inspect:settings",
            "restore-plan:settings",
            "restore-execute:settings",
            "restore-adapter:settings",
            "restore-recover:settings",
        ]
    );
}

#[test]
fn caller_authorization_is_not_delegated_to_tauri_capabilities() {
    let fixture = fixture();
    let service = ConfigOperationsHandlerAssembly::new(FixtureAuthority {
        fixture,
        calls: Arc::new(Mutex::new(Vec::new())),
    });
    let command = ConfigSnapshotCommand {
        protocol_version: longhorn_config::ConfigProtocolVersion::CURRENT,
        request_id: ConfigRequestId::new("request:unauthorized").unwrap(),
    };
    let error = service.snapshot("main", command).unwrap_err();
    assert_eq!(
        error.code,
        ConfigOperationsHostErrorCode::AuthorityUnavailable
    );
    assert!(!error.retryable);
}

#[test]
fn poisoned_handler_state_is_a_typed_retryable_failure() {
    let fixture = fixture();
    let service = Arc::new(ConfigOperationsHandlerAssembly::new(FixtureAuthority {
        fixture,
        calls: Arc::new(Mutex::new(Vec::new())),
    }));
    let poison = service.clone();
    assert!(
        std::thread::spawn(move || poison.with_authority::<()>(|_| panic!("poison")))
            .join()
            .is_err()
    );
    let command = ConfigSnapshotCommand {
        protocol_version: longhorn_config::ConfigProtocolVersion::CURRENT,
        request_id: ConfigRequestId::new("request:poisoned").unwrap(),
    };
    let error = service.snapshot("settings", command).unwrap_err();
    assert_eq!(error.code, ConfigOperationsHostErrorCode::StateUnavailable);
    assert!(error.retryable);
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../../fixtures/config/protocol-v1.json")).unwrap()
}

fn serialized<Value>(value: &Value) -> Value
where
    Value: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}
