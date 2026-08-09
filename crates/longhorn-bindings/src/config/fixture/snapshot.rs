use longhorn_config::{
    BackupArchiveProjection, BackupEncryptionState, BackupInventoryProjection,
    BackupOperationsProjection, BackupPendingState, BackupRetentionProjection,
    BackupRetentionReasonProjection, ConfigGeneration, ConfigOperationCapability,
    ConfigOperationsSnapshot, ConfigProtocolVersion, RestoreOperationStateProjection,
    RestoreOperationsProjection, StorageBootstrapProjection, StorageLayoutProjection,
    StorageLeafProvenanceProjection, StorageOperationsProjection, StorageProfileId,
    StorageRootProjection,
};

use super::{digest, inventory_states};

pub(super) fn snapshot() -> ConfigOperationsSnapshot {
    ConfigOperationsSnapshot {
        protocol_version: ConfigProtocolVersion::CURRENT,
        generation: ConfigGeneration::new(7),
        capabilities: vec![
            ConfigOperationCapability::StorageDiagnostics,
            ConfigOperationCapability::StorageTransition,
            ConfigOperationCapability::BackupInventory,
            ConfigOperationCapability::BackupCreate,
            ConfigOperationCapability::BackupExport,
            ConfigOperationCapability::BackupRetention,
            ConfigOperationCapability::BackupEncryption,
            ConfigOperationCapability::RestoreInspection,
            ConfigOperationCapability::RestoreExecution,
            ConfigOperationCapability::RestoreAdapterExecution,
            ConfigOperationCapability::RestoreRecovery,
        ],
        storage: Some(StorageOperationsProjection {
            layout: StorageLayoutProjection {
                profile: StorageProfileId::PlatformNativeV1,
                platform: "macos".into(),
                canonical_application_id: "audio.example.soundcheck".into(),
                effective_leaf: "audio.example.soundcheck".into(),
                leaf_provenance: StorageLeafProvenanceProjection::CanonicalApplicationId,
                roots: vec![
                    StorageRootProjection {
                        kind: "config".into(),
                        path:
                            "/Users/example/Library/Application Support/audio.example.soundcheck"
                                .into(),
                        provenance: "platform:config".into(),
                    },
                    StorageRootProjection {
                        kind: "cache".into(),
                        path: "/Users/example/Library/Caches/audio.example.soundcheck".into(),
                        provenance: "platform:cache".into(),
                    },
                ],
                warnings: vec![],
                layout_digest: digest('a'),
            },
            bootstrap: StorageBootstrapProjection::Selected {
                origin: "locator".into(),
                locator_path: Some(
                    "/Users/example/Library/Application Support/Longhorn/locator.json".into(),
                ),
                transition_id: None,
                last_committed_layout_digest: Some(digest('a')),
            },
            available_profiles: vec![
                StorageProfileId::PlatformNativeV1,
                StorageProfileId::UnifiedAppRootV1,
                StorageProfileId::SharedProductRootV1,
                StorageProfileId::PortableV1,
            ],
        }),
        backup: Some(BackupOperationsProjection {
            inventory: BackupInventoryProjection {
                root: "/backups".into(),
                archives: vec![BackupArchiveProjection {
                    path: "/backups/current.longhorn-backup".into(),
                    archive_id: "backup:fixture".into(),
                    created_at: "2026-07-29T12:00:00Z".into(),
                    kind: "manual".into(),
                    archive_sha256: digest('e'),
                }],
                entries: inventory_states(),
                complete: true,
            },
            pending: BackupPendingState::Pending {
                domain_count: 1,
                domain_ids: vec!["app.preferences".into()],
            },
            encryption: BackupEncryptionState::Available {
                provider: "age".into(),
            },
            retention: Some(BackupRetentionProjection {
                deletion_paths: vec!["/backups/old.longhorn-backup".into()],
                retained: vec![(
                    digest('e'),
                    vec![BackupRetentionReasonProjection::NewestCount],
                )],
                diagnostics: inventory_states(),
                confirmation_digest: digest('f'),
            }),
        }),
        restore: Some(RestoreOperationsProjection {
            state: RestoreOperationStateProjection::Inactive,
            safety_backup_sha256: None,
        }),
    }
}
