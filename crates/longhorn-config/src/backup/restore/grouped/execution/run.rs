//! Grouped adapter restore execution.

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

use super::super::{
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

use super::{
    failure, prepare_domains, rollback_after_failure, validate_plan,
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

    let (prepared_domains, _staged_total) = prepare_domains(
        store,
        catalog,
        archive,
        inspection,
        plan,
        &options,
    )?;

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
        if let Err(error) = journal::cleanup(authority) {
            longhorn_core::report_best_effort_failure(
                "config.restore.grouped-journal-cleanup",
                error,
            );
        }
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


