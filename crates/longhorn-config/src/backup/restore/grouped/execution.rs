use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    BackupAdapterGroupedApplyKind, BackupAdapterGroupedApplyRequest,
    BackupAdapterGroupedStageRequest, BackupAdapterGroupedVerifyRequest,
    BackupAdapterInspectRequest, BackupAdapterRestoreParticipation, BackupAdapterStateEvidence,
    BackupArchiveInspection, BackupCatalog, ConfigStore, backup::CatalogDecision,
};

use super::{
    journal::{self, GroupedJournalPhase, PreparedGroupedDomain},
    planning::group_confirmation_digest,
    recovery,
    types::{
        RestoreAdapterGroupError, RestoreAdapterGroupExecutionOptions,
        RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupExecutionStage,
        RestoreAdapterGroupPlan,
    },
};
use crate::backup::restore::{
    RestoreFailureTerminal, RestoreInspection, inspection::payloads_for_adapter,
};

static OPERATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn execute(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    archive: &BackupArchiveInspection,
    inspection: &RestoreInspection,
    plan: &RestoreAdapterGroupPlan,
    confirmation: &crate::Sha256Digest,
    options: RestoreAdapterGroupExecutionOptions,
) -> Result<RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupError> {
    validate_plan(archive, inspection, plan, confirmation)?;
    let guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(|error| {
            failure(
                RestoreAdapterGroupExecutionStage::RecoverPrevious,
                None,
                RestoreFailureTerminal::NoLiveMutation,
                error,
            )
        })?;
    recovery::recover_guarded(store, catalog, &guard).map_err(|error| {
        failure(
            RestoreAdapterGroupExecutionStage::RecoverPrevious,
            error.domain().cloned(),
            RestoreFailureTerminal::RecoveryRequired,
            error,
        )
    })?;
    crate::backup::restore::recovery::recover_guarded(store, &guard).map_err(|error| {
        failure(
            RestoreAdapterGroupExecutionStage::RecoverPrevious,
            error.domain.clone(),
            RestoreFailureTerminal::RecoveryRequired,
            error,
        )
    })?;

    let mut prepared_domains = Vec::with_capacity(plan.entries.len());
    let mut staged_total = 0usize;
    for planned in &plan.entries {
        let descriptor = store
            .registered_descriptor(&planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore domain is no longer registered",
                )
            })?;
        let source = inspection
            .manifest
            .domains()
            .iter()
            .find(|source| source.domain() == &planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore source disappeared after inspection",
                )
            })?;
        let inspected = inspection
            .custom_prepared
            .get(&planned.domain)
            .ok_or_else(|| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore target disappeared after inspection",
                )
            })?;
        let adapter = match catalog.decision(descriptor) {
            Some(CatalogDecision::Custom(adapter))
                if adapter.id() == &planned.adapter
                    && source.adapter() == adapter.id().as_str()
                    && adapter.capabilities().restore()
                        == &BackupAdapterRestoreParticipation::GroupedFailureAtomic
                    && inspected.adapter == planned.adapter
                    && inspected.participation
                        == BackupAdapterRestoreParticipation::GroupedFailureAtomic
                    && inspected.confirmation_digest == planned.adapter_confirmation =>
            {
                adapter
            }
            _ => {
                return Err(failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    "grouped restore descriptor or adapter changed",
                ));
            }
        };
        let grouped = adapter.grouped_restore().ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "adapter declares grouped participation without grouped protocol",
            )
        })?;

        let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "verified archive payload is unavailable",
            )
        })?;
        let fresh_preview = adapter
            .inspect(BackupAdapterInspectRequest::new(
                descriptor,
                source.state(),
                source.source_schema_version(),
                payloads,
            ))
            .map_err(|error| {
                failure(
                    RestoreAdapterGroupExecutionStage::Reinspect,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    error,
                )
            })?;
        if fresh_preview.target_evidence() != &planned.target_evidence
            || fresh_preview.current_evidence() != &planned.rollback_evidence
            || fresh_preview != inspected.preview
        {
            return Err(failure(
                RestoreAdapterGroupExecutionStage::Reinspect,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "grouped restore semantic preview changed",
            ));
        }

        let payloads = payloads_for_adapter(archive, source).ok_or_else(|| {
            failure(
                RestoreAdapterGroupExecutionStage::Stage,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "verified archive payload is unavailable",
            )
        })?;
        let stage = grouped
            .stage(BackupAdapterGroupedStageRequest::new(
                BackupAdapterInspectRequest::new(
                    descriptor,
                    source.state(),
                    source.source_schema_version(),
                    payloads,
                ),
                &fresh_preview,
                options.limits,
            ))
            .map_err(|error| {
                failure(
                    RestoreAdapterGroupExecutionStage::Stage,
                    Some(planned.domain.clone()),
                    RestoreFailureTerminal::NoLiveMutation,
                    error,
                )
            })?;
        if stage.target_evidence() != &planned.target_evidence
            || stage.rollback_evidence() != &planned.rollback_evidence
        {
            return Err(failure(
                RestoreAdapterGroupExecutionStage::Stage,
                Some(planned.domain.clone()),
                RestoreFailureTerminal::NoLiveMutation,
                "grouped adapter stage evidence contradicts the confirmed preview",
            ));
        }
        let domain_bytes = validate_state_payload_set(
            &planned.domain,
            stage.target_evidence(),
            stage.target_payloads(),
        )?
        .checked_add(validate_state_payload_set(
            &planned.domain,
            stage.rollback_evidence(),
            stage.rollback_payloads(),
        )?)
        .ok_or_else(|| stage_limit_failure(&planned.domain, "stage byte length overflow"))?;
        if domain_bytes > options.limits.max_domain_bytes() {
            return Err(stage_limit_failure(
                &planned.domain,
                format!(
                    "stage bytes {domain_bytes} exceed domain limit {}",
                    options.limits.max_domain_bytes()
                ),
            ));
        }
        staged_total = staged_total
            .checked_add(domain_bytes)
            .ok_or_else(|| stage_limit_failure(&planned.domain, "total stage length overflow"))?;
        if staged_total > options.limits.max_total_bytes() {
            return Err(stage_limit_failure(
                &planned.domain,
                format!(
                    "stage bytes {staged_total} exceed total limit {}",
                    options.limits.max_total_bytes()
                ),
            ));
        }
        prepared_domains.push(PreparedGroupedDomain {
            domain: planned.domain.clone(),
            adapter: planned.adapter.clone(),
            descriptor_digest: journal::descriptor_digest(descriptor),
            target_evidence: planned.target_evidence.clone(),
            rollback_evidence: planned.rollback_evidence.clone(),
            target_payloads: stage.target_payloads().to_vec(),
            rollback_payloads: stage.rollback_payloads().to_vec(),
        });
    }

    let authority = store.coordinator.authority_root();
    let operation_id = format!(
        "grouped-restore-{}-{}",
        std::process::id(),
        OPERATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut state = journal::persist_prepared(
        authority,
        operation_id,
        plan.archive_sha256.clone(),
        plan.confirmation_digest.clone(),
        &prepared_domains,
    )
    .map_err(|error| {
        let _ = journal::cleanup(authority);
        failure(
            RestoreAdapterGroupExecutionStage::PublishJournal,
            None,
            RestoreFailureTerminal::NoLiveMutation,
            error,
        )
    })?;

    state.phase = GroupedJournalPhase::Applying;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            catalog,
            &mut state,
            RestoreAdapterGroupExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    for entry in state.entries.clone() {
        let (descriptor, adapter) = match recovery::resolve_adapter(store, catalog, &entry) {
            Ok(resolved) => resolved,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    catalog,
                    &mut state,
                    RestoreAdapterGroupExecutionStage::ApplyTarget,
                    Some(entry.domain.clone()),
                    error,
                ));
            }
        };
        let payloads = match journal::read_payloads(authority, &entry.target_payloads) {
            Ok(payloads) => payloads,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    catalog,
                    &mut state,
                    RestoreAdapterGroupExecutionStage::ApplyTarget,
                    Some(entry.domain.clone()),
                    error,
                ));
            }
        };
        if let Err(error) = adapter.apply(BackupAdapterGroupedApplyRequest::new(
            descriptor,
            BackupAdapterGroupedApplyKind::Target,
            &payloads,
            &entry.target_evidence,
        )) {
            return Err(rollback_after_failure(
                store,
                catalog,
                &mut state,
                RestoreAdapterGroupExecutionStage::ApplyTarget,
                Some(entry.domain.clone()),
                error,
            ));
        }
    }

    state.phase = GroupedJournalPhase::Verifying;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            catalog,
            &mut state,
            RestoreAdapterGroupExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    for entry in state.entries.clone() {
        let (descriptor, adapter) = match recovery::resolve_adapter(store, catalog, &entry) {
            Ok(resolved) => resolved,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    catalog,
                    &mut state,
                    RestoreAdapterGroupExecutionStage::VerifyTarget,
                    Some(entry.domain.clone()),
                    error,
                ));
            }
        };
        let observed = match adapter.verify(BackupAdapterGroupedVerifyRequest::new(
            descriptor,
            BackupAdapterGroupedApplyKind::Target,
            &entry.target_evidence,
        )) {
            Ok(observed) => observed,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    catalog,
                    &mut state,
                    RestoreAdapterGroupExecutionStage::VerifyTarget,
                    Some(entry.domain.clone()),
                    error,
                ));
            }
        };
        if observed != entry.target_evidence {
            return Err(rollback_after_failure(
                store,
                catalog,
                &mut state,
                RestoreAdapterGroupExecutionStage::VerifyTarget,
                Some(entry.domain.clone()),
                "grouped adapter target evidence mismatch",
            ));
        }
    }

    state.phase = GroupedJournalPhase::Succeeded;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            catalog,
            &mut state,
            RestoreAdapterGroupExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    journal::cleanup(authority).map_err(|error| {
        failure(
            RestoreAdapterGroupExecutionStage::Cleanup,
            None,
            RestoreFailureTerminal::RecoveryRequired,
            error,
        )
    })?;
    Ok(RestoreAdapterGroupExecutionReceipt {
        confirmation_digest: plan.confirmation_digest.clone(),
        entries: plan.entries.iter().map(Into::into).collect(),
    })
}

fn validate_plan(
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

fn validate_payload_set(
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

fn validate_state_payload_set(
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

fn rollback_after_failure(
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

fn validation_failure(detail: impl std::fmt::Display) -> RestoreAdapterGroupError {
    failure(
        RestoreAdapterGroupExecutionStage::ValidatePlan,
        None,
        RestoreFailureTerminal::NoLiveMutation,
        detail,
    )
}

fn stage_limit_failure(
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

fn failure(
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
