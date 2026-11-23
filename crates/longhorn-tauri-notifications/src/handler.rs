use std::sync::Mutex;

use longhorn_notifications::{
    NotificationMutationCommand, NotificationMutationResult, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};

use crate::{NotificationHostAuthority, NotificationHostError};

/// Shared injected notification assembly used by Tauri and conformance tests.
pub struct NotificationHandlerAssembly<A: ?Sized> {
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

impl<A: ?Sized> NotificationHandlerAssembly<A> {
    /// Runs trusted host work against the injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, NotificationHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| NotificationHostError::authority_state_unavailable())
    }
}

impl<A> NotificationHandlerAssembly<A>
where
    A: NotificationHostAuthority + ?Sized,
{
    /// Returns a caller-authorized snapshot page.
    pub fn snapshot(
        &self,
        caller: &str,
        query: NotificationSnapshotQuery,
    ) -> Result<NotificationSnapshotResponse, NotificationHostError> {
        self.with_authority(|authority| authority.snapshot(caller, query))?
    }

    /// Applies a caller-authorized mutation.
    pub fn mutate(
        &self,
        caller: &str,
        command: NotificationMutationCommand,
    ) -> Result<NotificationMutationResult, NotificationHostError> {
        self.with_authority(|authority| authority.mutate(caller, command))?
    }
}
