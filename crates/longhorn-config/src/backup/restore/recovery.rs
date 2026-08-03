use std::path::{Path, PathBuf};

use crate::{ConfigStore, DomainLocation, ResolvedFile, coordination::CoordinationGuard};

use super::{
    RestoreRecoveryError, RestoreRecoveryOutcome, RestoreRecoveryReceipt,
    journal::{self, JournalEntry, JournalPhase, RestoreJournal},
    live_io,
};

pub(crate) fn recover(
    store: &ConfigStore,
    timeout: std::time::Duration,
) -> Result<RestoreRecoveryReceipt, RestoreRecoveryError> {
    let guard = store
        .coordinator
        .acquire(timeout)
        .map_err(|error| RestoreRecoveryError {
            path: error.lock_path.clone(),
            domain: None,
            detail: error.to_string(),
        })?;
    recover_guarded(store, &guard)
}

pub(crate) fn recover_guarded(
    store: &ConfigStore,
    _guard: &CoordinationGuard<'_>,
) -> Result<RestoreRecoveryReceipt, RestoreRecoveryError> {
    let authority = store.coordinator.authority_root();
    if super::grouped::blocks_ordinary_recovery(authority) {
        return Err(RestoreRecoveryError {
            path: super::grouped::journal_path(authority),
            domain: None,
            detail: "grouped adapter recovery requires the exact adapter catalogue".into(),
        });
    }
    let Some(mut state) = journal::load(authority)
        .map_err(|error| recovery_error(journal::journal_path(authority), None, error))?
    else {
        return Ok(RestoreRecoveryReceipt {
            outcome: RestoreRecoveryOutcome::NoRecoveryNeeded,
            domains: Vec::new(),
        });
    };
    let domains = state
        .entries
        .iter()
        .map(|entry| entry.domain.clone())
        .collect::<Vec<_>>();

    if matches!(
        state.phase,
        JournalPhase::Succeeded | JournalPhase::RolledBack
    ) {
        journal::cleanup(authority)
            .map_err(|error| recovery_error(journal::journal_path(authority), None, error))?;
        return Ok(RestoreRecoveryReceipt {
            outcome: RestoreRecoveryOutcome::TerminalCleanup,
            domains,
        });
    }

    state.phase = JournalPhase::RollingBack;
    journal::publish(authority, &state)
        .map_err(|error| recovery_error(journal::journal_path(authority), None, error))?;
    if let Err(error) = rollback_all(store, authority, &state) {
        state.phase = JournalPhase::RecoveryRequired;
        let marker_error = journal::publish(authority, &state).err();
        return Err(match marker_error {
            Some(marker_error) => RestoreRecoveryError {
                detail: format!(
                    "{}; cannot persist recovery-required marker: {marker_error}",
                    error.detail
                ),
                ..error
            },
            None => error,
        });
    }
    state.phase = JournalPhase::RolledBack;
    journal::publish(authority, &state)
        .map_err(|error| recovery_error(journal::journal_path(authority), None, error))?;
    journal::cleanup(authority)
        .map_err(|error| recovery_error(journal::journal_path(authority), None, error))?;
    Ok(RestoreRecoveryReceipt {
        outcome: RestoreRecoveryOutcome::RolledBack,
        domains,
    })
}

pub(super) fn rollback_all(
    store: &ConfigStore,
    authority: &Path,
    state: &RestoreJournal,
) -> Result<(), RestoreRecoveryError> {
    for entry in &state.entries {
        let file = resolve_entry(store, entry)?;
        let rollback = journal::read_rollback(authority, entry).map_err(|error| {
            recovery_error(
                journal::journal_path(authority),
                Some(entry.domain.clone()),
                error,
            )
        })?;
        live_io::publish_state(&file, rollback.as_deref()).map_err(|error| {
            recovery_error(
                file.full_path().to_path_buf(),
                Some(entry.domain.clone()),
                error,
            )
        })?;
    }
    for entry in &state.entries {
        let file = resolve_entry(store, entry)?;
        live_io::verify_state(&file, &entry.old).map_err(|error| {
            recovery_error(
                file.full_path().to_path_buf(),
                Some(entry.domain.clone()),
                error,
            )
        })?;
    }
    Ok(())
}

pub(super) fn resolve_entry(
    store: &ConfigStore,
    entry: &JournalEntry,
) -> Result<ResolvedFile, RestoreRecoveryError> {
    let descriptor =
        store
            .registered_descriptor(&entry.domain)
            .ok_or_else(|| RestoreRecoveryError {
                path: PathBuf::from(&entry.relative_path),
                domain: Some(entry.domain.clone()),
                detail: "journal domain is not registered".into(),
            })?;
    if descriptor.storage_class() != entry.storage_class
        || descriptor.file_path().map(|path| path.as_str()) != Some(entry.relative_path.as_str())
    {
        return Err(RestoreRecoveryError {
            path: PathBuf::from(&entry.relative_path),
            domain: Some(entry.domain.clone()),
            detail: "registered descriptor no longer matches journal authority".into(),
        });
    }
    match store.roots.resolve(descriptor) {
        DomainLocation::File(file) => Ok(file),
        _ => Err(RestoreRecoveryError {
            path: PathBuf::from(&entry.relative_path),
            domain: Some(entry.domain.clone()),
            detail: "journal domain no longer resolves to an ordinary file".into(),
        }),
    }
}

fn recovery_error(
    path: PathBuf,
    domain: Option<longhorn_core::DomainId>,
    error: impl std::fmt::Display,
) -> RestoreRecoveryError {
    RestoreRecoveryError {
        path,
        domain,
        detail: error.to_string(),
    }
}
