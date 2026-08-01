use std::sync::Mutex;

use longhorn_notifications::{
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};

use crate::{NotificationHostAuthority, NotificationHostError, NotificationHostService};

/// Shared injected notification assembly used by Tauri and conformance tests.
pub struct NotificationHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> NotificationHandlerAssembly<A> {
    /// Binds one explicitly injected authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            authority: Mutex::new(authority),
        }
    }
}

impl<A> NotificationHostService for NotificationHandlerAssembly<A>
where
    A: NotificationHostAuthority,
{
    fn snapshot(
        &self,
        caller: &str,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError> {
        self.authority
            .lock()
            .map_err(|_| NotificationHostError::authority_state_unavailable())?
            .snapshot(caller, query)
    }

    fn mutate(
        &self,
        caller: &str,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError> {
        self.authority
            .lock()
            .map_err(|_| NotificationHostError::authority_state_unavailable())?
            .mutate(caller, command)
    }
}
