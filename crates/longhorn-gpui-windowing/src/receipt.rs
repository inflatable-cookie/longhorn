use longhorn_core::WindowId;
use longhorn_windowing::{
    ApplyGeneration, HostWindowHandle, WindowDiffError, WindowDiffReceipt, WindowOperationKind,
};

use crate::{GpuiDesktopObservation, GpuiObservationError};

/// One discrete native or registry call within an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiWindowCall {
    /// Resolve logical and transport identity.
    ResolveManagedWindow,
    /// Compose creation-time options from the plan.
    ComposeCreateRequest,
    /// Open a GPUI window.
    OpenWindow,
    /// Insert a successfully created managed slot.
    RegistryInsert,
    /// Retag managed bookkeeping.
    RegistryRetag,
    /// Enforce protected-primary close policy.
    ProtectPrimary,
    /// Set content size.
    Resize,
    /// Drive absolute maximized state.
    SetMaximized,
    /// Bring forward and focus.
    Activate,
    /// Remove the window.
    Close,
}

/// Stable category for one failed operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuiApplyFailureKind {
    /// Registry identity or generation invariant failed.
    Registry,
    /// The plan could not be turned into GPUI creation options.
    CreateComposition,
    /// A GPUI call failed.
    Native,
    /// Protected-primary policy refused close.
    ProtectedPrimary,
}

/// Typed operation failure with the exact failed call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiApplyFailure {
    call: GpuiWindowCall,
    kind: GpuiApplyFailureKind,
    detail: String,
}

impl GpuiApplyFailure {
    pub(crate) fn new(
        call: GpuiWindowCall,
        kind: GpuiApplyFailureKind,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            call,
            kind,
            detail: detail.into(),
        }
    }

    /// Returns the failed call.
    #[must_use]
    pub const fn call(&self) -> GpuiWindowCall {
        self.call
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> GpuiApplyFailureKind {
        self.kind
    }

    /// Returns the boundary diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Result of one planned operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiApplyOutcome {
    /// Every constituent call succeeded.
    Succeeded {
        /// Calls completed in order.
        completed_calls: Vec<GpuiWindowCall>,
    },
    /// Some calls succeeded before one failed.
    Failed {
        /// Calls completed before failure.
        completed_calls: Vec<GpuiWindowCall>,
        /// Exact failure.
        failure: GpuiApplyFailure,
    },
    /// An earlier failed operation blocked this same logical window.
    DependencySkipped {
        /// Earlier operation category.
        blocked_by: WindowOperationKind,
    },
}

/// Execution record for one planned operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiApplyAttempt {
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    outcome: GpuiApplyOutcome,
}

impl GpuiApplyAttempt {
    pub(crate) const fn new(
        generation: ApplyGeneration,
        window_id: WindowId,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperationKind,
        outcome: GpuiApplyOutcome,
    ) -> Self {
        Self {
            generation,
            window_id,
            transport_handle,
            operation,
            outcome,
        }
    }

    /// Returns the apply generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns stable logical target identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the resolved or created native handle.
    #[must_use]
    pub const fn transport_handle(&self) -> Option<&HostWindowHandle> {
        self.transport_handle.as_ref()
    }

    /// Returns operation category.
    #[must_use]
    pub const fn operation(&self) -> WindowOperationKind {
        self.operation
    }

    /// Returns success, partial failure, or dependency skip.
    #[must_use]
    pub const fn outcome(&self) -> &GpuiApplyOutcome {
        &self.outcome
    }
}

/// Fresh readback diff result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiApplyConvergence {
    /// Fresh evidence produced a valid remaining diff.
    Planned(WindowDiffReceipt),
    /// Fresh evidence could not be diffed.
    Invalid(WindowDiffError),
}

/// Complete post-apply readback result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuiApplyReadback {
    /// A complete fresh snapshot and its remaining diff.
    Complete {
        /// Fresh managed windows.
        observation: GpuiDesktopObservation,
        /// Remaining work derived only from the fresh snapshot.
        convergence: GpuiApplyConvergence,
    },
    /// A complete fresh snapshot was unavailable.
    Failed(GpuiObservationError),
}

/// Complete nontransactional GPUI apply receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuiApplyReceipt {
    plan: WindowDiffReceipt,
    attempts: Vec<GpuiApplyAttempt>,
    readback: GpuiApplyReadback,
}

impl GpuiApplyReceipt {
    pub(crate) const fn new(
        plan: WindowDiffReceipt,
        attempts: Vec<GpuiApplyAttempt>,
        readback: GpuiApplyReadback,
    ) -> Self {
        Self {
            plan,
            attempts,
            readback,
        }
    }

    /// Returns the original capability-aware plan.
    ///
    /// Its diagnostics carry every placement GPUI refused, because the pure
    /// planner turns a withheld capability into an `UnsupportedOperation`
    /// rather than an error.
    #[must_use]
    pub const fn plan(&self) -> &WindowDiffReceipt {
        &self.plan
    }

    /// Returns one result for every planned operation.
    #[must_use]
    pub fn attempts(&self) -> &[GpuiApplyAttempt] {
        &self.attempts
    }

    /// Returns complete fresh readback or its typed failure.
    #[must_use]
    pub const fn readback(&self) -> &GpuiApplyReadback {
        &self.readback
    }

    /// Returns whether fresh evidence has no remaining operations or diagnostics.
    #[must_use]
    pub fn is_converged(&self) -> bool {
        matches!(
            self.readback,
            GpuiApplyReadback::Complete {
                convergence: GpuiApplyConvergence::Planned(ref receipt),
                ..
            } if receipt.is_empty()
        )
    }
}
