use std::error::Error;

use longhorn_config::{
    BackupCaptureReceiptProjection, BackupCreateCommand, BackupCreateOutcome, BackupExportCommand,
    BackupExportOutcome, BackupInventoryEntry, BackupInventoryEntryState,
    BackupPublicationReceiptProjection, BackupRetentionApplyCommand, BackupRetentionApplyOutcome,
    ConfigGeneration, ConfigOperationRejection, ConfigOperationRejectionCode,
    ConfigOperationsSnapshot, ConfigProtocolVersion, ConfigSnapshotCommand, PendingBackupPolicy,
    RestoreAdapterExecuteCommand, RestoreAdapterExecuteOutcome,
    RestoreAdapterParticipationProjection, RestoreAdapterReceiptProjection,
    RestoreAdapterRequirementProjection, RestoreArchiveSelection, RestoreConflictChoiceProjection,
    RestoreDomainChoice, RestoreExecuteCommand, RestoreExecuteOutcome, RestoreInspectCommand,
    RestoreInspectOutcome, RestorePlanCommand, RestorePlanOutcome, RestoreRecoveryCommand,
    RestoreRecoveryOutcomeProjection, RestoreRecoveryReceiptProjection, StorageCleanupCommand,
    StorageCleanupOutcome, StorageCleanupReceiptProjection, StorageProfileId,
    StorageRecoveryCommand, StorageRecoveryOutcome, StorageRecoveryReceiptProjection,
    StorageTransitionConflictProjection, StorageTransitionDomainProjection,
    StorageTransitionExecuteCommand, StorageTransitionExecuteOutcome,
    StorageTransitionInspectCommand, StorageTransitionInspectOutcome,
    StorageTransitionPreviewProjection, StorageTransitionReceiptProjection,
};
use longhorn_core::ConfigRequestId;
use serde::Serialize;
use serde_json::{Value, json};

mod restore;
mod snapshot;

use restore::{
    restore_execution_receipt, restore_execution_states, restore_inspection,
    restore_inspection_states, restore_plan, restore_plan_states,
};
use snapshot::snapshot;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenFixture {
    protocol_version: u16,
    snapshot: ConfigOperationsSnapshot,
    commands: Commands,
    outcomes: Outcomes,
    inventory_states: Vec<BackupInventoryEntry>,
    restore_inspection_states: Vec<RestoreInspectOutcome>,
    restore_plan_states: Vec<RestorePlanOutcome>,
    restore_execution_states: Vec<RestoreExecuteOutcome>,
    incompatibility: Incompatibility,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Incompatibility {
    future_protocol_version: u16,
    unknown_capability: Value,
    unknown_inventory_state: Value,
    unknown_bootstrap_state: Value,
    unknown_outcome_status: Value,
    unknown_restore_compatibility: Value,
}

pub fn render() -> Result<String, Box<dyn Error>> {
    let base_snapshot = snapshot();
    let operational_publication = publication("operational");
    let transition_receipt = StorageTransitionReceiptProjection {
        transition_id: "transition:fixture".into(),
        outcome: "committed".into(),
        target_layout_digest: digest('b'),
        copied_domain_ids: vec!["app.preferences".into()],
        custom_domain_ids: vec!["app.database".into()],
        retained_source_paths: vec!["/old/config/preferences.json".into()],
        receipt_digest: digest('c'),
    };
    let fixture = GoldenFixture {
        protocol_version: 1,
        snapshot: base_snapshot.clone(),
        commands: Commands {
            snapshot: ConfigSnapshotCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:snapshot"),
            },
            inspect_transition: StorageTransitionInspectCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:inspect"),
                target_profile: StorageProfileId::UnifiedAppRootV1,
                include_logs: true,
            },
            execute_transition: StorageTransitionExecuteCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:execute"),
                generation: ConfigGeneration::new(7),
                confirmation_digest: digest('d'),
            },
            recover_storage: StorageRecoveryCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:recover"),
            },
            cleanup_storage: StorageCleanupCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:cleanup"),
                transition_id: "transition:fixture".into(),
                transition_receipt_digest: digest('c'),
            },
            create_backup: BackupCreateCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:create-backup"),
                pending_policy: PendingBackupPolicy::Flush,
            },
            export_backup: BackupExportCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:export-backup"),
                archive_sha256: digest('e'),
            },
            apply_retention: BackupRetentionApplyCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:retention"),
                generation: ConfigGeneration::new(7),
                confirmation_digest: digest('f'),
            },
            inspect_restore: RestoreInspectCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:inspect-restore"),
                selection: RestoreArchiveSelection::HostPicker,
            },
            plan_restore: RestorePlanCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:plan-restore"),
                generation: ConfigGeneration::new(7),
                archive_sha256: digest('e'),
                choices: vec![
                    RestoreDomainChoice {
                        domain_id: "app.preferences".into(),
                        choice: RestoreConflictChoiceProjection::UseArchive,
                    },
                    RestoreDomainChoice {
                        domain_id: "app.database".into(),
                        choice: RestoreConflictChoiceProjection::KeepCurrent,
                    },
                ],
            },
            execute_restore: RestoreExecuteCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:execute-restore"),
                generation: ConfigGeneration::new(8),
                confirmation_digest: digest('8'),
            },
            execute_adapter_restore: RestoreAdapterExecuteCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:adapter-restore"),
                generation: ConfigGeneration::new(8),
                archive_sha256: digest('e'),
                domain_id: "app.database".into(),
                confirmation_digest: digest('7'),
                requirement: RestoreAdapterRequirementProjection::AllowSeparate,
            },
            recover_restore: RestoreRecoveryCommand {
                protocol_version: ConfigProtocolVersion::CURRENT,
                request_id: request_id("request:recover-restore"),
            },
        },
        outcomes: Outcomes {
            inspect_transition: StorageTransitionInspectOutcome::Ready {
                generation: ConfigGeneration::new(7),
                preview: transition_preview(),
            },
            execute_transition: StorageTransitionExecuteOutcome::Committed {
                receipt: transition_receipt,
                snapshot: Box::new(base_snapshot.clone()),
            },
            recover_storage: StorageRecoveryOutcome::Recovered {
                receipt: StorageRecoveryReceiptProjection {
                    transition_id: Some("transition:fixture".into()),
                    outcome: "rolledForward".into(),
                    active_layout_digest: digest('b'),
                    detail: "locator and journal agree".into(),
                },
                snapshot: Box::new(base_snapshot.clone()),
            },
            cleanup_storage: StorageCleanupOutcome::Applied {
                receipt: StorageCleanupReceiptProjection {
                    transition_id: "transition:fixture".into(),
                    transition_receipt_digest: digest('c'),
                    removed_paths: vec!["/old/config/preferences.json".into()],
                },
                snapshot: Box::new(base_snapshot.clone()),
            },
            create_backup: BackupCreateOutcome::Published {
                capture: BackupCaptureReceiptProjection {
                    selected_domains: 3,
                    captured_domains: 2,
                    absent_domains: 0,
                    source_preserved_domains: 0,
                    excluded_domains: 1,
                    custom_domains: 1,
                    external_consistency_groups: 1,
                    total_payload_bytes: 4096,
                    flushed_pending_publication: true,
                },
                publication: operational_publication,
                snapshot: Box::new(base_snapshot.clone()),
            },
            export_backup: BackupExportOutcome::Published {
                publication: publication("userExport"),
                snapshot: Box::new(base_snapshot.clone()),
            },
            apply_retention: BackupRetentionApplyOutcome::Applied {
                deleted_paths: vec!["/backups/old.longhorn-backup".into()],
                snapshot: Box::new(base_snapshot),
            },
            inspect_restore: RestoreInspectOutcome::Ready {
                generation: ConfigGeneration::new(8),
                inspection: Box::new(restore_inspection()),
            },
            plan_restore: RestorePlanOutcome::Ready {
                generation: ConfigGeneration::new(8),
                plan: restore_plan(),
            },
            execute_restore: RestoreExecuteOutcome::Succeeded {
                receipt: Box::new(restore_execution_receipt()),
                snapshot: Box::new(snapshot()),
            },
            execute_adapter_restore: RestoreAdapterExecuteOutcome::Completed {
                receipt: RestoreAdapterReceiptProjection {
                    domain_id: "app.database".into(),
                    adapter: "sqlite-v1".into(),
                    participation: RestoreAdapterParticipationProjection::Separate,
                    confirmation_digest: digest('7'),
                    outcome: "verified".into(),
                    evidence: Some(digest('6')),
                },
                snapshot: Box::new(snapshot()),
            },
            recover_restore: RestoreRecoveryOutcomeProjection::Recovered {
                receipt: RestoreRecoveryReceiptProjection {
                    outcome: "rolledBack".into(),
                    domain_ids: vec!["app.preferences".into()],
                },
                snapshot: Box::new(snapshot()),
            },
        },
        inventory_states: inventory_states(),
        restore_inspection_states: restore_inspection_states(),
        restore_plan_states: restore_plan_states(),
        restore_execution_states: restore_execution_states(),
        incompatibility: Incompatibility {
            future_protocol_version: 2,
            unknown_capability: json!("restoreMerge"),
            unknown_inventory_state: json!("decrypted"),
            unknown_bootstrap_state: json!({"state": "guessed"}),
            unknown_outcome_status: json!({"status": "scheduled"}),
            unknown_restore_compatibility: json!({"status": "mergeAutomatically"}),
        },
    };
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

fn transition_preview() -> StorageTransitionPreviewProjection {
    StorageTransitionPreviewProjection {
        source_layout_digest: digest('a'),
        target_layout_digest: digest('b'),
        target_profile: StorageProfileId::UnifiedAppRootV1,
        domains: vec![StorageTransitionDomainProjection {
            domain_id: "app.preferences".into(),
            storage_class: "userConfig".into(),
            action: "copy".into(),
            source_path: Some("/old/config/preferences.json".into()),
            target_path: Some("/new/config/preferences.json".into()),
            source_sha256: Some(digest('1')),
        }],
        unknown_source_paths: vec!["/old/config/unregistered.json".into()],
        conflicts: vec![StorageTransitionConflictProjection {
            kind: "targetExists".into(),
            path: Some("/new/config/preferences.json".into()),
            detail: "target contains different bytes".into(),
        }],
        evidence_digest: digest('2'),
        confirmation_digest: digest('d'),
    }
}

fn publication(destination: &str) -> BackupPublicationReceiptProjection {
    BackupPublicationReceiptProjection {
        path: format!("/backups/{destination}.longhorn-backup"),
        destination: destination.into(),
        archive_sha256: digest('e'),
        durability: "synced".into(),
        replaced_existing: false,
    }
}

fn inventory_states() -> Vec<BackupInventoryEntry> {
    [
        (BackupInventoryEntryState::Locked, "locked"),
        (BackupInventoryEntryState::Corrupt, "corrupt"),
        (BackupInventoryEntryState::Foreign, "foreignApplication"),
        (BackupInventoryEntryState::Unknown, "unknownFormat"),
        (BackupInventoryEntryState::Unreadable, "unreadable"),
        (BackupInventoryEntryState::Unmanaged, "unmanaged"),
        (BackupInventoryEntryState::Valid, "valid"),
    ]
    .into_iter()
    .map(|(state, kind)| BackupInventoryEntry {
        path: Some(format!("/backups/{kind}.archive")),
        state,
        diagnostic_kind: kind.into(),
        detail: format!("{kind} fixture"),
    })
    .collect()
}

fn rejection(code: ConfigOperationRejectionCode) -> ConfigOperationRejection {
    ConfigOperationRejection {
        code,
        detail: "fixture refusal".into(),
        snapshot: None,
    }
}

fn digest(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn request_id(value: &str) -> ConfigRequestId {
    ConfigRequestId::new(value).expect("fixture request id must be valid")
}
