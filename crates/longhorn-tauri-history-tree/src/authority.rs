use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkContinuationPageCommand,
    ForkContinuationPageSnapshot, ForkDeleteContinuationCommand, ForkNavigationCommand,
    ForkNavigationResult, ForkPathPageCommand, ForkPathPageSnapshot, ForkRemovalReceiptProjection,
    ForkSnapshot,
};

use crate::ForkHistoryHostError;

/// Consumer-injected caller authorization, product apply, and graph authority.
pub trait ForkHistoryHostAuthority: Send {
    /// Returns one caller-authorized linear-default summary.
    fn snapshot(&mut self, caller: &str) -> Result<ForkSnapshot, ForkHistoryHostError>;

    /// Returns one caller-authorized bounded path page.
    fn path(
        &mut self,
        caller: &str,
        command: ForkPathPageCommand,
    ) -> Result<ForkPathPageSnapshot, ForkHistoryHostError>;

    /// Returns one caller-authorized bounded branch page.
    fn branches(
        &mut self,
        caller: &str,
        command: ForkBranchPageCommand,
    ) -> Result<ForkBranchPageSnapshot, ForkHistoryHostError>;

    /// Returns one caller-authorized bounded continuation page.
    fn continuations(
        &mut self,
        caller: &str,
        command: ForkContinuationPageCommand,
    ) -> Result<ForkContinuationPageSnapshot, ForkHistoryHostError>;

    /// Deletes one continuation and everything below it. Irreversible.
    ///
    /// Separate from `navigate` because it destroys authority rather than
    /// moving through it, and its capability is separate for the same reason.
    fn delete_continuation(
        &mut self,
        caller: &str,
        command: ForkDeleteContinuationCommand,
    ) -> Result<ForkRemovalReceiptProjection, ForkHistoryHostError>;

    /// Applies and commits one caller-authorized graph navigation.
    fn navigate(
        &mut self,
        caller: &str,
        command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError>;
}
