use longhorn_history::{
    HistoryNavigationCommand, HistoryNavigationResult, HistoryPageCommand, HistoryPageSnapshot,
    HistorySnapshot,
};

use crate::HistoryHostError;

/// Consumer-injected caller authorization, product apply, and history authority.
pub trait HistoryHostAuthority: Send {
    /// Returns one caller-authorized metadata snapshot.
    fn snapshot(&mut self, caller: &str) -> Result<HistorySnapshot, HistoryHostError>;

    /// Returns one caller-authorized bounded metadata page.
    fn page(
        &mut self,
        caller: &str,
        command: HistoryPageCommand,
    ) -> Result<HistoryPageSnapshot, HistoryHostError>;

    /// Applies and commits one caller-authorized checked navigation.
    fn navigate(
        &mut self,
        caller: &str,
        command: HistoryNavigationCommand,
    ) -> Result<HistoryNavigationResult, HistoryHostError>;
}
