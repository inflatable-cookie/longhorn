use std::collections::BTreeSet;

use longhorn_core::DomainId;
use serde_json::json;

use crate::{BackupAdapterRestoreParticipation, BackupAdapterStateEvidence, Sha256Digest};

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
            rollback_evidence: prepared.preview.current_evidence().clone(),
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
                "targetEvidence": evidence_confirmation(&entry.target_evidence),
                "rollbackEvidence": evidence_confirmation(&entry.rollback_evidence),
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

pub(crate) fn evidence_confirmation(evidence: &BackupAdapterStateEvidence) -> serde_json::Value {
    match evidence {
        BackupAdapterStateEvidence::Absent => json!({"state": "absent"}),
        BackupAdapterStateEvidence::Present { sha256 } => {
            json!({"state": "present", "sha256": sha256.as_str()})
        }
    }
}
