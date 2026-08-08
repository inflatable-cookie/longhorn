//! Host-destroy, navigation, and teardown receipts.

use longhorn_native_content::{AttachGeneration, NativeContentIslandId};
use serde::Serialize;
use tauri::Url;

/// Result of applying one exact host-destruction notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewHostDestroyOutcome {
    /// A live or attaching child was invalidated.
    Invalidated,
    /// The same generation was already invalidated.
    AlreadyInvalidated,
}

/// Adapter-local host-destruction evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewHostDestroyReceipt {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) generation: AttachGeneration,
    pub(crate) outcome: ChildViewHostDestroyOutcome,
}

impl ChildViewHostDestroyReceipt {
    /// Returns the invalidated island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact invalidated generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call performed or confirmed invalidation.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewHostDestroyOutcome {
        self.outcome
    }
}

/// Result of one adapter shutdown attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewTeardownOutcome {
    /// The retained child was closed.
    Closed,
    /// No child remained to close.
    AlreadyDetached,
}

/// Result of one policy-admitted document request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildViewNavigationOutcome {
    /// The requested URL was already current; no navigation was submitted.
    Unchanged,
    /// The native runtime accepted one navigation request.
    Submitted,
}

/// Adapter-local evidence for one generation-bound document request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewNavigationReceipt {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) generation: AttachGeneration,
    pub(crate) previous_url: Url,
    pub(crate) requested_url: Url,
    pub(crate) outcome: ChildViewNavigationOutcome,
}

impl ChildViewNavigationReceipt {
    /// Returns the retained island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact retained attach generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns the fresh URL observed before the request.
    #[must_use]
    pub const fn previous_url(&self) -> &Url {
        &self.previous_url
    }

    /// Returns the consumer-requested URL.
    #[must_use]
    pub const fn requested_url(&self) -> &Url {
        &self.requested_url
    }

    /// Returns whether native navigation was unnecessary or submitted.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewNavigationOutcome {
        self.outcome
    }
}

/// Adapter-local bounded teardown evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChildViewTeardownReceipt {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) generation: Option<AttachGeneration>,
    pub(crate) outcome: ChildViewTeardownOutcome,
}

impl ChildViewTeardownReceipt {
    /// Returns the adapter island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the closed generation, when one existed.
    #[must_use]
    pub const fn generation(&self) -> Option<AttachGeneration> {
        self.generation
    }

    /// Returns the exact teardown result.
    #[must_use]
    pub const fn outcome(&self) -> ChildViewTeardownOutcome {
        self.outcome
    }
}
