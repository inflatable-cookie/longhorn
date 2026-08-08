//! Detach and host-destroy receipts.

use longhorn_native_content::{AttachGeneration, NativeContentIslandId};
use serde::Serialize;

/// Exact result of one reversible detach request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackingSurfaceDetachOutcome {
    /// Current storage and renderer resources were detached.
    Detached,
    /// The generation had already detached successfully.
    AlreadyDetached,
}

/// Adapter-local reversible detach evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackingSurfaceDetachReceipt {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) generation: AttachGeneration,
    pub(crate) outcome: BackingSurfaceDetachOutcome,
}

impl BackingSurfaceDetachReceipt {
    /// Returns the detached island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the exact generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call detached or confirmed prior detach.
    #[must_use]
    pub const fn outcome(&self) -> BackingSurfaceDetachOutcome {
        self.outcome
    }
}

/// Local host-destruction invalidation result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackingSurfaceHostDestroyOutcome {
    /// This call invalidated current callback authority.
    Invalidated,
    /// Callback authority was already invalidated.
    AlreadyInvalidated,
}

/// Exact local invalidation and reversible-detach evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackingSurfaceHostDestroyReceipt {
    pub(crate) island_id: NativeContentIslandId,
    pub(crate) generation: AttachGeneration,
    pub(crate) outcome: BackingSurfaceHostDestroyOutcome,
    pub(crate) detach: BackingSurfaceDetachOutcome,
}

impl BackingSurfaceHostDestroyReceipt {
    /// Returns the invalidated island.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the invalidated generation.
    #[must_use]
    pub const fn generation(&self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether this call established local invalidation.
    #[must_use]
    pub const fn outcome(&self) -> BackingSurfaceHostDestroyOutcome {
        self.outcome
    }

    /// Returns exact reversible-detach evidence.
    #[must_use]
    pub const fn detach(&self) -> BackingSurfaceDetachOutcome {
        self.detach
    }
}
