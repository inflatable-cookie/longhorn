use longhorn_config::{
    BackupConsistencyMode, BackupKind, BackupPublicationReceiptProjection, BackupSourceState,
    ConfigGeneration, ConfigOperationRejectionCode, RestoreAdapterParticipationProjection,
    RestoreAuthenticityProjection, RestoreConflictChoiceProjection,
    RestoreConsistencyGroupProjection, RestoreCurrentEvidenceProjection,
    RestoreDomainCompatibilityProjection, RestoreDomainInspectionProjection,
    RestoreExclusionProjection, RestoreExecuteOutcome, RestoreExecutionFailureProjection,
    RestoreExecutionReceiptProjection, RestoreExecutionStageProjection,
    RestoreFailureTerminalProjection, RestoreIdentityProjection, RestoreIdentityStatusProjection,
    RestoreInspectOutcome, RestoreInspectionProjection, RestoreInspectionReceiptProjection,
    RestoreIntegrityProjection, RestoreOperationStateProjection, RestoreOperationsProjection,
    RestorePlanEntryProjection, RestorePlanOutcome, RestorePlanProjection,
    RestorePlanReceiptProjection, RestoreStagingReceiptProjection, StorageClass,
};

use super::{digest, rejection, snapshot};

pub(super) fn restore_inspection() -> RestoreInspectionProjection {
    RestoreInspectionProjection {
        archive_sha256: digest('e'),
        archive_id: "backup:fixture".into(),
        created_at: "2026-07-29T12:00:00Z".into(),
        kind: BackupKind::Operational,
        application_version: "1.2.3".into(),
        producer_version: "0.1.0".into(),
        integrity: RestoreIntegrityProjection::Verified,
        authenticity: RestoreAuthenticityProjection::Authenticated,
        identity: RestoreIdentityProjection {
            application: RestoreIdentityStatusProjection::Compatible,
            producer: RestoreIdentityStatusProjection::Compatible,
        },
        consistency_groups: vec![
            RestoreConsistencyGroupProjection {
                id: "longhorn-config-store".into(),
                mode: BackupConsistencyMode::CoordinatedBounded,
                authority: "longhorn-config-store-coordinator".into(),
            },
            RestoreConsistencyGroupProjection {
                id: "sqlite-main".into(),
                mode: BackupConsistencyMode::ExternalSnapshot,
                authority: "sqlite-online-backup".into(),
            },
        ],
        domains: vec![
            RestoreDomainInspectionProjection {
                domain_id: "app.preferences".into(),
                storage_class: StorageClass::UserConfig,
                consistency_group: "longhorn-config-store".into(),
                adapter: "longhorn-json-v1".into(),
                source_state: BackupSourceState::Present,
                source_schema_version: Some(1),
                target_schema_version: Some(2),
                compatibility: RestoreDomainCompatibilityProjection::MigrationRequired {
                    from: 1,
                    to: 2,
                },
            },
            RestoreDomainInspectionProjection {
                domain_id: "app.database".into(),
                storage_class: StorageClass::MachineState,
                consistency_group: "sqlite-main".into(),
                adapter: "sqlite-v1".into(),
                source_state: BackupSourceState::Present,
                source_schema_version: Some(1),
                target_schema_version: Some(1),
                compatibility: RestoreDomainCompatibilityProjection::CustomAdapterReady {
                    adapter: "sqlite-v1".into(),
                    participation: RestoreAdapterParticipationProjection::Separate,
                    confirmation_digest: digest('7'),
                },
            },
            RestoreDomainInspectionProjection {
                domain_id: "app.future".into(),
                storage_class: StorageClass::UserConfig,
                consistency_group: "longhorn-config-store".into(),
                adapter: "longhorn-json-v1".into(),
                source_state: BackupSourceState::SourcePreserved,
                source_schema_version: Some(9),
                target_schema_version: Some(2),
                compatibility: RestoreDomainCompatibilityProjection::SourcePreserved {
                    issue: "FutureSchema".into(),
                },
            },
        ],
        exclusions: vec![RestoreExclusionProjection {
            domain_id: "app.secrets".into(),
            storage_class: StorageClass::Secret,
            reason: "secret".into(),
            registered: true,
        }],
        receipt: RestoreInspectionReceiptProjection {
            manifest_domains: 3,
            exclusions: 1,
            restorable: 1,
            migrations: 1,
            adapter_restorable: 1,
            blocked: 1,
        },
    }
}

pub(super) fn restore_plan() -> RestorePlanProjection {
    RestorePlanProjection {
        archive_sha256: digest('e'),
        confirmation_digest: digest('8'),
        entries: vec![
            RestorePlanEntryProjection {
                domain_id: "app.preferences".into(),
                choice: RestoreConflictChoiceProjection::UseArchive,
                action: Some("migrate".into()),
                current: Some(RestoreCurrentEvidenceProjection::Present {
                    byte_length: 128,
                    sha256: digest('5'),
                }),
            },
            RestorePlanEntryProjection {
                domain_id: "app.future".into(),
                choice: RestoreConflictChoiceProjection::KeepCurrent,
                action: None,
                current: None,
            },
        ],
        receipt: RestorePlanReceiptProjection {
            selected: 1,
            skipped: 1,
            creates: 0,
            replaces: 0,
            deletes: 0,
            migrations: 1,
            unchanged: 0,
        },
    }
}

pub(super) fn restore_execution_receipt() -> RestoreExecutionReceiptProjection {
    RestoreExecutionReceiptProjection {
        confirmation_digest: digest('8'),
        staging: RestoreStagingReceiptProjection {
            selected: 1,
            documents: 1,
            deletions: 0,
            unchanged: 0,
            total_document_bytes: 256,
        },
        safety_backup: BackupPublicationReceiptProjection {
            path: "/backups/pre-restore.longhorn-backup".into(),
            destination: "operational".into(),
            archive_sha256: digest('9'),
            durability: "fileAndDirectorySynced".into(),
            replaced_existing: false,
        },
        restored_domain_ids: vec!["app.preferences".into()],
        deleted_domain_ids: vec![],
        migrated_domain_ids: vec!["app.preferences".into()],
        unchanged_domain_ids: vec![],
        skipped_domain_ids: vec!["app.future".into()],
        excluded_domain_ids: vec!["app.secrets".into()],
    }
}

fn restore_failure(
    terminal: RestoreFailureTerminalProjection,
    state: RestoreOperationStateProjection,
) -> RestoreExecuteOutcome {
    let mut state_snapshot = snapshot();
    state_snapshot.restore = Some(RestoreOperationsProjection {
        state,
        safety_backup_sha256: Some(digest('9')),
    });
    let failure = RestoreExecutionFailureProjection {
        stage: RestoreExecutionStageProjection::PublishTarget,
        domain_id: Some("app.preferences".into()),
        terminal,
        detail: "fixture publication failure".into(),
    };
    if terminal == RestoreFailureTerminalProjection::RolledBack {
        RestoreExecuteOutcome::RolledBack {
            failure,
            snapshot: Box::new(state_snapshot),
        }
    } else {
        RestoreExecuteOutcome::RecoveryRequired {
            failure,
            snapshot: Box::new(state_snapshot),
        }
    }
}

pub(super) fn restore_inspection_states() -> Vec<RestoreInspectOutcome> {
    vec![
        RestoreInspectOutcome::Ready {
            generation: ConfigGeneration::new(8),
            inspection: Box::new(restore_inspection()),
        },
        RestoreInspectOutcome::Locked {
            detail: "configured identity is unavailable".into(),
        },
        RestoreInspectOutcome::Rejected {
            rejection: rejection(ConfigOperationRejectionCode::ArchiveCorrupt),
        },
        RestoreInspectOutcome::Rejected {
            rejection: rejection(ConfigOperationRejectionCode::ArchiveFutureVersion),
        },
    ]
}

pub(super) fn restore_plan_states() -> Vec<RestorePlanOutcome> {
    vec![
        RestorePlanOutcome::Ready {
            generation: ConfigGeneration::new(8),
            plan: restore_plan(),
        },
        RestorePlanOutcome::Rejected {
            rejection: rejection(ConfigOperationRejectionCode::RestorePlanStale),
        },
    ]
}

pub(super) fn restore_execution_states() -> Vec<RestoreExecuteOutcome> {
    vec![
        RestoreExecuteOutcome::Succeeded {
            receipt: Box::new(restore_execution_receipt()),
            snapshot: Box::new(snapshot()),
        },
        restore_failure(
            RestoreFailureTerminalProjection::RolledBack,
            RestoreOperationStateProjection::Inactive,
        ),
        restore_failure(
            RestoreFailureTerminalProjection::RecoveryRequired,
            RestoreOperationStateProjection::RecoveryRequired,
        ),
    ]
}
