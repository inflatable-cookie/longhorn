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

use crate::{BackupPublicationReceiptProjection, ConfigOperationProjectionError};

use super::super::{backup, storage};
use super::{
    action_id, choice, compatibility, current_evidence, domain_ids, execution_stage,
    failure_terminal, identity_status, location_id, participation, source_state,
};


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
            kind: backup::backup_kind_id(manifest.kind()).into(),
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
                        storage_class: storage::storage_class_id(source.storage_class()).into(),
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
                    storage_class: storage::storage_class_id(report.exclusion().storage_class())
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
