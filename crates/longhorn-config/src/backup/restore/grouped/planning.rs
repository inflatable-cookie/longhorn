use std::collections::BTreeSet;

use longhorn_core::DomainId;
use serde_json::json;

use crate::{BackupAdapterRestoreParticipation, Sha256Digest};

use super::{
    super::RestoreInspection,
    types::{RestoreAdapterGroupPlan, RestoreAdapterGroupPlanEntry, RestoreAdapterGroupPlanError},
};

pub(crate) fn plan(
    inspection: &RestoreInspection,
    domains: impl IntoIterator<Item = DomainId>,
) -> Result<RestoreAdapterGroupPlan, RestoreAdapterGroupPlanError> {
    if !inspection.identity.is_compatible() {
        return Err(RestoreAdapterGroupPlanError::IdentityMismatch);
    }
    let mut selected = BTreeSet::new();
    for domain in domains {
        if !selected.insert(domain.clone()) {
            return Err(RestoreAdapterGroupPlanError::DuplicateDomain { domain });
        }
    }
    if selected.is_empty() {
        return Err(RestoreAdapterGroupPlanError::Empty);
    }

    let mut entries = Vec::with_capacity(selected.len());
    for domain in selected {
        let prepared = inspection.custom_prepared.get(&domain).ok_or_else(|| {
            RestoreAdapterGroupPlanError::UnknownDomain {
                domain: domain.clone(),
            }
        })?;
        if prepared.participation != BackupAdapterRestoreParticipation::GroupedFailureAtomic {
            return Err(RestoreAdapterGroupPlanError::GroupedParticipationRequired {
                domain,
                adapter: prepared.adapter.clone(),
            });
        }
        entries.push(RestoreAdapterGroupPlanEntry {
            domain,
            adapter: prepared.adapter.clone(),
            adapter_confirmation: prepared.confirmation_digest.clone(),
            target_evidence: prepared.preview.target_evidence().clone(),
            current_evidence: prepared.preview.current_evidence().cloned(),
        });
    }

    let confirmation_digest = group_confirmation_digest(inspection.archive_sha256(), &entries);
    Ok(RestoreAdapterGroupPlan {
        archive_sha256: inspection.archive_sha256().clone(),
        entries,
        confirmation_digest,
    })
}

pub(super) fn group_confirmation_digest(
    archive_sha256: &Sha256Digest,
    entries: &[RestoreAdapterGroupPlanEntry],
) -> Sha256Digest {
    let entries = entries
        .iter()
        .map(|entry| {
            json!({
                "domain": entry.domain.as_str(),
                "adapter": entry.adapter.as_str(),
                "adapterConfirmation": entry.adapter_confirmation.as_str(),
                "targetEvidence": entry.target_evidence.as_str(),
                "currentEvidence": entry.current_evidence.as_ref().map(Sha256Digest::as_str),
            })
        })
        .collect::<Vec<_>>();
    let canonical = serde_json::to_vec(&json!({
        "archiveSha256": archive_sha256.as_str(),
        "entries": entries,
    }))
    .expect("grouped restore confirmation form is serializable");
    Sha256Digest::from_bytes(&canonical)
}
