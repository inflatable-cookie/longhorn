use longhorn_core::{TransferClientId, WindowId};
use longhorn_transfer::{ClientEpoch, TransferDuration};

use super::CurrentClient;

/// Current host-issued renderer authority passed to a domain adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferCallerAuthority {
    window_id: WindowId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    session_lifetime: TransferDuration,
}

impl TransferCallerAuthority {
    pub(super) fn new(
        window_id: WindowId,
        client: CurrentClient,
        session_lifetime: TransferDuration,
    ) -> Self {
        Self {
            window_id,
            client_id: client.client_id,
            client_epoch: client.epoch,
            session_lifetime,
        }
    }

    /// Returns current managed caller identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns current host-issued renderer identity.
    #[must_use]
    pub const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    /// Returns current host-issued renderer epoch.
    #[must_use]
    pub const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    /// Returns the host's bounded default session lifetime.
    #[must_use]
    pub const fn session_lifetime(&self) -> TransferDuration {
        self.session_lifetime
    }
}

/// Idempotent handler teardown status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferHandlerTeardownStatus {
    /// Process-local transfer authority was discarded.
    TornDown,
    /// A prior teardown already discarded authority.
    AlreadyTornDown,
}

/// Bounded process-local authority discarded at handler teardown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferHandlerTeardownReceipt {
    pub(super) status: TransferHandlerTeardownStatus,
    pub(super) sessions: usize,
    pub(super) client_windows: usize,
    pub(super) leases: usize,
}

impl TransferHandlerTeardownReceipt {
    /// Returns idempotent teardown status.
    #[must_use]
    pub const fn status(self) -> TransferHandlerTeardownStatus {
        self.status
    }

    /// Returns discarded session records.
    #[must_use]
    pub const fn sessions(self) -> usize {
        self.sessions
    }

    /// Returns discarded current client-window bindings.
    #[must_use]
    pub const fn client_windows(self) -> usize {
        self.client_windows
    }

    /// Returns discarded complete leases.
    #[must_use]
    pub const fn leases(self) -> usize {
        self.leases
    }
}
