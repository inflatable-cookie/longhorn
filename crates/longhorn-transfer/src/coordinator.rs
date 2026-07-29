mod authority;
mod lease;
mod resolution;
mod session;

use std::collections::{BTreeMap, VecDeque};

use longhorn_core::{TransferClientId, WindowId};

use crate::{
    ClientEpoch, DragSessionId, DropZone, LeaseGeneration, TransferInstant, TransferLimits,
    TransferSourceAuthority,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionStatus {
    Active,
    Cancelled,
    Attempted,
    SourceWindowDestroyed,
    SourceClientChanged,
}

#[derive(Clone, Debug)]
struct SessionRecord {
    source: TransferSourceAuthority,
    expires_at: TransferInstant,
    status: SessionStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClientBinding {
    client_id: TransferClientId,
    epoch: ClientEpoch,
}

#[derive(Clone, Debug)]
struct LeaseRecord {
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    generation: LeaseGeneration,
    expires_at: TransferInstant,
    window_outer_bounds: longhorn_core::ScreenRect,
    zones: Vec<DropZone>,
}

/// Process-local bounded transfer-session and drop-zone coordinator.
#[derive(Clone, Debug)]
pub struct TransferCoordinator {
    limits: TransferLimits,
    last_now: Option<TransferInstant>,
    sessions: BTreeMap<DragSessionId, SessionRecord>,
    session_order: VecDeque<DragSessionId>,
    clients: BTreeMap<WindowId, ClientBinding>,
    leases: BTreeMap<WindowId, LeaseRecord>,
}

impl TransferCoordinator {
    /// Constructs an empty process-local coordinator.
    #[must_use]
    pub const fn new(limits: TransferLimits) -> Self {
        Self {
            limits,
            last_now: None,
            sessions: BTreeMap::new(),
            session_order: VecDeque::new(),
            clients: BTreeMap::new(),
            leases: BTreeMap::new(),
        }
    }

    /// Returns configured finite bounds.
    #[must_use]
    pub const fn limits(&self) -> TransferLimits {
        self.limits
    }

    /// Returns retained session records, including bounded terminal evidence.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Returns current client-window bindings.
    #[must_use]
    pub fn client_window_count(&self) -> usize {
        self.clients.len()
    }

    /// Returns current complete window leases.
    #[must_use]
    pub fn lease_count(&self) -> usize {
        self.leases.len()
    }

    /// Discards all process-local transfer authority during host shutdown.
    pub fn discard_all(&mut self) -> CoordinatorDiscardReceipt {
        let receipt = CoordinatorDiscardReceipt {
            sessions: self.sessions.len(),
            client_windows: self.clients.len(),
            leases: self.leases.len(),
        };
        self.sessions.clear();
        self.session_order.clear();
        self.clients.clear();
        self.leases.clear();
        receipt
    }
}

/// Process-local authority discarded during host shutdown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoordinatorDiscardReceipt {
    sessions: usize,
    client_windows: usize,
    leases: usize,
}

impl CoordinatorDiscardReceipt {
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

    /// Returns discarded current leases.
    #[must_use]
    pub const fn leases(self) -> usize {
        self.leases
    }
}

/// Result of installing current renderer authority for one window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientEpochBindingStatus {
    /// First current renderer binding for the window.
    Installed,
    /// A greater epoch replaced prior renderer authority.
    Advanced,
    /// The exact current renderer binding was already installed.
    Unchanged,
}

/// Window-destroy invalidation evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowInvalidationReceipt {
    removed_client_binding: bool,
    removed_lease: bool,
    invalidated_source_sessions: usize,
}

impl WindowInvalidationReceipt {
    /// Returns whether current renderer authority was removed.
    #[must_use]
    pub const fn removed_client_binding(self) -> bool {
        self.removed_client_binding
    }

    /// Returns whether a current lease was removed.
    #[must_use]
    pub const fn removed_lease(self) -> bool {
        self.removed_lease
    }

    /// Returns active source sessions invalidated by the destroy.
    #[must_use]
    pub const fn invalidated_source_sessions(self) -> usize {
        self.invalidated_source_sessions
    }
}

/// Successful complete lease replacement evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeasePublicationReceipt {
    generation: LeaseGeneration,
    expires_at: TransferInstant,
    zone_count: usize,
}

impl LeasePublicationReceipt {
    /// Returns the installed generation.
    #[must_use]
    pub const fn generation(self) -> LeaseGeneration {
        self.generation
    }

    /// Returns the lease expiry.
    #[must_use]
    pub const fn expires_at(self) -> TransferInstant {
        self.expires_at
    }

    /// Returns the complete installed zone count.
    #[must_use]
    pub const fn zone_count(self) -> usize {
        self.zone_count
    }
}
