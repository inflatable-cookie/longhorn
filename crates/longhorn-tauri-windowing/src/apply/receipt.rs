use longhorn_core::WindowId;
use longhorn_windowing::{
    ApplyGeneration, HostWindowHandle, WindowDiffError, WindowDiffReceipt, WindowOperation,
    WindowOperationKind,
};

use crate::{DesktopObservation, TauriObservationError};

/// One generation marker registered before a native mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgrammaticApplyEvidence {
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperation,
}

impl ProgrammaticApplyEvidence {
    pub(crate) fn new(
        generation: ApplyGeneration,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperation,
    ) -> Self {
        Self {
            generation,
            window_id: operation.window_id().clone(),
            transport_handle,
            operation,
        }
    }

    /// Returns the owning apply generation.
    #[must_use]
    pub const fn generation(&self) -> ApplyGeneration {
        self.generation
    }

    /// Returns stable logical target identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the known native handle.
    #[must_use]
    pub const fn transport_handle(&self) -> Option<&HostWindowHandle> {
        self.transport_handle.as_ref()
    }

    /// Returns the complete expected operation registered before native mutation.
    #[must_use]
    pub const fn operation(&self) -> &WindowOperation {
        &self.operation
    }

    /// Returns the originating operation category.
    #[must_use]
    pub const fn operation_kind(&self) -> WindowOperationKind {
        self.operation.kind()
    }
}

/// One discrete native or registry call within an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeWindowCall {
    /// Resolve logical and transport identity.
    ResolveManagedWindow,
    /// Invoke the consumer window factory.
    FactoryCreate,
    /// Verify the factory returned a hidden window.
    ValidateHidden,
    /// Verify the factory returned an unmaximized window.
    ValidateUnmaximized,
    /// Insert a successfully created managed slot.
    RegistryInsert,
    /// Install lifecycle handling for a newly managed native window.
    InstallLifecycleListener,
    /// Retag managed bookkeeping.
    RegistryRetag,
    /// Enforce protected-primary close policy.
    ProtectPrimary,
    /// Leave maximized state.
    Unmaximize,
    /// Set outer-frame position.
    SetOuterPosition,
    /// Set inner-content size.
    SetInnerSize,
    /// Enter maximized state.
    Maximize,
    /// Show.
    Show,
    /// Hide.
    Hide,
    /// Focus.
    Focus,
    /// Request close.
    Close,
}

/// Stable category for one failed native operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowApplyFailureKind {
    /// Registry identity or generation invariant failed.
    Registry,
    /// Consumer factory failed or violated its neutral-slot contract.
    Factory,
    /// Tauri host call failed.
    Native,
    /// Protected-primary policy refused close.
    ProtectedPrimary,
}

/// Typed operation failure with the exact failed call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowApplyFailure {
    call: NativeWindowCall,
    kind: WindowApplyFailureKind,
    detail: String,
}

impl WindowApplyFailure {
    pub(crate) fn new(
        call: NativeWindowCall,
        kind: WindowApplyFailureKind,
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
    pub const fn call(&self) -> NativeWindowCall {
        self.call
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> WindowApplyFailureKind {
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
pub enum WindowApplyOutcome {
    /// Every constituent call succeeded.
    Succeeded {
        /// Calls completed in order.
        completed_calls: Vec<NativeWindowCall>,
    },
    /// Some calls succeeded before one failed.
    Failed {
        /// Calls completed before failure.
        completed_calls: Vec<NativeWindowCall>,
        /// Exact failure.
        failure: WindowApplyFailure,
    },
    /// An earlier failed operation blocked this same logical window.
    DependencySkipped {
        /// Earlier operation category.
        blocked_by: WindowOperationKind,
    },
}

/// Execution record for one planned operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowApplyAttempt {
    generation: ApplyGeneration,
    window_id: WindowId,
    transport_handle: Option<HostWindowHandle>,
    operation: WindowOperationKind,
    outcome: WindowApplyOutcome,
}

impl WindowApplyAttempt {
    pub(crate) const fn new(
        generation: ApplyGeneration,
        window_id: WindowId,
        transport_handle: Option<HostWindowHandle>,
        operation: WindowOperationKind,
        outcome: WindowApplyOutcome,
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

    /// Returns the resolved or supplied native handle.
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
    pub const fn outcome(&self) -> &WindowApplyOutcome {
        &self.outcome
    }
}

/// Fresh readback diff result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyConvergence {
    /// Fresh evidence produced a valid remaining diff.
    Planned(WindowDiffReceipt),
    /// Fresh evidence could not be diffed.
    Invalid(WindowDiffError),
}

/// Complete post-apply readback result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyReadback {
    /// A complete fresh snapshot and its remaining diff.
    Complete {
        /// Fresh displays and managed windows.
        observation: DesktopObservation,
        /// Remaining work derived only from the fresh snapshot.
        convergence: ApplyConvergence,
    },
    /// A complete fresh snapshot was unavailable.
    Failed(TauriObservationError),
}

/// Complete nontransactional native apply receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriApplyReceipt {
    plan: WindowDiffReceipt,
    attempts: Vec<WindowApplyAttempt>,
    readback: ApplyReadback,
}

impl TauriApplyReceipt {
    pub(crate) const fn new(
        plan: WindowDiffReceipt,
        attempts: Vec<WindowApplyAttempt>,
        readback: ApplyReadback,
    ) -> Self {
        Self {
            plan,
            attempts,
            readback,
        }
    }

    /// Returns the original capability-aware plan.
    #[must_use]
    pub const fn plan(&self) -> &WindowDiffReceipt {
        &self.plan
    }

    /// Returns one result for every planned operation.
    #[must_use]
    pub fn attempts(&self) -> &[WindowApplyAttempt] {
        &self.attempts
    }

    /// Returns complete fresh readback or its typed failure.
    #[must_use]
    pub const fn readback(&self) -> &ApplyReadback {
        &self.readback
    }

    /// Returns whether fresh evidence has no remaining operations or diagnostics.
    #[must_use]
    pub fn is_converged(&self) -> bool {
        matches!(
            self.readback,
            ApplyReadback::Complete {
                convergence: ApplyConvergence::Planned(ref receipt),
                ..
            } if receipt.is_empty()
        )
    }
}
