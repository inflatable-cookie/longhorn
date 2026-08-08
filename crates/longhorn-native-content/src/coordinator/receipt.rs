use longhorn_core::NativeContentRevision;
use serde::{Deserialize, Serialize};

use crate::{AttachGeneration, AttachmentLifecycle};

/// Successful desired-state replacement evidence.
/// Successful desired-state replacement evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct DesiredUpdateReceipt {
    pub(crate) previous_revision: NativeContentRevision,
    pub(crate) current_revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
}

impl DesiredUpdateReceipt {
    /// Returns the revision checked by the caller.
    #[must_use]
    pub const fn previous_revision(self) -> NativeContentRevision {
        self.previous_revision
    }
    /// Returns the committed desired revision.
    #[must_use]
    pub const fn current_revision(self) -> NativeContentRevision {
        self.current_revision
    }
    /// Returns the current desired attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
}

/// Successful fresh-observation admission evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ObservationReceipt {
    pub(crate) previous_revision: NativeContentRevision,
    pub(crate) current_revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
    pub(crate) lifecycle: AttachmentLifecycle,
}

impl ObservationReceipt {
    /// Returns the previously observed revision.
    #[must_use]
    pub const fn previous_revision(self) -> NativeContentRevision {
        self.previous_revision
    }
    /// Returns the committed observed revision.
    #[must_use]
    pub const fn current_revision(self) -> NativeContentRevision {
        self.current_revision
    }
    /// Returns admitted attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
    /// Returns admitted lifecycle.
    #[must_use]
    pub const fn lifecycle(self) -> AttachmentLifecycle {
        self.lifecycle
    }
}

/// Result of applying one host-destruction event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HostDestroyOutcome {
    /// The current generation was invalidated and observation became absent.
    Invalidated,
    /// This exact generation was already invalidated.
    AlreadyInvalidated,
}

/// Exact evidence for host-destruction invalidation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct HostDestroyReceipt {
    pub(crate) previous_observed_revision: NativeContentRevision,
    pub(crate) current_observed_revision: NativeContentRevision,
    pub(crate) generation: AttachGeneration,
    pub(crate) outcome: HostDestroyOutcome,
}

impl HostDestroyReceipt {
    /// Returns the observed revision checked by the caller.
    #[must_use]
    pub const fn previous_observed_revision(self) -> NativeContentRevision {
        self.previous_observed_revision
    }
    /// Returns the observed revision after invalidation.
    #[must_use]
    pub const fn current_observed_revision(self) -> NativeContentRevision {
        self.current_observed_revision
    }
    /// Returns the invalidated attach generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }
    /// Returns whether this call performed or confirmed invalidation.
    #[must_use]
    pub const fn outcome(self) -> HostDestroyOutcome {
        self.outcome
    }
}
