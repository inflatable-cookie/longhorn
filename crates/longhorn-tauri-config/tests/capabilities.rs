//! Tauri command registration, capability examples, and async offload tests.

use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

use serde_json::Value;
use tauri::{WebviewUrl, WebviewWindowBuilder};

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
fn permission_examples_dispatch_through_registered_tauri_commands() {
    let service: Arc<dyn longhorn_tauri_config::ConfigOperationsCommandService> =
        Arc::new(async_offload::StubService);
    let app = tauri::test::mock_builder()
        .manage(longhorn_tauri_config::TauriConfigOperationsState::new(
            service,
        ))
        .invoke_handler(tauri::generate_handler![
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
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::default())
        .build()
        .unwrap();

    let permission_files = [
        "examples/permissions/read-config-operations.toml",
        "examples/permissions/mutate-storage.toml",
        "examples/permissions/mutate-backups.toml",
        "examples/permissions/mutate-restore.toml",
    ];

    let mut seen = BTreeSet::new();
    let mut callback_id = 0_u32;
    for file in permission_files {
        let allowed = parse_allowed_commands(&read(file));
        assert!(!allowed.is_empty(), "expected commands in {file}");
        for command in allowed {
            assert!(
                seen.insert(command.clone()),
                "duplicate permission entry for {command}"
            );
            let cb = callback_id;
            callback_id += 2;
            // Dispatch each allowed command name through the live mock invoke
            // handler. An unregistered command fails with Tauri's
            // "command <name> not found"; a registered command reaches its
            // generated argument deserializer and rejects the empty payload.
            let err = tauri::test::get_ipc_response(
                &webview,
                tauri::webview::InvokeRequest {
                    cmd: command.clone(),
                    callback: tauri::ipc::CallbackFn(cb),
                    error: tauri::ipc::CallbackFn(cb + 1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(serde_json::json!({})),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .expect_err("empty payload should fail command argument deserialization");
            let message = err.to_string();
            assert!(
                !message.contains("not found"),
                "permission-allowed command {command} is not registered in invoke_handler: {message}"
            );
            assert!(
                message.contains("command"),
                "expected registered command argument error for {command}, got: {message}"
            );
        }
    }

    // Positive control: an unregistered command must be rejected as not found.
    let unknown = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "longhorn_config_unregistered".into(),
            callback: tauri::ipc::CallbackFn(callback_id),
            error: tauri::ipc::CallbackFn(callback_id + 1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(serde_json::json!({})),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .expect_err("unregistered command must fail");
    assert!(
        unknown.to_string().contains("not found"),
        "expected not found for unregistered command, got: {unknown}"
    );
    assert_eq!(seen.len(), 13);
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

    pub(super) struct StubService;

    impl ConfigOperationsCommandService for StubService {
        fn snapshot(
            &self,
            _caller: &str,
            _command: ConfigSnapshotCommand,
        ) -> Result<ConfigOperationsSnapshot, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn inspect_storage_transition(
            &self,
            _caller: &str,
            _command: StorageTransitionInspectCommand,
        ) -> Result<StorageTransitionInspectOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn execute_storage_transition(
            &self,
            _caller: &str,
            _command: StorageTransitionExecuteCommand,
        ) -> Result<StorageTransitionExecuteOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn recover_storage(
            &self,
            _caller: &str,
            _command: StorageRecoveryCommand,
        ) -> Result<StorageRecoveryOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn cleanup_storage(
            &self,
            _caller: &str,
            _command: StorageCleanupCommand,
        ) -> Result<StorageCleanupOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn create_backup(
            &self,
            _caller: &str,
            _command: BackupCreateCommand,
        ) -> Result<BackupCreateOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn export_backup(
            &self,
            _caller: &str,
            _command: BackupExportCommand,
        ) -> Result<BackupExportOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn apply_backup_retention(
            &self,
            _caller: &str,
            _command: BackupRetentionApplyCommand,
        ) -> Result<BackupRetentionApplyOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn inspect_restore(
            &self,
            _caller: &str,
            _command: RestoreInspectCommand,
        ) -> Result<RestoreInspectOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn plan_restore(
            &self,
            _caller: &str,
            _command: RestorePlanCommand,
        ) -> Result<RestorePlanOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn execute_restore(
            &self,
            _caller: &str,
            _command: RestoreExecuteCommand,
        ) -> Result<RestoreExecuteOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn execute_adapter_restore(
            &self,
            _caller: &str,
            _command: RestoreAdapterExecuteCommand,
        ) -> Result<RestoreAdapterExecuteOutcome, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }

        fn recover_restore(
            &self,
            _caller: &str,
            _command: RestoreRecoveryCommand,
        ) -> Result<RestoreRecoveryOutcomeProjection, ConfigOperationsHostError> {
            Err(ConfigOperationsHostError::authority("stub", false))
        }
    }

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
