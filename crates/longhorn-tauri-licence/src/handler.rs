use std::sync::Mutex;

use longhorn_licence::{
    LicenceActivateCommand, LicenceDeactivateCommand, LicenceOutcomeProjection,
    LicenceRefreshCommand, LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSnapshot,
};

use crate::{LicenceHostAuthority, LicenceHostError, LicenceHostService};

/// Shared injected assembly used by Tauri and conformance tests.
pub struct LicenceHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> LicenceHandlerAssembly<A> {
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
    ) -> Result<Output, LicenceHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| LicenceHostError::state_unavailable())
    }
}

impl<A> LicenceHostService for LicenceHandlerAssembly<A>
where
    A: LicenceHostAuthority,
{
    fn snapshot(&self, caller: &str) -> Result<LicenceSnapshot, LicenceHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    fn activate(
        &self,
        caller: &str,
        command: LicenceActivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.activate(caller, command))?
    }

    fn deactivate(
        &self,
        caller: &str,
        command: LicenceDeactivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.deactivate(caller, command))?
    }

    fn refresh(
        &self,
        caller: &str,
        command: LicenceRefreshCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.refresh(caller, command))?
    }

    fn release_seat(
        &self,
        caller: &str,
        command: LicenceReleaseSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.release_seat(caller, command))?
    }

    fn rename_seat(
        &self,
        caller: &str,
        command: LicenceRenameSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.rename_seat(caller, command))?
    }
}
