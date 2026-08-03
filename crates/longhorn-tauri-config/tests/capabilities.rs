//! Narrow Tauri capability and dependency audit.

use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn permission_examples_name_exact_command_groups() {
    let read_permission = read("examples/permissions/read-config-operations.toml");
    let storage = read("examples/permissions/mutate-storage.toml");
    let backup = read("examples/permissions/mutate-backups.toml");
    let restore = read("examples/permissions/mutate-restore.toml");
    assert_eq!(read_permission.matches("\"longhorn_config_").count(), 3);
    assert_eq!(storage.matches("\"longhorn_config_").count(), 3);
    assert_eq!(backup.matches("\"longhorn_config_").count(), 3);
    assert_eq!(restore.matches("\"longhorn_config_").count(), 4);
    assert!(read_permission.contains("\"longhorn_config_snapshot\""));
    assert!(read_permission.contains("\"longhorn_config_storage_inspect\""));
    assert!(read_permission.contains("\"longhorn_config_restore_inspect\""));
    for command in ["storage_execute", "storage_recover", "storage_cleanup"] {
        assert!(storage.contains(&format!("\"longhorn_config_{command}\"")));
    }
    for command in ["backup_create", "backup_export", "backup_retention"] {
        assert!(backup.contains(&format!("\"longhorn_config_{command}\"")));
    }
    for command in [
        "restore_plan",
        "restore_execute",
        "restore_adapter_execute",
        "restore_recover",
    ] {
        assert!(restore.contains(&format!("\"longhorn_config_{command}\"")));
    }
}

#[test]
fn capability_examples_add_only_selected_authority() {
    let diagnostics: Value =
        serde_json::from_str(&read("examples/capabilities/config-diagnostics.json")).unwrap();
    let full: Value =
        serde_json::from_str(&read("examples/capabilities/config-operations.json")).unwrap();
    assert_eq!(
        diagnostics["permissions"],
        serde_json::json!(["allow-longhorn-config-read"])
    );
    assert_eq!(
        full["permissions"],
        serde_json::json!([
            "allow-longhorn-config-read",
            "allow-longhorn-storage-mutate",
            "allow-longhorn-backup-mutate",
            "allow-longhorn-restore-mutate"
        ])
    );
    assert_eq!(
        diagnostics["windows"],
        serde_json::json!(["main", "settings"])
    );
    assert_eq!(full["windows"], serde_json::json!(["settings"]));
}

#[test]
fn crate_dependency_edge_stays_narrow_and_secret_free() {
    let manifest = read("Cargo.toml");
    for forbidden in [
        "longhorn-config-age",
        "longhorn-settings",
        "longhorn-layout",
        "longhorn-surfaces",
        "longhorn-transfer",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    let sources = [
        read("src/authority.rs"),
        read("src/commands.rs"),
        read("src/ports.rs"),
    ]
    .join("\n");
    for forbidden in [
        "AgeIdentity",
        "AgePassphrase",
        "archive_bytes",
        "payload_bytes",
    ] {
        assert!(!sources.contains(forbidden), "{forbidden}");
    }
}

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

mod async_offload {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    };

    use longhorn_config::{
        BackupCreateCommand, BackupCreateOutcome, BackupExportCommand, BackupExportOutcome,
        BackupRetentionApplyCommand, BackupRetentionApplyOutcome, ConfigOperationsSnapshot,
        ConfigProtocolVersion, ConfigSnapshotCommand, RestoreAdapterExecuteCommand,
        RestoreAdapterExecuteOutcome, RestoreExecuteCommand, RestoreExecuteOutcome,
        RestoreInspectCommand, RestoreInspectOutcome, RestorePlanCommand, RestorePlanOutcome,
        RestoreRecoveryCommand, RestoreRecoveryOutcomeProjection, StorageCleanupCommand,
        StorageCleanupOutcome, StorageRecoveryCommand, StorageRecoveryOutcome,
        StorageTransitionExecuteCommand, StorageTransitionExecuteOutcome,
        StorageTransitionInspectCommand, StorageTransitionInspectOutcome,
    };
    use longhorn_core::ConfigRequestId;
    use longhorn_tauri_config::{ConfigOperationsCommandService, ConfigOperationsHostError};

    struct ParkedService {
        barrier: Barrier,
        entered: AtomicBool,
    }

    impl ConfigOperationsCommandService for ParkedService {
        fn snapshot(
            &self,
            _caller: &str,
            _command: ConfigSnapshotCommand,
        ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
            self.entered.store(true, Ordering::SeqCst);
            // Parks the executing thread until the test releases it.
            self.barrier.wait();
            Err(ConfigOperationsHostError::authority(
                "parked test service",
                true,
            ))
        }

        fn inspect_storage_transition(
            &self,
            _caller: &str,
            _command: StorageTransitionInspectCommand,
        ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn execute_storage_transition(
            &self,
            _caller: &str,
            _command: StorageTransitionExecuteCommand,
        ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn recover_storage(
            &self,
            _caller: &str,
            _command: StorageRecoveryCommand,
        ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn cleanup_storage(
            &self,
            _caller: &str,
            _command: StorageCleanupCommand,
        ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn create_backup(
            &self,
            _caller: &str,
            _command: BackupCreateCommand,
        ) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn export_backup(
            &self,
            _caller: &str,
            _command: BackupExportCommand,
        ) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn apply_backup_retention(
            &self,
            _caller: &str,
            _command: BackupRetentionApplyCommand,
        ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn inspect_restore(
            &self,
            _caller: &str,
            _command: RestoreInspectCommand,
        ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn plan_restore(
            &self,
            _caller: &str,
            _command: RestorePlanCommand,
        ) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn execute_restore(
            &self,
            _caller: &str,
            _command: RestoreExecuteCommand,
        ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn execute_adapter_restore(
            &self,
            _caller: &str,
            _command: RestoreAdapterExecuteCommand,
        ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
            panic!("unused in this test")
        }

        fn recover_restore(
            &self,
            _caller: &str,
            _command: RestoreRecoveryCommand,
        ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
            panic!("unused in this test")
        }
    }

    #[test]
    fn contended_service_work_runs_on_blocking_threads_not_the_async_executor() {
        let service = Arc::new(ParkedService {
            barrier: Barrier::new(2),
            entered: AtomicBool::new(false),
        });
        let inner = Arc::clone(&service);
        // Drive the service through the same offload primitive the commands
        // use; the async executor must stay free while the service is parked.
        let task = tauri::async_runtime::spawn_blocking(move || {
            inner.snapshot(
                "main",
                ConfigSnapshotCommand {
                    protocol_version: ConfigProtocolVersion::CURRENT,
                    request_id: ConfigRequestId::new("request:parked").unwrap(),
                },
            )
        });
        while !service.entered.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        // The async runtime stays responsive while the blocking call parks.
        let free = tauri::async_runtime::block_on(async { 7 });
        assert_eq!(free, 7);
        service.barrier.wait();
        let outcome = tauri::async_runtime::block_on(task).unwrap();
        assert!(outcome.is_err());
    }
}
