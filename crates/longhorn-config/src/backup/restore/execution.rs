use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    BackupCatalog, BackupKind, BackupPublicationOptions, BackupScope, ConfigStore,
    DurabilityRequirement, RestoreAction, RestoreExecutionError, RestoreExecutionOptions,
    RestoreExecutionReceipt, RestoreExecutionStage, RestoreFailureTerminal, RestoreStaging,
    backup::{capture, encode_backup_archive, publish_operational_backup},
    coordination::CoordinationGuard,
};

use super::{
    journal::{self, JournalPhase, JournalSeed, RestoreJournal},
    live_io, recovery, transaction,
};

static OPERATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn execute(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    staging: RestoreStaging,
    options: RestoreExecutionOptions,
) -> Result<RestoreExecutionReceipt, RestoreExecutionError> {
    if options.safety_backup.metadata.kind() != BackupKind::PreRestore {
        return Err(failure(
            RestoreExecutionStage::ValidateSafetyBackup,
            None,
            RestoreFailureTerminal::NoLiveMutation,
            "safety backup metadata must use pre-restore kind",
        ));
    }
    let guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(|error| {
            failure(
                RestoreExecutionStage::RecoverPrevious,
                None,
                RestoreFailureTerminal::NoLiveMutation,
                error,
            )
        })?;
    recovery::recover_guarded(store, &guard).map_err(|error| {
        let domain = error.domain.clone();
        failure(
            RestoreExecutionStage::RecoverPrevious,
            domain,
            RestoreFailureTerminal::RecoveryRequired,
            error,
        )
    })?;
    execute_guarded(store, catalog, staging, options, &guard)
}

pub(super) fn execute_guarded(
    store: &ConfigStore,
    catalog: &BackupCatalog<'_>,
    staging: RestoreStaging,
    options: RestoreExecutionOptions,
    guard: &CoordinationGuard<'_>,
) -> Result<RestoreExecutionReceipt, RestoreExecutionError> {
    let authority = store.coordinator.authority_root();
    let rollback =
        transaction::capture_rollback(store, &staging, options.safety_backup.capture.limits)?;
    let selected =
        BackupScope::selected(staging.domains.iter().map(|domain| domain.domain.clone()))
            .expect("restore staging is non-empty");
    let safety = capture::capture_guarded(
        store,
        catalog,
        &selected,
        options.safety_backup.metadata.clone(),
        options.safety_backup.capture,
        guard,
    )
    .map_err(|error| {
        failure(
            RestoreExecutionStage::CaptureSafetyBackup,
            None,
            RestoreFailureTerminal::NoLiveMutation,
            error,
        )
    })?;
    let archive =
        encode_backup_archive(&safety, options.safety_backup.archive_limits).map_err(|error| {
            failure(
                RestoreExecutionStage::EncodeSafetyBackup,
                None,
                RestoreFailureTerminal::NoLiveMutation,
                error,
            )
        })?;
    let safety_receipt = publish_operational_backup(
        &options.safety_backup.root,
        &options.safety_backup.file_name,
        &archive,
        BackupPublicationOptions::new(
            DurabilityRequirement::Durable,
            options.safety_backup.archive_limits,
        ),
    )
    .map_err(|error| {
        failure(
            RestoreExecutionStage::PublishSafetyBackup,
            None,
            RestoreFailureTerminal::NoLiveMutation,
            error,
        )
    })?;

    let operation_id = format!(
        "restore-{}-{}",
        std::process::id(),
        OPERATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut state = journal::persist_rollback(
        authority,
        &staging.domains,
        &rollback.entries,
        &rollback.descriptors,
        JournalSeed {
            operation_id,
            plan_digest: staging.plan_digest.clone(),
            safety_path: safety_receipt.path.clone(),
            safety_sha256: safety_receipt.archive_sha256.clone(),
        },
    )
    .and_then(|state| {
        journal::publish(authority, &state)?;
        Ok(state)
    })
    .map_err(|error| {
        if let Err(error) = journal::cleanup(authority) {
            longhorn_core::report_best_effort_failure("config.restore.journal-cleanup", error);
        }
        failure(
            RestoreExecutionStage::PublishJournal,
            None,
            RestoreFailureTerminal::NoLiveMutation,
            error,
        )
    })?;

    if let Err(error) =
        transaction::verify_published_safety(&safety_receipt, options.safety_backup.archive_limits)
    {
        return Err(rollback_after_failure(
            store,
            &mut state,
            RestoreExecutionStage::PublishSafetyBackup,
            None,
            error,
        ));
    }

    state.phase = JournalPhase::Applying;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            &mut state,
            RestoreExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    for staged in &staging.domains {
        if staged.action == RestoreAction::Unchanged {
            continue;
        }
        let entry = state
            .entries
            .iter()
            .find(|entry| entry.domain == staged.domain)
            .expect("journal mirrors complete staging");
        let file = match recovery::resolve_entry(store, entry) {
            Ok(file) => file,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    &mut state,
                    RestoreExecutionStage::PublishTarget,
                    Some(staged.domain.clone()),
                    error,
                ));
            }
        };
        if let Err(error) = live_io::publish_state(&file, staged.bytes.as_deref()) {
            return Err(rollback_after_failure(
                store,
                &mut state,
                RestoreExecutionStage::PublishTarget,
                Some(staged.domain.clone()),
                error,
            ));
        }
    }

    state.phase = JournalPhase::Verifying;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            &mut state,
            RestoreExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    for entry in state.entries.clone() {
        let file = match recovery::resolve_entry(store, &entry) {
            Ok(file) => file,
            Err(error) => {
                return Err(rollback_after_failure(
                    store,
                    &mut state,
                    RestoreExecutionStage::VerifyTarget,
                    Some(entry.domain.clone()),
                    error,
                ));
            }
        };
        if let Err(error) = live_io::verify_state(&file, &entry.target) {
            return Err(rollback_after_failure(
                store,
                &mut state,
                RestoreExecutionStage::VerifyTarget,
                Some(entry.domain.clone()),
                error,
            ));
        }
    }

    state.phase = JournalPhase::Succeeded;
    if let Err(error) = journal::publish(authority, &state) {
        return Err(rollback_after_failure(
            store,
            &mut state,
            RestoreExecutionStage::PublishJournal,
            None,
            error,
        ));
    }
    if let Err(error) = journal::cleanup(authority) {
        longhorn_core::report_best_effort_failure("config.restore.journal-cleanup", error);
    }
    Ok(receipt(staging, safety_receipt))
}

fn rollback_after_failure(
    store: &ConfigStore,
    state: &mut RestoreJournal,
    stage: RestoreExecutionStage,
    domain: Option<longhorn_core::DomainId>,
    original: impl std::fmt::Display,
) -> RestoreExecutionError {
    let authority = store.coordinator.authority_root();
    state.phase = JournalPhase::RollingBack;
    let marker = journal::publish(authority, state);
    let rollback = recovery::rollback_all(store, authority, state);
    if let Err(rollback) = rollback {
        state.phase = JournalPhase::RecoveryRequired;
        let marker_failure = journal::publish(authority, state).err();
        let mut detail = format!("{original}; rollback failed: {}", rollback.detail);
        if let Some(marker) = marker_failure {
            detail.push_str(&format!("; recovery marker failed: {marker}"));
        }
        return failure(
            RestoreExecutionStage::Rollback,
            rollback.domain.or(domain),
            RestoreFailureTerminal::RecoveryRequired,
            detail,
        );
    }
    state.phase = JournalPhase::RolledBack;
    let terminal = journal::publish(authority, state);
    let cleanup = terminal.and_then(|()| journal::cleanup(authority));
    let mut detail = original.to_string();
    if let Err(marker) = marker {
        detail.push_str(&format!("; rollback phase marker failed: {marker}"));
    }
    if let Err(cleanup) = cleanup {
        detail.push_str(&format!("; terminal cleanup failed: {cleanup}"));
    }
    failure(stage, domain, RestoreFailureTerminal::RolledBack, detail)
}

fn receipt(
    staging: RestoreStaging,
    safety_backup: crate::BackupPublicationReceipt,
) -> RestoreExecutionReceipt {
    let mut restored = Vec::new();
    let mut deleted = Vec::new();
    let mut migrated = Vec::new();
    let mut unchanged = Vec::new();
    for staged in staging.domains {
        match staged.action {
            RestoreAction::Create | RestoreAction::Replace => restored.push(staged.domain),
            RestoreAction::Delete => deleted.push(staged.domain),
            RestoreAction::Migrate => {
                migrated.push(staged.domain.clone());
                restored.push(staged.domain);
            }
            RestoreAction::Unchanged => unchanged.push(staged.domain),
        }
    }
    RestoreExecutionReceipt {
        plan_digest: staging.plan_digest,
        safety_backup,
        restored,
        deleted,
        migrated,
        unchanged,
    }
}

pub(super) fn failure(
    stage: RestoreExecutionStage,
    domain: Option<longhorn_core::DomainId>,
    terminal: RestoreFailureTerminal,
    detail: impl std::fmt::Display,
) -> RestoreExecutionError {
    RestoreExecutionError {
        stage,
        domain,
        terminal,
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
