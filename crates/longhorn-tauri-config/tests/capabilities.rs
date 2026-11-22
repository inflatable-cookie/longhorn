//! Tauri command registration, capability examples, and async offload tests.

use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

/// Every `#[tauri::command]` exported by `longhorn-tauri-config`.
///
/// The mock handler below registers these exact functions with
/// `tauri::generate_handler!`, so a renamed or removed command fails to
/// compile here.
const REGISTERED_COMMANDS: [&str; 13] = [
    "longhorn_config_snapshot",
    "longhorn_config_storage_inspect",
    "longhorn_config_storage_execute",
    "longhorn_config_storage_recover",
    "longhorn_config_storage_cleanup",
    "longhorn_config_backup_create",
    "longhorn_config_backup_export",
    "longhorn_config_backup_retention",
    "longhorn_config_restore_inspect",
    "longhorn_config_restore_plan",
    "longhorn_config_restore_execute",
    "longhorn_config_restore_adapter_execute",
    "longhorn_config_restore_recover",
];

fn parse_allowed_commands(toml_text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_allow = false;
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("commands.allow") {
            in_allow = true;
        }
        if in_allow {
            let mut rest = trimmed;
            while let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                let Some(end) = after.find('"') else {
                    break;
                };
                commands.push(after[..end].to_owned());
                rest = &after[end + 1..];
            }
            if trimmed.contains(']') {
                in_allow = false;
            }
        }
    }
    commands
}

#[test]
fn permission_examples_partition_every_registered_command_once() {
    // Compile-check that every command in REGISTERED_COMMANDS is a real
    // `#[tauri::command]` accepted by Tauri's invoke handler.
    let _builder = tauri::test::mock_builder().invoke_handler(tauri::generate_handler![
        longhorn_tauri_config::longhorn_config_snapshot,
        longhorn_tauri_config::longhorn_config_storage_inspect,
        longhorn_tauri_config::longhorn_config_storage_execute,
        longhorn_tauri_config::longhorn_config_storage_recover,
        longhorn_tauri_config::longhorn_config_storage_cleanup,
        longhorn_tauri_config::longhorn_config_backup_create,
        longhorn_tauri_config::longhorn_config_backup_export,
        longhorn_tauri_config::longhorn_config_backup_retention,
        longhorn_tauri_config::longhorn_config_restore_inspect,
        longhorn_tauri_config::longhorn_config_restore_plan,
        longhorn_tauri_config::longhorn_config_restore_execute,
        longhorn_tauri_config::longhorn_config_restore_adapter_execute,
        longhorn_tauri_config::longhorn_config_restore_recover,
    ]);

    let expected_groups: &[(&str, &[&str])] = &[
        (
            "examples/permissions/read-config-operations.toml",
            &[
                "longhorn_config_snapshot",
                "longhorn_config_storage_inspect",
                "longhorn_config_restore_inspect",
            ],
        ),
        (
            "examples/permissions/mutate-storage.toml",
            &[
                "longhorn_config_storage_execute",
                "longhorn_config_storage_recover",
                "longhorn_config_storage_cleanup",
            ],
        ),
        (
            "examples/permissions/mutate-backups.toml",
            &[
                "longhorn_config_backup_create",
                "longhorn_config_backup_export",
                "longhorn_config_backup_retention",
            ],
        ),
        (
            "examples/permissions/mutate-restore.toml",
            &[
                "longhorn_config_restore_plan",
                "longhorn_config_restore_execute",
                "longhorn_config_restore_adapter_execute",
                "longhorn_config_restore_recover",
            ],
        ),
    ];

    let mut seen = BTreeSet::new();
    for (file, expected) in expected_groups {
        let allowed = parse_allowed_commands(&read(file));
        let expected_vec: Vec<String> = expected.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(allowed, expected_vec, "mismatch in {file}");
        for command in allowed {
            assert!(seen.insert(command.clone()), "duplicate command {command}");
        }
    }

    let registered: BTreeSet<String> = REGISTERED_COMMANDS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(seen, registered);
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
