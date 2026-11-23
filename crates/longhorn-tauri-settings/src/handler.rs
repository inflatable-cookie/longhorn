use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use longhorn_core::SettingsApplyUnitId;
use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationResult,
    SettingsRegistrySnapshot, SettingsResetCommand,
};
use longhorn_settings_config::ConsumerSettingsTransactionAuthority;

use crate::{SettingsHostAuthority, SettingsHostError};

type ApplyTransactionFn = dyn Fn(&SettingsApplyCommand) -> Result<SettingsMutationResult, SettingsHostError>
    + Send
    + Sync;
type ResetTransactionFn = dyn Fn(&SettingsResetCommand) -> Result<SettingsMutationResult, SettingsHostError>
    + Send
    + Sync;

struct TransactionSeam {
    units: BTreeSet<SettingsApplyUnitId>,
    apply: Box<ApplyTransactionFn>,
    reset: Box<ResetTransactionFn>,
}

/// Shared command assembly used by Tauri and direct/serialized tests.
pub struct SettingsHandlerAssembly<A: ?Sized> {
    transaction: Option<TransactionSeam>,
    authority: Mutex<A>,
}

impl<A> SettingsHandlerAssembly<A> {
    /// Binds one explicitly injected authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            transaction: None,
            authority: Mutex::new(authority),
        }
    }

    /// Binds one explicitly injected authority together with an optional
    /// consumer-owned failure-atomic transaction authority for the named apply
    /// units (contract 005).
    #[must_use]
    pub fn with_transaction_authority<T, R>(
        authority: A,
        units: impl IntoIterator<Item = SettingsApplyUnitId>,
        transaction: T,
    ) -> Self
    where
        T: ConsumerSettingsTransactionAuthority<Error = SettingsHostError, Receipt = R>
            + Send
            + Sync
            + 'static,
    {
        let shared = Arc::new(transaction);
        let apply_tx = Arc::clone(&shared);
        let reset_tx = shared;
        Self {
            transaction: Some(TransactionSeam {
                units: units.into_iter().collect(),
                apply: Box::new(move |command| {
                    apply_tx
                        .apply_transaction(command)
                        .map(|outcome| outcome.result)
                }),
                reset: Box::new(move |command| {
                    reset_tx
                        .reset_transaction(command)
                        .map(|outcome| outcome.result)
                }),
            }),
            authority: Mutex::new(authority),
        }
    }
}

impl<A: ?Sized> SettingsHandlerAssembly<A> {
    /// Runs trusted host work against the injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, SettingsHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| SettingsHostError::state_unavailable())
    }
}

impl<A> SettingsHandlerAssembly<A>
where
    A: SettingsHostAuthority + ?Sized,
{
    /// Returns one caller-authorized registry snapshot.
    pub fn registry(&self, caller: &str) -> Result<SettingsRegistrySnapshot, SettingsHostError> {
        self.with_authority(|authority| authority.registry(caller))?
    }

    /// Loads one caller-authorized scope.
    pub fn load(
        &self,
        caller: &str,
        command: SettingsLoadCommand,
    ) -> Result<SettingsLoadOutcome, SettingsHostError> {
        self.with_authority(|authority| authority.load(caller, command))?
    }

    /// Applies one caller-authorized mutation.
    pub fn apply(
        &self,
        caller: &str,
        command: SettingsApplyCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        if let Some(seam) = self.transaction.as_ref()
            && seam.units.contains(&command.apply_unit_id)
        {
            self.with_authority(|authority| {
                authority.authorize_transaction(caller, &command.apply_unit_id)
            })??;
            return (seam.apply)(&command);
        }
        self.with_authority(|authority| authority.apply(caller, command))?
    }

    /// Resets caller-authorized user overrides.
    pub fn reset(
        &self,
        caller: &str,
        command: SettingsResetCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        if let Some(seam) = self.transaction.as_ref()
            && seam.units.contains(&command.apply_unit_id)
        {
            self.with_authority(|authority| {
                authority.authorize_transaction(caller, &command.apply_unit_id)
            })??;
            return (seam.reset)(&command);
        }
        self.with_authority(|authority| authority.reset(caller, command))?
    }
}
