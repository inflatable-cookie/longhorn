use std::sync::Mutex;

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkNavigationCommand, ForkNavigationResult,
    ForkPathPageCommand, ForkPathPageSnapshot, ForkSnapshot,
};

use crate::{ForkHistoryHostAuthority, ForkHistoryHostError, ForkHistoryHostService};

/// Shared injected assembly used by Tauri and conformance tests.
pub struct ForkHistoryHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> ForkHistoryHandlerAssembly<A> {
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
    ) -> Result<Output, ForkHistoryHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| ForkHistoryHostError::state_unavailable())
    }
}

impl<A> ForkHistoryHostService for ForkHistoryHandlerAssembly<A>
where
    A: ForkHistoryHostAuthority,
{
    fn snapshot(&self, caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    fn path(
        &self,
        caller: &str,
        command: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.path(caller, command))?
    }

    fn branches(
        &self,
        caller: &str,
        command: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.branches(caller, command))?
    }

    fn navigate(
        &self,
        caller: &str,
        command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError> {
        self.with_authority(|authority| authority.navigate(caller, command))?
    }
}
