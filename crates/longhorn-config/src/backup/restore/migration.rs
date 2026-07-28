use crate::{
    AccessMode, BackupCatalog, BackupKind, ConfigDomain, ConfigStore, DomainLocation, LoadOutcome,
    LoadedOrigin, MigrationRewriteError, MigrationRewriteOptions, MigrationRewriteReceipt,
    RestoreAction, RestoreCurrentEvidence, RestoreExecutionOptions, RestoreExecutionStage,
    RestoreFailureTerminal, RestoreStaging, RootKind, Sha256Digest, StoreError,
    store::{document::SerializedDocument, load::load_file},
};

use super::{
    execution::{execute_guarded, failure},
    recovery,
    types::{RestoreStagingReceipt, StagedDomain},
};

pub(crate) fn rewrite<D: ConfigDomain>(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    domain: &D,
    options: MigrationRewriteOptions,
) -> Result<MigrationRewriteReceipt, MigrationRewriteError> {
    if options.safety_backup.metadata.kind() != BackupKind::PreMigration {
        return Err(MigrationRewriteError::Preparation(
            "safety backup metadata must use pre-migration kind".into(),
        ));
    }
    let descriptor = store
        .registered_descriptor(domain.descriptor().id())
        .ok_or_else(|| {
            MigrationRewriteError::Store(StoreError::NotRegistered {
                id: domain.descriptor().id().clone(),
            })
        })?;
    if descriptor != domain.descriptor() {
        return Err(MigrationRewriteError::Store(
            StoreError::DescriptorChanged {
                id: domain.descriptor().id().clone(),
            },
        ));
    }
    let DomainLocation::File(file) = store.roots.resolve(descriptor) else {
        return Err(MigrationRewriteError::Unavailable(
            crate::UnavailableState::Authority {
                location: store.roots.resolve(descriptor),
            },
        ));
    };
    if file.access() != AccessMode::ReadWrite || file.root_kind() == RootKind::Project {
        return Err(MigrationRewriteError::Preparation(
            "domain does not have ordinary local write authority".into(),
        ));
    }
    let guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(|error| {
            MigrationRewriteError::Execution(failure(
                RestoreExecutionStage::RecoverPrevious,
                Some(descriptor.id().clone()),
                RestoreFailureTerminal::NoLiveMutation,
                error,
            ))
        })?;
    recovery::recover_guarded(store, &guard).map_err(|error| {
        MigrationRewriteError::Execution(failure(
            RestoreExecutionStage::RecoverPrevious,
            error.domain.clone(),
            RestoreFailureTerminal::RecoveryRequired,
            error,
        ))
    })?;
    let loaded = match load_file(domain, &file) {
        LoadOutcome::Ready(loaded) => loaded,
        LoadOutcome::Recovery(recovery) => {
            return Err(MigrationRewriteError::Recovery(recovery));
        }
        LoadOutcome::Unavailable(unavailable) => {
            return Err(MigrationRewriteError::Unavailable(unavailable));
        }
    };
    let (from, to) = match loaded.origin {
        LoadedOrigin::MigratedInMemory { from, to } => (from, to),
        LoadedOrigin::Default | LoadedOrigin::File => {
            return Err(MigrationRewriteError::NotRequired);
        }
    };
    let source = loaded.source.ok_or_else(|| {
        MigrationRewriteError::Preparation("migration source bytes missing".into())
    })?;
    let value = domain
        .encode(&loaded.value)
        .map_err(|issue| MigrationRewriteError::Preparation(issue.message))?;
    domain
        .validate_raw(descriptor.schema_version(), &value)
        .map_err(|issue| MigrationRewriteError::Preparation(issue.message))?;
    let target = serde_json::to_vec_pretty(&SerializedDocument::new(
        descriptor.id().clone(),
        descriptor.schema_version(),
        value,
    ))
    .map_err(|error| MigrationRewriteError::Preparation(error.to_string()))?;
    let source_sha = Sha256Digest::from_bytes(&source.bytes);
    let target_sha = Sha256Digest::from_bytes(&target);
    let plan_digest = Sha256Digest::from_bytes(
        &serde_json::to_vec(&serde_json::json!({
            "operation": "pre-migration-rewrite",
            "domain": descriptor.id().as_str(),
            "from": from.get(),
            "to": to.get(),
            "sourceSha256": source_sha.as_str(),
            "targetSha256": target_sha.as_str(),
        }))
        .expect("migration plan form is serializable"),
    );
    let target_length = target.len() as u64;
    let staging = RestoreStaging {
        archive_sha256: source_sha.clone(),
        plan_digest,
        domains: vec![StagedDomain {
            domain: descriptor.id().clone(),
            action: RestoreAction::Migrate,
            path: file.full_path().to_path_buf(),
            current: RestoreCurrentEvidence::Present {
                byte_length: source.bytes.len() as u64,
                sha256: source_sha,
            },
            schema_version: Some(descriptor.schema_version()),
            bytes: Some(target),
        }],
        receipt: RestoreStagingReceipt {
            selected: 1,
            documents: 1,
            deletions: 0,
            unchanged: 0,
            total_document_bytes: target_length,
        },
    };
    let execution = execute_guarded(
        store,
        catalog,
        staging,
        RestoreExecutionOptions::new(options.lock_timeout, options.safety_backup),
        &guard,
    )
    .map_err(MigrationRewriteError::Execution)?;
    Ok(MigrationRewriteReceipt {
        domain: descriptor.id().clone(),
        from,
        to,
        safety_backup: execution.safety_backup,
    })
}
