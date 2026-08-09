use longhorn_core::WindowId;
use serde::{Deserialize, Serialize};

use super::{
    ApplyGeneration, HostCapability, HostWindowHandle, WindowOperation, WindowOperationKind,
};

/// One operation bound to the caller's apply generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedWindowOperation {
    generation: ApplyGeneration,
    operation: WindowOperation,
}

impl PlannedWindowOperation {
    pub(crate) const fn new(generation: ApplyGeneration, operation: WindowOperation) -> Self {
        Self {
            generation,
            operation,
        }
    }

    /// Returns caller-supplied apply generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns the pure host instruction.
    #[must_use]
    pub const fn operation(&self) -> &WindowOperation {
        &self.operation
    }

    /// Returns evidence a host may attach to resulting feedback.
    #[must_use]
    pub fn feedback_evidence(&self) -> ApplyFeedbackEvidence {
        ApplyFeedbackEvidence {
            generation: self.generation,
            window_id: self.operation.window_id().clone(),
            operation: self.operation.kind(),
        }
    }
}

/// Evidence used by a host to reject stale programmatic feedback.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplyFeedbackEvidence {
    generation: ApplyGeneration,
    window_id: WindowId,
    operation: WindowOperationKind,
}

impl ApplyFeedbackEvidence {
    /// Returns whether this evidence belongs to the host's current generation.
    #[must_use]
    pub const fn is_current(&self, current: ApplyGeneration) -> bool {
        self.generation.get() == current.get()
    }

    /// Returns the operation generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns stable logical identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns originating operation category.
    #[must_use]
    pub const fn operation(&self) -> WindowOperationKind {
        self.operation
    }
}

/// Inspectable reason an intended mutation was unavailable or unsafe.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowDiffDiagnostic {
    /// The host did not declare a required operation capability.
    UnsupportedOperation {
        /// Intended operation.
        operation: WindowOperationKind,
        /// Stable logical target.
        window_id: WindowId,
        /// Existing handle when one was available.
        transport_handle: Option<HostWindowHandle>,
        /// Missing capability.
        required_capability: HostCapability,
    },
    /// Explicit protected slot was absent from the live snapshot.
    ProtectedSlotMissing {
        /// Missing opaque handle.
        transport_handle: HostWindowHandle,
    },
    /// Explicit reuse target was not present in desired state.
    ProtectedReuseTargetMissing {
        /// Missing desired logical identity.
        window_id: WindowId,
    },
    /// Stable identity already matched the reuse target at another handle.
    ProtectedReuseConflict {
        /// Protected slot requested by policy.
        protected_handle: HostWindowHandle,
        /// Desired logical identity already matched elsewhere.
        window_id: WindowId,
        /// Existing stable-identity match.
        matched_handle: HostWindowHandle,
    },
    /// Focus policy named no desired window.
    FocusTargetMissing {
        /// Missing desired logical identity.
        window_id: WindowId,
    },
    /// Focus policy named a desired hidden window.
    FocusTargetHidden {
        /// Hidden logical identity.
        window_id: WindowId,
    },
    /// A live slot had no stable id and was not explicitly protected.
    UnidentifiedLiveWindow {
        /// Opaque handle that was not interpreted as identity.
        transport_handle: HostWindowHandle,
    },
}

/// Complete deterministic diff output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowDiffReceipt {
    generation: ApplyGeneration,
    operations: Vec<PlannedWindowOperation>,
    diagnostics: Vec<WindowDiffDiagnostic>,
}

impl WindowDiffReceipt {
    pub(crate) const fn new(
        generation: ApplyGeneration,
        operations: Vec<PlannedWindowOperation>,
        diagnostics: Vec<WindowDiffDiagnostic>,
    ) -> Self {
        Self {
            generation,
            operations,
            diagnostics,
        }
    }

    /// Returns caller-supplied apply generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns ordered programmatic operations.
    #[must_use]
    pub fn operations(&self) -> &[PlannedWindowOperation] {
        &self.operations
    }

    /// Returns canonical diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[WindowDiffDiagnostic] {
        &self.diagnostics
    }

    /// Returns whether no operations or diagnostics remain.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty() && self.diagnostics.is_empty()
    }

    /// Drops operations a host has declared it cannot observe settling.
    ///
    /// A post-apply readback re-plans from fresh evidence, which assumes the
    /// platform finished before it was read. Not every platform has: on GPUI
    /// under macOS, `zoom_window` returns and the next `is_maximized` still
    /// reports the old state, because the window server animates the zoom.
    /// Re-planning against that reading schedules an operation that already
    /// succeeded, and a caller that trusts convergence retries forever.
    ///
    /// So readback is evidence, not a verdict. A host names its deferred
    /// operations and convergence stops counting them. Diagnostics are kept:
    /// an unsupported operation is a fact about the host, not a timing
    /// artefact, and it does not become true later.
    #[must_use]
    pub fn without_deferred(mut self, deferred: &DeferredSettlement) -> Self {
        self.operations
            .retain(|planned| !deferred.contains(planned.operation().kind()));
        self
    }
}

/// The operations whose effect a host cannot observe in the same turn.
///
/// Empty for a host that settles synchronously, which is the common case and
/// the default.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DeferredSettlement(std::collections::BTreeSet<WindowOperationKind>);

impl DeferredSettlement {
    /// Declares that every operation settles before the host reads it back.
    #[must_use]
    pub const fn immediate() -> Self {
        Self(std::collections::BTreeSet::new())
    }

    /// Declares an exact set of deferred operations.
    #[must_use]
    pub fn from_operations(kinds: impl IntoIterator<Item = WindowOperationKind>) -> Self {
        Self(kinds.into_iter().collect())
    }

    /// Returns whether the host declared this operation deferred.
    #[must_use]
    pub fn contains(&self, kind: WindowOperationKind) -> bool {
        self.0.contains(&kind)
    }

    /// Returns whether every operation settles synchronously.
    #[must_use]
    pub fn is_immediate(&self) -> bool {
        self.0.is_empty()
    }
}
