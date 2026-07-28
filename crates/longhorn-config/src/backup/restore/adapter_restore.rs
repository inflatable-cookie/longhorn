use longhorn_core::DomainId;

use crate::{
    BackupAdapterInspectRequest, BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation,
    BackupAdapterRestoreRequest, BackupArchiveInspection, BackupCatalog, ConfigStore, Sha256Digest,
    backup::CatalogDecision,
};

use super::{
    inspection::payloads_for_adapter,
    types::{
        RestoreAdapterError, RestoreAdapterReceipt, RestoreAdapterRequirement, RestoreInspection,
    },
};

pub(crate) fn execute(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    inspection: &RestoreInspection,
    domain: &DomainId,
    confirmation: &Sha256Digest,
    requirement: RestoreAdapterRequirement,
) -> Result<RestoreAdapterReceipt, RestoreAdapterError> {
    if !inspection.identity.is_compatible() {
        return Err(RestoreAdapterError::IdentityMismatch);
    }
    if archive.archive_sha256() != inspection.archive_sha256() {
        return Err(RestoreAdapterError::ArchiveChanged);
    }
    let source = inspection
        .manifest
        .domains()
        .iter()
        .find(|source| source.domain() == domain)
        .ok_or_else(|| RestoreAdapterError::UnknownDomain {
            domain: domain.clone(),
        })?;
    let prepared = inspection.custom_prepared.get(domain).ok_or_else(|| {
        RestoreAdapterError::UnknownDomain {
            domain: domain.clone(),
        }
    })?;
    if &prepared.confirmation_digest != confirmation {
        return Err(RestoreAdapterError::ConfirmationMismatch {
            domain: domain.clone(),
        });
    }
    let descriptor =
        store
            .registered_descriptor(domain)
            .ok_or_else(|| RestoreAdapterError::AdapterChanged {
                domain: domain.clone(),
            })?;
    let adapter = match catalog.decision(descriptor) {
        Some(CatalogDecision::Custom(adapter))
            if adapter.id() == &prepared.adapter
                && adapter.id().as_str() == source.adapter()
                && adapter.capabilities().restore() == &prepared.participation =>
        {
            adapter
        }
        _ => {
            return Err(RestoreAdapterError::AdapterChanged {
                domain: domain.clone(),
            });
        }
    };
    match &prepared.participation {
        BackupAdapterRestoreParticipation::Excluded(reason) => {
            return Err(RestoreAdapterError::Excluded {
                domain: domain.clone(),
                reason: reason.as_str().into(),
            });
        }
        BackupAdapterRestoreParticipation::Separate
            if requirement == RestoreAdapterRequirement::FailureAtomic =>
        {
            return Err(RestoreAdapterError::FailureAtomicRequired {
                domain: domain.clone(),
                adapter: prepared.adapter.clone(),
            });
        }
        BackupAdapterRestoreParticipation::Separate
        | BackupAdapterRestoreParticipation::FailureAtomic => {}
    }

    let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
        RestoreAdapterError::PreviewChanged {
            domain: domain.clone(),
        }
    })?;
    let current_preview = adapter
        .inspect(BackupAdapterInspectRequest::new(
            descriptor,
            source.source_schema_version(),
            payloads,
        ))
        .map_err(|error| RestoreAdapterError::AdapterFailed {
            domain: domain.clone(),
            adapter: prepared.adapter.clone(),
            error,
        })?;
    if current_preview != prepared.preview {
        return Err(RestoreAdapterError::PreviewChanged {
            domain: domain.clone(),
        });
    }

    let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
        RestoreAdapterError::PreviewChanged {
            domain: domain.clone(),
        }
    })?;
    let request = BackupAdapterRestoreRequest::new(
        BackupAdapterInspectRequest::new(descriptor, source.source_schema_version(), payloads),
        &prepared.preview,
    );
    let outcome = adapter
        .restore(request)
        .map_err(|error| RestoreAdapterError::AdapterFailed {
            domain: domain.clone(),
            adapter: prepared.adapter.clone(),
            error,
        })?;
    let evidence_matches = match (&prepared.participation, &outcome) {
        (_, BackupAdapterRestoreOutcome::Verified { evidence }) => {
            evidence == prepared.preview.target_evidence()
        }
        (
            BackupAdapterRestoreParticipation::FailureAtomic,
            BackupAdapterRestoreOutcome::RolledBack { evidence },
        ) => prepared.preview.current_evidence() == Some(evidence),
        (
            BackupAdapterRestoreParticipation::FailureAtomic,
            BackupAdapterRestoreOutcome::RecoveryRequired,
        ) => true,
        (
            BackupAdapterRestoreParticipation::Separate
            | BackupAdapterRestoreParticipation::Excluded(_),
            BackupAdapterRestoreOutcome::RolledBack { .. }
            | BackupAdapterRestoreOutcome::RecoveryRequired,
        ) => false,
    };
    if !evidence_matches {
        return Err(RestoreAdapterError::OutcomeEvidenceMismatch {
            domain: domain.clone(),
        });
    }
    Ok(RestoreAdapterReceipt::new(
        domain.clone(),
        prepared.adapter.clone(),
        prepared.participation.clone(),
        prepared.confirmation_digest.clone(),
        outcome,
    ))
}
