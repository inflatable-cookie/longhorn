use std::sync::Mutex;

use longhorn_history::{
    HistoryNavigationCommand, HistoryNavigationResult, HistoryPageCommand, HistoryPageSnapshot,
    HistorySnapshot,
};

use crate::{HistoryHostAuthority, HistoryHostError, HistoryHostService};

/// Shared history assembly used by Tauri and direct/serialized tests.
pub struct HistoryHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> HistoryHandlerAssembly<A> {
    /// Binds one explicitly injected consumer authority.
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
    ) -> Result<Output, HistoryHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| HistoryHostError::state_unavailable())
    }
}

impl<A> HistoryHostService for HistoryHandlerAssembly<A>
where
    A: HistoryHostAuthority,
{
    fn snapshot(&self, caller: &str) -> Result<HistorySnapshot, HistoryHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    fn page(
        &self,
        caller: &str,
        command: HistoryPageCommand,
    ) -> Result<HistoryPageSnapshot, HistoryHostError> {
        self.with_authority(|authority| authority.page(caller, command))?
    }

    fn navigate(
        &self,
        caller: &str,
        command: HistoryNavigationCommand,
    ) -> Result<HistoryNavigationResult, HistoryHostError> {
        self.with_authority(|authority| authority.navigate(caller, command))?
    }
}
