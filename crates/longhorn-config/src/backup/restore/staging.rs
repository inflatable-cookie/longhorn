use std::path::{Path, PathBuf};

use longhorn_core::SchemaVersion;

use crate::{
    BackupArchiveInspection, BackupCatalog, BackupSourceIssue, BackupSourceState, ConfigDomain,
    ConfigStore, DomainLocation, LoadOutcome, LoadedOrigin, RecoveryKind, Sha256Digest,
    SourceDocument, backup::CatalogDecision, store::document::SerializedDocument,
    store::load::load_source,
};

use super::{
    inspection::payload_for,
    planning::read_current_evidence,
    types::{
        RestoreAction, RestoreInspection, RestorePlan, RestorePrepareError, RestorePrepareOptions,
        RestoreStaging, RestoreStagingReceipt, StagedDomain,
    },
};

pub(crate) fn prepare(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    inspection: &RestoreInspection,
    plan: &RestorePlan,
    options: RestorePrepareOptions,
) -> Result<RestoreStaging, RestorePrepareError> {
    if archive.archive_sha256() != plan.archive_sha256()
        || inspection.archive_sha256() != plan.archive_sha256()
    {
        return Err(RestorePrepareError::ArchiveChanged);
    }

    let _guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(RestorePrepareError::Coordination)?;
    for target in &plan.targets {
        let descriptor = store.registered_descriptor(&target.domain).ok_or_else(|| {
            RestorePrepareError::DomainCapabilityChanged {
                domain: target.domain.clone(),
            }
        })?;
        let DomainLocation::File(file) = store.roots.resolve(descriptor) else {
            return Err(RestorePrepareError::DomainCapabilityChanged {
                domain: target.domain.clone(),
            });
        };
        let observed = read_current_evidence(&file).map_err(|error| {
            RestorePrepareError::CurrentReadFailed {
                domain: target.domain.clone(),
                path: target.path.clone(),
                detail: error.to_string(),
            }
        })?;
        if observed != target.current {
            return Err(RestorePrepareError::StaleCurrent {
                domain: target.domain.clone(),
                planned: target.current.clone(),
                observed,
            });
        }
    }

    let mut domains = Vec::with_capacity(plan.targets.len());
    let mut documents = 0;
    let mut deletions = 0;
    let mut unchanged = 0;
    let mut total_document_bytes = 0_u64;
    for target in &plan.targets {
        let source = inspection
            .manifest
            .domains()
            .iter()
            .find(|source| source.domain() == &target.domain)
            .expect("planned domain exists in inspected manifest");
        let (schema_version, bytes) = if source.state() == BackupSourceState::Absent {
            (None, None)
        } else {
            let descriptor = store.registered_descriptor(&target.domain).ok_or_else(|| {
                RestorePrepareError::DomainCapabilityChanged {
                    domain: target.domain.clone(),
                }
            })?;
            let domain = match catalog.decision(descriptor) {
                Some(CatalogDecision::Include(domain)) => domain,
                _ => {
                    return Err(RestorePrepareError::DomainCapabilityChanged {
                        domain: target.domain.clone(),
                    });
                }
            };
            let payload = payload_for(archive, source).ok_or_else(|| {
                RestorePrepareError::DomainCapabilityChanged {
                    domain: target.domain.clone(),
                }
            })?;
            let prepared = domain
                .prepare_restore_source(
                    payload,
                    Path::new(
                        source
                            .payloads()
                            .first()
                            .expect("planned present source has one payload")
                            .path()
                            .as_str(),
                    ),
                )
                .map_err(|error| RestorePrepareError::DomainStagingFailed {
                    domain: target.domain.clone(),
                    detail: error.detail(),
                })?;
            let sha256 = Sha256Digest::from_bytes(&prepared.bytes);
            if Some(&sha256) != target.target_sha256.as_ref()
                || Some(prepared.schema_version) != target.target_schema_version
            {
                return Err(RestorePrepareError::TargetChanged {
                    domain: target.domain.clone(),
                });
            }
            (Some(prepared.schema_version), Some(prepared.bytes))
        };

        if let Some(bytes) = bytes.as_ref() {
            documents += 1;
            total_document_bytes += bytes.len() as u64;
        } else if target.action == RestoreAction::Delete {
            deletions += 1;
        }
        if target.action == RestoreAction::Unchanged {
            unchanged += 1;
        }
        domains.push(StagedDomain {
            domain: target.domain.clone(),
            action: target.action,
            path: target.path.clone(),
            current: target.current.clone(),
            schema_version,
            bytes,
        });
    }

    Ok(RestoreStaging {
        archive_sha256: plan.archive_sha256.clone(),
        plan_digest: plan.digest.clone(),
        domains,
        receipt: RestoreStagingReceipt {
            selected: plan.targets.len(),
            documents,
            deletions,
            unchanged,
            total_document_bytes,
        },
    })
}

pub(crate) struct PreparedRestoreSource {
    pub(super) source_schema_version: SchemaVersion,
    pub(super) schema_version: SchemaVersion,
    pub(super) bytes: Vec<u8>,
}

pub(crate) enum PrepareSourceError {
    Source(BackupSourceIssue),
    Target(String),
}

impl PrepareSourceError {
    fn detail(self) -> String {
        match self {
            Self::Source(issue) => format!("{issue:?}"),
            Self::Target(detail) => detail,
        }
    }
}

pub(crate) fn prepare_typed_source<D: ConfigDomain>(
    domain: &D,
    bytes: &[u8],
    path: &Path,
) -> Result<PreparedRestoreSource, PrepareSourceError> {
    let source = SourceDocument {
        path: PathBuf::from(path),
        bytes: bytes.to_vec(),
    };
    let loaded = match load_source(domain, source) {
        LoadOutcome::Ready(loaded) => loaded,
        LoadOutcome::Recovery(recovery) => {
            let issue = backup_issue(recovery.kind).ok_or_else(|| {
                PrepareSourceError::Target(format!(
                    "unclassified source recovery: {}",
                    recovery.detail
                ))
            })?;
            return Err(PrepareSourceError::Source(issue));
        }
        LoadOutcome::Unavailable(unavailable) => {
            return Err(PrepareSourceError::Target(format!(
                "unexpected unavailable source: {:?}",
                unavailable
            )));
        }
    };
    let source_schema_version = match loaded.origin {
        LoadedOrigin::File => loaded.schema_version,
        LoadedOrigin::MigratedInMemory { from, .. } => from,
        LoadedOrigin::Default => {
            return Err(PrepareSourceError::Target(
                "archive source unexpectedly resolved to defaults".into(),
            ));
        }
    };
    let value = domain
        .encode(&loaded.value)
        .map_err(|issue| PrepareSourceError::Target(format!("encode: {}", issue.message)))?;
    domain
        .validate_raw(domain.descriptor().schema_version(), &value)
        .map_err(|issue| {
            PrepareSourceError::Target(format!("validate encoded: {}", issue.message))
        })?;
    let document = SerializedDocument::new(
        domain.descriptor().id().clone(),
        domain.descriptor().schema_version(),
        value,
    );
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| PrepareSourceError::Target(format!("serialize: {error}")))?;
    Ok(PreparedRestoreSource {
        source_schema_version,
        schema_version: domain.descriptor().schema_version(),
        bytes,
    })
}

fn backup_issue(kind: RecoveryKind) -> Option<BackupSourceIssue> {
    match kind {
        RecoveryKind::CorruptDocument => Some(BackupSourceIssue::CorruptDocument),
        RecoveryKind::DomainMismatch => Some(BackupSourceIssue::DomainMismatch),
        RecoveryKind::FutureSchema => Some(BackupSourceIssue::FutureSchema),
        RecoveryKind::InvalidValue => Some(BackupSourceIssue::InvalidValue),
        RecoveryKind::MissingMigration => Some(BackupSourceIssue::MissingMigration),
        RecoveryKind::InvalidMigrationStep => Some(BackupSourceIssue::InvalidMigrationStep),
        RecoveryKind::MigrationFailed => Some(BackupSourceIssue::MigrationFailed),
        RecoveryKind::DecodeFailed => Some(BackupSourceIssue::DecodeFailed),
        RecoveryKind::ReadFailed | RecoveryKind::InvalidDefault => None,
    }
}
