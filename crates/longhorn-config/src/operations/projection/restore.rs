use crate::{
    BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation, BackupConsistencyMode,
    BackupSourceState, DomainLocation, RestoreAction, RestoreAdapterReceipt, RestoreConflictChoice,
    RestoreCurrentEvidence, RestoreDomainCompatibility, RestoreExecutionError,
    RestoreExecutionReceipt, RestoreExecutionStage, RestoreFailureTerminal, RestoreIdentityStatus,
    RestoreInspection, RestoreOperationState, RestorePlan, RestoreRecoveryOutcome,
    RestoreRecoveryReceipt, RestoreStagingReceipt, Sha256Digest,
    operations::{
        RestoreAdapterParticipationProjection, RestoreAdapterReceiptProjection,
        RestoreAuthenticityProjection, RestoreConflictChoiceProjection,
        RestoreConsistencyGroupProjection, RestoreCurrentEvidenceProjection,
        RestoreDomainCompatibilityProjection, RestoreDomainInspectionProjection,
        RestoreExclusionProjection, RestoreExecutionFailureProjection,
        RestoreExecutionReceiptProjection, RestoreIdentityProjection,
        RestoreIdentityStatusProjection, RestoreInspectionProjection,
        RestoreInspectionReceiptProjection, RestoreIntegrityProjection,
        RestoreOperationStateProjection, RestoreOperationsProjection, RestorePlanEntryProjection,
        RestorePlanProjection, RestorePlanReceiptProjection, RestoreRecoveryReceiptProjection,
        RestoreStagingReceiptProjection,
    },
};

use super::super::{BackupPublicationReceiptProjection, ConfigOperationProjectionError};

impl RestoreOperationsProjection {
    /// Projects durable journal state without journal or rollback paths.
    #[must_use]
    pub fn from_state(state: RestoreOperationState, safety_backup: Option<&Sha256Digest>) -> Self {
        Self {
            state: match state {
                RestoreOperationState::Inactive => RestoreOperationStateProjection::Inactive,
                RestoreOperationState::Active => RestoreOperationStateProjection::Active,
                RestoreOperationState::RecoveryRequired => {
                    RestoreOperationStateProjection::RecoveryRequired
                }
            },
            safety_backup_sha256: safety_backup.map(|digest| digest.as_str().into()),
        }
    }
}

impl RestoreInspectionProjection {
    /// Projects one side-effect-free verified inspection.
    #[must_use]
    pub fn from_inspection(
        inspection: &RestoreInspection,
        authenticity: RestoreAuthenticityProjection,
    ) -> Self {
        let manifest = inspection.manifest();
        Self {
            archive_sha256: inspection.archive_sha256().as_str().into(),
            archive_id: manifest.archive_id().into(),
            created_at: manifest.created_at().into(),
            kind: super::backup::backup_kind_id(manifest.kind()).into(),
            application_version: manifest.application().version().into(),
            producer_version: manifest.producer().version().into(),
            integrity: RestoreIntegrityProjection::Verified,
            authenticity,
            identity: RestoreIdentityProjection {
                application: identity_status(inspection.identity().application()),
                producer: identity_status(inspection.identity().producer()),
            },
            consistency_groups: manifest
                .consistency_groups()
                .iter()
                .map(|group| RestoreConsistencyGroupProjection {
                    id: group.id().into(),
                    mode: match group.mode() {
                        BackupConsistencyMode::CoordinatedBounded => "coordinatedBounded",
                        BackupConsistencyMode::ExternalSnapshot => "externalSnapshot",
                    }
                    .into(),
                    authority: group.authority().into(),
                })
                .collect(),
            domains: inspection
                .domains()
                .iter()
                .map(|domain| {
                    let source = manifest
                        .domains()
                        .iter()
                        .find(|source| source.domain() == domain.domain())
                        .expect("inspection domains originate from the verified manifest");
                    RestoreDomainInspectionProjection {
                        domain_id: domain.domain().as_str().into(),
                        storage_class: super::storage::storage_class_id(source.storage_class())
                            .into(),
                        consistency_group: source.consistency_group().into(),
                        adapter: source.adapter().into(),
                        source_state: source_state(source.state()).into(),
                        source_schema_version: domain
                            .source_schema_version()
                            .map(|version| version.get()),
                        target_schema_version: domain
                            .target_schema_version()
                            .map(|version| version.get()),
                        compatibility: compatibility(domain.compatibility()),
                    }
                })
                .collect(),
            exclusions: inspection
                .exclusions()
                .iter()
                .map(|report| RestoreExclusionProjection {
                    domain_id: report.exclusion().domain().as_str().into(),
                    storage_class: super::storage::storage_class_id(
                        report.exclusion().storage_class(),
                    )
                    .into(),
                    reason: report.exclusion().reason().into(),
                    registered: report.is_registered(),
                })
                .collect(),
            receipt: RestoreInspectionReceiptProjection {
                manifest_domains: inspection.receipt().manifest_domains(),
                exclusions: inspection.receipt().exclusions(),
                restorable: inspection.receipt().restorable(),
                migrations: inspection.receipt().migrations(),
                adapter_restorable: inspection.receipt().adapter_restorable(),
                blocked: inspection.receipt().blocked(),
            },
        }
    }
}

impl From<&RestorePlan> for RestorePlanProjection {
    fn from(plan: &RestorePlan) -> Self {
        Self {
            archive_sha256: plan.archive_sha256().as_str().into(),
            confirmation_digest: plan.digest().as_str().into(),
            entries: plan
                .entries()
                .iter()
                .map(|entry| RestorePlanEntryProjection {
                    domain_id: entry.domain().as_str().into(),
                    choice: choice(entry.choice()),
                    action: entry.action().map(|action| action_id(action).into()),
                    current: entry.current().map(current_evidence),
                })
                .collect(),
            receipt: RestorePlanReceiptProjection {
                selected: plan.receipt().selected(),
                skipped: plan.receipt().skipped(),
                creates: plan.receipt().actions(RestoreAction::Create),
                replaces: plan.receipt().actions(RestoreAction::Replace),
                deletes: plan.receipt().actions(RestoreAction::Delete),
                migrations: plan.receipt().actions(RestoreAction::Migrate),
                unchanged: plan.receipt().actions(RestoreAction::Unchanged),
            },
        }
    }
}

impl From<&RestoreStagingReceipt> for RestoreStagingReceiptProjection {
    fn from(receipt: &RestoreStagingReceipt) -> Self {
        Self {
            selected: receipt.selected(),
            documents: receipt.documents(),
            deletions: receipt.deletions(),
            unchanged: receipt.unchanged(),
            total_document_bytes: receipt.total_document_bytes(),
        }
    }
}

impl RestoreExecutionReceiptProjection {
    /// Combines exact inspection, planning, staging, and execution evidence.
    pub fn try_from_parts(
        execution: &RestoreExecutionReceipt,
        staging: &RestoreStagingReceipt,
        plan: &RestorePlan,
        inspection: &RestoreInspection,
    ) -> Result<Self, ConfigOperationProjectionError> {
        Ok(Self {
            confirmation_digest: execution.plan_digest().as_str().into(),
            staging: staging.into(),
            safety_backup: BackupPublicationReceiptProjection::try_from(execution.safety_backup())?,
            restored_domain_ids: domain_ids(execution.restored()),
            deleted_domain_ids: domain_ids(execution.deleted()),
            migrated_domain_ids: domain_ids(execution.migrated()),
            unchanged_domain_ids: domain_ids(execution.unchanged()),
            skipped_domain_ids: plan
                .entries()
                .iter()
                .filter(|entry| entry.choice() == RestoreConflictChoice::KeepCurrent)
                .map(|entry| entry.domain().as_str().into())
                .collect(),
            excluded_domain_ids: inspection
                .exclusions()
                .iter()
                .map(|entry| entry.exclusion().domain().as_str().into())
                .collect(),
        })
    }
}

impl From<&RestoreExecutionError> for RestoreExecutionFailureProjection {
    fn from(error: &RestoreExecutionError) -> Self {
        Self {
            stage: execution_stage(error.stage).into(),
            domain_id: error.domain.as_ref().map(|domain| domain.as_str().into()),
            terminal: failure_terminal(error.terminal).into(),
            detail: error.detail.clone(),
        }
    }
}

impl From<&RestoreRecoveryReceipt> for RestoreRecoveryReceiptProjection {
    fn from(receipt: &RestoreRecoveryReceipt) -> Self {
        Self {
            outcome: match receipt.outcome() {
                RestoreRecoveryOutcome::NoRecoveryNeeded => "noRecoveryNeeded",
                RestoreRecoveryOutcome::RolledBack => "rolledBack",
                RestoreRecoveryOutcome::TerminalCleanup => "terminalCleanup",
            }
            .into(),
            domain_ids: domain_ids(receipt.domains()),
        }
    }
}

impl From<&RestoreAdapterReceipt> for RestoreAdapterReceiptProjection {
    fn from(receipt: &RestoreAdapterReceipt) -> Self {
        let (outcome, evidence) = match receipt.outcome() {
            BackupAdapterRestoreOutcome::Verified { evidence } => {
                ("verified", Some(evidence.as_str().into()))
            }
            BackupAdapterRestoreOutcome::RolledBack { evidence } => {
                ("rolledBack", Some(evidence.as_str().into()))
            }
            BackupAdapterRestoreOutcome::RecoveryRequired => ("recoveryRequired", None),
        };
        Self {
            domain_id: receipt.domain().as_str().into(),
            adapter: receipt.adapter().as_str().into(),
            participation: participation(receipt.participation()),
            confirmation_digest: receipt.confirmation_digest().as_str().into(),
            outcome: outcome.into(),
            evidence,
        }
    }
}

fn identity_status(value: &RestoreIdentityStatus) -> RestoreIdentityStatusProjection {
    match value {
        RestoreIdentityStatus::Compatible => RestoreIdentityStatusProjection::Compatible,
        RestoreIdentityStatus::Mismatch { expected, archive } => {
            RestoreIdentityStatusProjection::Mismatch {
                expected: expected.clone(),
                archive: archive.clone(),
            }
        }
    }
}

fn compatibility(value: &RestoreDomainCompatibility) -> RestoreDomainCompatibilityProjection {
    match value {
        RestoreDomainCompatibility::Ready => RestoreDomainCompatibilityProjection::Ready,
        RestoreDomainCompatibility::MigrationRequired { from, to } => {
            RestoreDomainCompatibilityProjection::MigrationRequired {
                from: from.get(),
                to: to.get(),
            }
        }
        RestoreDomainCompatibility::UnknownDomain => {
            RestoreDomainCompatibilityProjection::UnknownDomain
        }
        RestoreDomainCompatibility::DescriptorMismatch => {
            RestoreDomainCompatibilityProjection::DescriptorMismatch
        }
        RestoreDomainCompatibility::DomainCodeUnavailable => {
            RestoreDomainCompatibilityProjection::DomainCodeUnavailable
        }
        RestoreDomainCompatibility::PolicyExcluded { reason } => {
            RestoreDomainCompatibilityProjection::PolicyExcluded {
                reason: reason.clone(),
            }
        }
        RestoreDomainCompatibility::CustomAdapterUnavailable { adapter } => {
            RestoreDomainCompatibilityProjection::CustomAdapterUnavailable {
                adapter: adapter.clone(),
            }
        }
        RestoreDomainCompatibility::CustomAdapterReady {
            adapter,
            participation: adapter_participation,
            confirmation_digest,
        } => RestoreDomainCompatibilityProjection::CustomAdapterReady {
            adapter: adapter.as_str().into(),
            participation: participation(adapter_participation),
            confirmation_digest: confirmation_digest.as_str().into(),
        },
        RestoreDomainCompatibility::CustomAdapterRejected { adapter, detail } => {
            RestoreDomainCompatibilityProjection::CustomAdapterRejected {
                adapter: adapter.as_str().into(),
                detail: detail.clone(),
            }
        }
        RestoreDomainCompatibility::TargetUnavailable { location } => {
            RestoreDomainCompatibilityProjection::TargetUnavailable {
                reason: location_id(location),
            }
        }
        RestoreDomainCompatibility::SourcePreserved { issue } => {
            RestoreDomainCompatibilityProjection::SourcePreserved {
                issue: format!("{issue:?}"),
            }
        }
        RestoreDomainCompatibility::SourceRejected { issue } => {
            RestoreDomainCompatibilityProjection::SourceRejected {
                issue: format!("{issue:?}"),
            }
        }
        RestoreDomainCompatibility::TargetPreparationFailed { detail } => {
            RestoreDomainCompatibilityProjection::TargetPreparationFailed {
                detail: detail.clone(),
            }
        }
    }
}

fn participation(
    value: &BackupAdapterRestoreParticipation,
) -> RestoreAdapterParticipationProjection {
    match value {
        BackupAdapterRestoreParticipation::Excluded(reason) => {
            RestoreAdapterParticipationProjection::Excluded {
                reason: reason.as_str().into(),
            }
        }
        BackupAdapterRestoreParticipation::Separate => {
            RestoreAdapterParticipationProjection::Separate
        }
        BackupAdapterRestoreParticipation::FailureAtomic => {
            RestoreAdapterParticipationProjection::FailureAtomic
        }
        BackupAdapterRestoreParticipation::GroupedFailureAtomic => {
            RestoreAdapterParticipationProjection::GroupedFailureAtomic
        }
    }
}

fn current_evidence(value: &RestoreCurrentEvidence) -> RestoreCurrentEvidenceProjection {
    match value {
        RestoreCurrentEvidence::Absent => RestoreCurrentEvidenceProjection::Absent,
        RestoreCurrentEvidence::Present {
            byte_length,
            sha256,
        } => RestoreCurrentEvidenceProjection::Present {
            byte_length: *byte_length,
            sha256: sha256.as_str().into(),
        },
    }
}

const fn choice(value: RestoreConflictChoice) -> RestoreConflictChoiceProjection {
    match value {
        RestoreConflictChoice::UseArchive => RestoreConflictChoiceProjection::UseArchive,
        RestoreConflictChoice::KeepCurrent => RestoreConflictChoiceProjection::KeepCurrent,
    }
}

const fn action_id(value: RestoreAction) -> &'static str {
    match value {
        RestoreAction::Create => "create",
        RestoreAction::Replace => "replace",
        RestoreAction::Delete => "delete",
        RestoreAction::Migrate => "migrate",
        RestoreAction::Unchanged => "unchanged",
    }
}

const fn source_state(value: BackupSourceState) -> &'static str {
    match value {
        BackupSourceState::Present => "present",
        BackupSourceState::Absent => "absent",
        BackupSourceState::SourcePreserved => "sourcePreserved",
    }
}

fn location_id(value: &DomainLocation) -> String {
    match value {
        DomainLocation::File(_) => "file-authority".into(),
        DomainLocation::DefaultsOnly => "defaults-only".into(),
        DomainLocation::SecureStoreRequired => "secure-store-required".into(),
        DomainLocation::RootRequired { root, .. } => {
            format!("root-required:{}", super::storage::root_kind_id(*root))
        }
    }
}

const fn execution_stage(value: RestoreExecutionStage) -> &'static str {
    match value {
        RestoreExecutionStage::ValidateSafetyBackup => "validateSafetyBackup",
        RestoreExecutionStage::RecoverPrevious => "recoverPrevious",
        RestoreExecutionStage::RecheckCurrent => "recheckCurrent",
        RestoreExecutionStage::CaptureRollback => "captureRollback",
        RestoreExecutionStage::CaptureSafetyBackup => "captureSafetyBackup",
        RestoreExecutionStage::EncodeSafetyBackup => "encodeSafetyBackup",
        RestoreExecutionStage::PublishSafetyBackup => "publishSafetyBackup",
        RestoreExecutionStage::PublishJournal => "publishJournal",
        RestoreExecutionStage::PublishTarget => "publishTarget",
        RestoreExecutionStage::VerifyTarget => "verifyTarget",
        RestoreExecutionStage::Rollback => "rollback",
        RestoreExecutionStage::Cleanup => "cleanup",
    }
}

const fn failure_terminal(value: RestoreFailureTerminal) -> &'static str {
    match value {
        RestoreFailureTerminal::NoLiveMutation => "noLiveMutation",
        RestoreFailureTerminal::RolledBack => "rolledBack",
        RestoreFailureTerminal::RecoveryRequired => "recoveryRequired",
    }
}

fn domain_ids(domains: &[longhorn_core::DomainId]) -> Vec<String> {
    domains
        .iter()
        .map(|domain| domain.as_str().into())
        .collect()
}
