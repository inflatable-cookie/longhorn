//! Validation, rollback, and failure helpers for grouped restore execution.

use std::collections::BTreeSet;

use crate::{BackupAdapterStateEvidence, BackupArchiveInspection, BackupCatalog, ConfigStore};

use super::super::{
    journal::{self, GroupedJournalPhase},
    planning::group_confirmation_digest,
    recovery,
    types::{RestoreAdapterGroupError, RestoreAdapterGroupExecutionStage, RestoreAdapterGroupPlan},
};
use crate::backup::restore::{RestoreFailureTerminal, RestoreInspection};

pub(crate) fn validate_plan(
    archive: &BackupArchiveInspection,
    inspection: &RestoreInspection,
    plan: &RestoreAdapterGroupPlan,
    confirmation: &crate::Sha256Digest,
) -> Result<(), RestoreAdapterGroupError> {
    if !inspection.identity.is_compatible() {
        return Err(validation_failure("restore identity does not match"));
    }
    if archive.archive_sha256() != inspection.archive_sha256()
        || archive.archive_sha256() != &plan.archive_sha256
    {
        return Err(validation_failure(
            "restore archive changed after inspection",
        ));
    }
    if confirmation != &plan.confirmation_digest
        || group_confirmation_digest(&plan.archive_sha256, &plan.entries)
            != plan.confirmation_digest
    {
        return Err(validation_failure(
            "group confirmation does not match the plan",
        ));
    }
    Ok(())
}

pub(crate) fn validate_payload_set(
    payloads: &[crate::BackupAdapterPayload],
) -> Result<usize, RestoreAdapterGroupError> {
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    for payload in payloads {
        if !paths.insert(payload.relative_path().as_str()) {
            return Err(validation_failure(
                "grouped adapter stage repeats a payload path",
            ));
        }
        total = total
            .checked_add(payload.bytes().len())
            .ok_or_else(|| validation_failure("grouped adapter stage length overflow"))?;
    }
    Ok(total)
}

pub(crate) fn validate_state_payload_set(
    domain: &longhorn_core::DomainId,
    evidence: &BackupAdapterStateEvidence,
    payloads: &[crate::BackupAdapterPayload],
) -> Result<usize, RestoreAdapterGroupError> {
    let valid = match evidence {
        BackupAdapterStateEvidence::Absent => payloads.is_empty(),
        BackupAdapterStateEvidence::Present { .. } => !payloads.is_empty(),
    };
    if !valid {
        return Err(stage_limit_failure(
            domain,
            "grouped adapter evidence contradicts payload presence",
        ));
    }
    validate_payload_set(payloads)
}

pub(crate) fn rollback_after_failure(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    state: &mut journal::GroupedRestoreJournal,
    stage: RestoreAdapterGroupExecutionStage,
    domain: Option<longhorn_core::DomainId>,
    original: impl std::fmt::Display,
) -> RestoreAdapterGroupError {
    let authority = store.coordinator.authority_root();
    state.phase = GroupedJournalPhase::RollingBack;
    let marker = journal::publish(authority, state);
    let rollback = recovery::rollback_all(store, catalog, authority, state);
    if let Err(rollback) = rollback {
        state.phase = GroupedJournalPhase::RecoveryRequired;
        let marker_failure = journal::publish(authority, state).err();
        let mut detail = format!("{original}; rollback failed: {}", rollback.detail());
        if let Some(marker) = marker_failure {
            detail.push_str(&format!("; recovery marker failed: {marker}"));
        }
        return failure(
            RestoreAdapterGroupExecutionStage::Rollback,
            rollback.domain().cloned().or(domain),
            RestoreFailureTerminal::RecoveryRequired,
            detail,
        );
    }
    state.phase = GroupedJournalPhase::RolledBack;
    let cleanup = journal::publish(authority, state).and_then(|()| journal::cleanup(authority));
    let mut detail = original.to_string();
    if let Err(marker) = marker {
        detail.push_str(&format!("; rollback phase marker failed: {marker}"));
    }
    if let Err(cleanup) = cleanup {
        detail.push_str(&format!("; terminal cleanup failed: {cleanup}"));
    }
    failure(stage, domain, RestoreFailureTerminal::RolledBack, detail)
}

pub(crate) fn validation_failure(detail: impl std::fmt::Display) -> RestoreAdapterGroupError {
    failure(
        RestoreAdapterGroupExecutionStage::ValidatePlan,
        None,
        RestoreFailureTerminal::NoLiveMutation,
        detail,
    )
}

pub(crate) fn stage_limit_failure(
    domain: &longhorn_core::DomainId,
    detail: impl std::fmt::Display,
) -> RestoreAdapterGroupError {
    failure(
        RestoreAdapterGroupExecutionStage::Stage,
        Some(domain.clone()),
        RestoreFailureTerminal::NoLiveMutation,
        detail,
    )
}

pub(crate) fn failure(
    stage: RestoreAdapterGroupExecutionStage,
    domain: Option<longhorn_core::DomainId>,
    terminal: RestoreFailureTerminal,
    detail: impl std::fmt::Display,
) -> RestoreAdapterGroupError {
    RestoreAdapterGroupError {
        stage,
        domain,
        terminal,
        detail: detail.to_string(),
    }
}
