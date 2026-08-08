use crate::{
    BackupAdapterRestoreParticipation, BackupSourceState, DomainLocation, RestoreAction,
    RestoreConflictChoice, RestoreCurrentEvidence, RestoreDomainCompatibility,
    RestoreExecutionStage, RestoreFailureTerminal, RestoreIdentityStatus,
    operations::{
        RestoreAdapterParticipationProjection, RestoreConflictChoiceProjection,
        RestoreCurrentEvidenceProjection, RestoreDomainCompatibilityProjection,
        RestoreIdentityStatusProjection,
    },
};

use super::super::storage;

pub(crate) fn identity_status(value: &RestoreIdentityStatus) -> RestoreIdentityStatusProjection {
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

pub(crate) fn compatibility(
    value: &RestoreDomainCompatibility,
) -> RestoreDomainCompatibilityProjection {
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

pub(crate) fn participation(
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

pub(crate) fn current_evidence(value: &RestoreCurrentEvidence) -> RestoreCurrentEvidenceProjection {
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

pub(crate) const fn choice(value: RestoreConflictChoice) -> RestoreConflictChoiceProjection {
    match value {
        RestoreConflictChoice::UseArchive => RestoreConflictChoiceProjection::UseArchive,
        RestoreConflictChoice::KeepCurrent => RestoreConflictChoiceProjection::KeepCurrent,
    }
}

pub(crate) const fn action_id(value: RestoreAction) -> &'static str {
    match value {
        RestoreAction::Create => "create",
        RestoreAction::Replace => "replace",
        RestoreAction::Delete => "delete",
        RestoreAction::Migrate => "migrate",
        RestoreAction::Unchanged => "unchanged",
    }
}

pub(crate) const fn source_state(value: BackupSourceState) -> &'static str {
    match value {
        BackupSourceState::Present => "present",
        BackupSourceState::Absent => "absent",
        BackupSourceState::SourcePreserved => "sourcePreserved",
    }
}

pub(crate) fn location_id(value: &DomainLocation) -> String {
    match value {
        DomainLocation::File(_) => "file-authority".into(),
        DomainLocation::DefaultsOnly => "defaults-only".into(),
        DomainLocation::SecureStoreRequired => "secure-store-required".into(),
        DomainLocation::RootRequired { root, .. } => {
            format!("root-required:{}", storage::root_kind_id(*root))
        }
    }
}

pub(crate) const fn execution_stage(value: RestoreExecutionStage) -> &'static str {
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

pub(crate) const fn failure_terminal(value: RestoreFailureTerminal) -> &'static str {
    match value {
        RestoreFailureTerminal::NoLiveMutation => "noLiveMutation",
        RestoreFailureTerminal::RolledBack => "rolledBack",
        RestoreFailureTerminal::RecoveryRequired => "recoveryRequired",
    }
}

pub(crate) fn domain_ids(domains: &[longhorn_core::DomainId]) -> Vec<String> {
    domains
        .iter()
        .map(|domain| domain.as_str().into())
        .collect()
}
