use std::sync::Mutex;

use longhorn_update::{
    UpdateCheckCommand, UpdateDeferCommand, UpdateInstallCommand, UpdateOutcomeProjection,
    UpdateSelectChannelCommand, UpdateSnapshot,
};

use crate::{UpdateHostAuthority, UpdateHostError, UpdateHostService};

/// Shared injected assembly used by Tauri and conformance tests.
pub struct UpdateHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> UpdateHandlerAssembly<A> {
    /// Binds one explicitly injected consumer authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            authority: Mutex::new(authority),
        }
    }

    /// Runs trusted host work against injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, UpdateHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| UpdateHostError::state_unavailable())
    }
}

impl<A> UpdateHostService for UpdateHandlerAssembly<A>
where
    A: UpdateHostAuthority,
{
    fn snapshot(&self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    fn check(
        &self,
        caller: &str,
        command: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.check(caller, command))?
    }

    fn select_channel(
        &self,
        caller: &str,
        command: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.select_channel(caller, command))?
    }

    fn defer(
        &self,
        caller: &str,
        command: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.defer(caller, command))?
    }

    fn install(
        &self,
        caller: &str,
        command: UpdateInstallCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.install(caller, command))?
    }
}
