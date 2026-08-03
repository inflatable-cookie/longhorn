use crate::{
    BackupAdapterGroupedApplyKind, BackupAdapterGroupedApplyRequest, BackupAdapterGroupedRestore,
    BackupAdapterGroupedVerifyRequest, BackupAdapterRestoreParticipation, BackupCatalog,
    ConfigStore, backup::CatalogDecision, coordination::CoordinationGuard,
};

use super::{
    journal::{self, GroupedJournalEntry, GroupedJournalPhase, GroupedRestoreJournal},
    types::{
        RestoreAdapterGroupReceiptEntry, RestoreAdapterGroupRecoveryError,
        RestoreAdapterGroupRecoveryOutcome, RestoreAdapterGroupRecoveryReceipt,
    },
};

pub(crate) fn recover(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    timeout: std::time::Duration,
) -> Result<RestoreAdapterGroupRecoveryReceipt, RestoreAdapterGroupRecoveryError> {
    let guard =
        store
            .coordinator
            .acquire(timeout)
            .map_err(|error| RestoreAdapterGroupRecoveryError {
                path: error.lock_path.clone(),
                domain: None,
                detail: error.to_string(),
            })?;
    recover_guarded(store, catalog, &guard)
}

pub(super) fn recover_guarded(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    _guard: &CoordinationGuard<'_>,
) -> Result<RestoreAdapterGroupRecoveryReceipt, RestoreAdapterGroupRecoveryError> {
    let authority = store.coordinator.authority_root();
    let Some(mut state) = journal::load(authority).map_err(|error| recovery_error(None, error))?
    else {
        return Ok(RestoreAdapterGroupRecoveryReceipt {
            outcome: RestoreAdapterGroupRecoveryOutcome::NoRecoveryNeeded,
            entries: Vec::new(),
        });
    };
    let entries = receipt_entries(&state);

    if matches!(
        state.phase,
        GroupedJournalPhase::Succeeded | GroupedJournalPhase::RolledBack
    ) {
        journal::cleanup(authority).map_err(|error| recovery_error(None, error))?;
        return Ok(RestoreAdapterGroupRecoveryReceipt {
            outcome: RestoreAdapterGroupRecoveryOutcome::TerminalCleanup,
            entries,
        });
    }

    state.phase = GroupedJournalPhase::RollingBack;
    journal::publish(authority, &state).map_err(|error| recovery_error(None, error))?;
    if let Err(error) = rollback_all(store, catalog, authority, &state) {
        state.phase = GroupedJournalPhase::RecoveryRequired;
        let marker_error = journal::publish(authority, &state).err();
        return Err(match marker_error {
            Some(marker) => RestoreAdapterGroupRecoveryError {
                detail: format!(
                    "{}; cannot persist recovery-required marker: {marker}",
                    error.detail
                ),
                ..error
            },
            None => error,
        });
    }
    state.phase = GroupedJournalPhase::RolledBack;
    journal::publish(authority, &state).map_err(|error| recovery_error(None, error))?;
    journal::cleanup(authority).map_err(|error| recovery_error(None, error))?;
    Ok(RestoreAdapterGroupRecoveryReceipt {
        outcome: RestoreAdapterGroupRecoveryOutcome::RolledBack,
        entries,
    })
}

pub(super) fn rollback_all(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    authority: &std::path::Path,
    state: &GroupedRestoreJournal,
) -> Result<(), RestoreAdapterGroupRecoveryError> {
    for entry in state.entries.iter().rev() {
        let (descriptor, adapter) = resolve_adapter(store, catalog, entry)?;
        let payloads = journal::read_payloads(authority, &entry.rollback_payloads)
            .map_err(|error| recovery_error(Some(entry.domain.clone()), error))?;
        adapter
            .apply(BackupAdapterGroupedApplyRequest::new(
                descriptor,
                BackupAdapterGroupedApplyKind::Rollback,
                &payloads,
                &entry.rollback_evidence,
            ))
            .map_err(|error| recovery_error(Some(entry.domain.clone()), error))?;
    }
    for entry in &state.entries {
        let (descriptor, adapter) = resolve_adapter(store, catalog, entry)?;
        let observed = adapter
            .verify(BackupAdapterGroupedVerifyRequest::new(
                descriptor,
                BackupAdapterGroupedApplyKind::Rollback,
                &entry.rollback_evidence,
            ))
            .map_err(|error| recovery_error(Some(entry.domain.clone()), error))?;
        if observed != entry.rollback_evidence {
            return Err(recovery_error(
                Some(entry.domain.clone()),
                "grouped adapter rollback evidence mismatch",
            ));
        }
    }
    Ok(())
}

fn receipt_entries(state: &GroupedRestoreJournal) -> Vec<RestoreAdapterGroupReceiptEntry> {
    state
        .entries
        .iter()
        .map(|entry| RestoreAdapterGroupReceiptEntry {
            domain: entry.domain.clone(),
            target_evidence: entry.target_evidence.clone(),
            rollback_evidence: entry.rollback_evidence.clone(),
        })
        .collect()
}

pub(super) fn resolve_adapter<'store, 'catalog>(
    store: &'store ConfigStore,
    catalog: &'catalog BackupCatalog<'_>,
    entry: &GroupedJournalEntry,
) -> Result<
    (
        &'store crate::DomainDescriptor,
        &'catalog dyn BackupAdapterGroupedRestore,
    ),
    RestoreAdapterGroupRecoveryError,
> {
    let descriptor = store.registered_descriptor(&entry.domain).ok_or_else(|| {
        recovery_error(
            Some(entry.domain.clone()),
            "grouped journal domain is not registered",
        )
    })?;
    if journal::descriptor_digest(descriptor) != entry.descriptor_digest {
        return Err(recovery_error(
            Some(entry.domain.clone()),
            "registered descriptor changed after grouped staging",
        ));
    }
    let adapter = match catalog.decision(descriptor) {
        Some(CatalogDecision::Custom(adapter))
            if adapter.id().as_str() == entry.adapter
                && adapter.capabilities().restore()
                    == &BackupAdapterRestoreParticipation::GroupedFailureAtomic =>
        {
            adapter.grouped_restore()
        }
        _ => None,
    }
    .ok_or_else(|| {
        recovery_error(
            Some(entry.domain.clone()),
            "exact grouped restore adapter is unavailable",
        )
    })?;
    Ok((descriptor, adapter))
}

fn recovery_error(
    domain: Option<longhorn_core::DomainId>,
    error: impl std::fmt::Display,
) -> RestoreAdapterGroupRecoveryError {
    RestoreAdapterGroupRecoveryError {
        path: std::path::PathBuf::from(".longhorn/grouped-adapter-restore/journal.json"),
        domain,
        detail: error.to_string(),
    }
}
