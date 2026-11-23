use std::sync::Mutex;

use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkContinuationPageCommand,
    ForkContinuationPageSnapshot, ForkDeleteContinuationCommand, ForkNavigationCommand,
    ForkNavigationResult, ForkPathPageCommand, ForkPathPageSnapshot, ForkPruneCommand,
    ForkPruneResult, ForkRemovalReceiptProjection, ForkSnapshot,
};

use crate::{ForkHistoryHostAuthority, ForkHistoryHostError};

/// Shared injected assembly used by Tauri and conformance tests.
pub struct ForkHistoryHandlerAssembly<A: ?Sized> {
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
}

impl<A: ?Sized> ForkHistoryHandlerAssembly<A> {
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

impl<A> ForkHistoryHandlerAssembly<A>
where
    A: ForkHistoryHostAuthority + ?Sized,
{
    /// Returns one caller-authorized linear-default summary.
    pub fn snapshot(&self, caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    /// Returns one caller-authorized bounded path page.
    pub fn path(
        &self,
        caller: &str,
        command: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.path(caller, command))?
    }

    /// Returns one caller-authorized bounded branch page.
    pub fn branches(
        &self,
        caller: &str,
        command: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.branches(caller, command))?
    }

    /// Returns one caller-authorized bounded continuation page.
    pub fn continuations(
        &self,
        caller: &str,
        command: ForkContinuationPageCommand,
    ) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError> {
        self.with_authority(|authority| authority.continuations(caller, command))?
    }

    /// Deletes one continuation and everything below it. Irreversible.
    pub fn delete_continuation(
        &self,
        caller: &str,
        command: ForkDeleteContinuationCommand,
    ) -> Result<ForkRemovalReceiptProjection, ForkHistoryHostError> {
        self.with_authority(|authority| authority.delete_continuation(caller, command))?
    }

    /// Prunes the unprotected share of the graph to a budget.
    pub fn prune(
        &self,
        caller: &str,
        command: ForkPruneCommand,
    ) -> Result<ForkPruneResult, ForkHistoryHostError> {
        self.with_authority(|authority| authority.prune(caller, command))?
    }

    /// Applies and commits one caller-authorized graph navigation.
    pub fn navigate(
        &self,
        caller: &str,
        command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError> {
        self.with_authority(|authority| authority.navigate(caller, command))?
    }
}
