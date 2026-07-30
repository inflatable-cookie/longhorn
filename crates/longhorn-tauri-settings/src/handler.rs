use std::sync::Mutex;

use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationResult,
    SettingsRegistrySnapshot, SettingsResetCommand,
};

use crate::{SettingsAuthority, SettingsCommandService, SettingsHostError};

/// Shared command assembly used by Tauri and direct/serialized tests.
pub struct SettingsHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> SettingsHandlerAssembly<A> {
    /// Binds one explicitly injected authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            authority: Mutex::new(authority),
        }
    }

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

impl<A> SettingsCommandService for SettingsHandlerAssembly<A>
where
    A: SettingsAuthority,
{
    fn registry(&self, caller: &str) -> Result<SettingsRegistrySnapshot, SettingsHostError> {
        self.with_authority(|authority| authority.registry(caller))?
    }

    fn load(
        &self,
        caller: &str,
        command: SettingsLoadCommand,
    ) -> Result<SettingsLoadOutcome, SettingsHostError> {
        self.with_authority(|authority| authority.load(caller, command))?
    }

    fn apply(
        &self,
        caller: &str,
        command: SettingsApplyCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        self.with_authority(|authority| authority.apply(caller, command))?
    }

    fn reset(
        &self,
        caller: &str,
        command: SettingsResetCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError> {
        self.with_authority(|authority| authority.reset(caller, command))?
    }
}
