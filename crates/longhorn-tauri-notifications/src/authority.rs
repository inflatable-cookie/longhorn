use longhorn_notifications::{
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};

use crate::NotificationHostError;

/// Consumer-injected caller authorization and ledger authority.
pub trait NotificationHostAuthority: Send {
    /// Returns one caller-authorized bounded page.
    fn snapshot(
        &mut self,
        caller: &str,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError>;

    /// Applies one caller-authorized ledger mutation.
    fn mutate(
        &mut self,
        caller: &str,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError>;
}
