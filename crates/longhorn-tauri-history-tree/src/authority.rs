use longhorn_history_tree::{
    ForkBranchPageCommand, ForkBranchPageSnapshot, ForkNavigationCommand, ForkNavigationResult,
    ForkPathPageCommand, ForkPathPageSnapshot, ForkSnapshot,
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

    /// Applies and commits one caller-authorized graph navigation.
    fn navigate(
        &mut self,
        caller: &str,
        command: ForkNavigationCommand,
    ) -> Result<ForkNavigationResult, ForkHistoryHostError>;
}
