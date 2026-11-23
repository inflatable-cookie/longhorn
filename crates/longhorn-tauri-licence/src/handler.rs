use std::sync::Mutex;

use longhorn_licence::{
    LicenceActivateCommand, LicenceDeactivateCommand, LicenceOutcomeProjection,
    LicenceRefreshCommand, LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSnapshot,
};

use crate::{LicenceHostAuthority, LicenceHostError};

/// Shared injected assembly used by Tauri and conformance tests.
pub struct LicenceHandlerAssembly<A: ?Sized> {
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
}

impl<A: ?Sized> LicenceHandlerAssembly<A> {
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

impl<A> LicenceHandlerAssembly<A>
where
    A: LicenceHostAuthority + ?Sized,
{
    /// Returns the caller-authorized licence state.
    pub fn snapshot(&self, caller: &str) -> Result<LicenceSnapshot, LicenceHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    /// Presents a credential and asks for a licence.
    pub fn activate(
        &self,
        caller: &str,
        command: LicenceActivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.activate(caller, command))?
    }

    /// Releases this machine's seat.
    pub fn deactivate(
        &self,
        caller: &str,
        command: LicenceDeactivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.deactivate(caller, command))?
    }

    /// Re-checks the lease now.
    pub fn refresh(
        &self,
        caller: &str,
        command: LicenceRefreshCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.refresh(caller, command))?
    }

    /// Releases a named machine's seat.
    pub fn release_seat(
        &self,
        caller: &str,
        command: LicenceReleaseSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.release_seat(caller, command))?
    }

    /// Renames a machine's seat.
    pub fn rename_seat(
        &self,
        caller: &str,
        command: LicenceRenameSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError> {
        self.with_authority(|authority| authority.rename_seat(caller, command))?
    }
}
