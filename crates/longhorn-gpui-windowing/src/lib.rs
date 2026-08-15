//! GPUI host adapter: window create/destroy/observe, placement application,
//! lifecycle events, close handling, quiescence participation, and display
//! facts.
//!
//! This crate is the second implementation of contract 020's host boundary.
//! It composes the same pure planning `longhorn-windowing` produces; only
//! execution differs. Where a contract-020 requirement cannot be met on GPUI,
//! the type system says so rather than the adapter faking it — see
//! [`WITHHELD_CAPABILITIES`], [`UnobtainableDisplayFact`], and
//! [`GpuiDiagnosticDisposition`].
//!
//! The crate does not depend on `gpui`. Every GPUI value crossing the boundary
//! is a plain Rust type in `model`, and [`GpuiWindowBackend`] is the seam a
//! real GPUI application implements. `prototypes/gpui-windowing` binds that
//! seam to `gpui` itself and is where the shape below was measured.

mod apply;
mod backend;
mod capabilities;
mod error;
mod lifecycle;
mod model;
mod observe;
mod receipt;
mod registry;
mod scale;
mod transfer;

pub use apply::{GpuiApplyOutcomeBundle, GpuiDiagnosticDisposition, execute_gpui_window_apply};
pub use backend::{GpuiWindowBackend, GpuiWindowCreateRequest, GpuiWindowError};
pub use capabilities::{
    WITHHELD_CAPABILITIES, WithheldCapability, gpui_deferred_settlement, gpui_host_capabilities,
};
pub use error::{
    GpuiApplyError, GpuiDisplayError, GpuiObservationError, GpuiScaleFactorError,
    GpuiWindowLifecycleError, UnobtainableDisplayFact,
};
pub use lifecycle::{
    GpuiCloseDecision, GpuiLifecycleAction, GpuiLifecycleClock, GpuiLifecycleReceipt,
    GpuiLifecycleScheduler, GpuiUserCloseHandler, GpuiWindowCaptureBackend, GpuiWindowEvent,
    GpuiWindowLifecycleHost, GpuiWindowLifecycleServices, GpuiWindowQuiescenceProbe,
    NoopGpuiUserCloseHandler, capture_from_gpui_facts, close_is_safe, translate_gpui_window_event,
};
pub use model::{
    GpuiDisplayFacts, GpuiGeometryError, GpuiLogicalRect, GpuiLogicalSize, GpuiWindowBoundsState,
    GpuiWindowFacts, GpuiWindowKey,
};
pub use observe::{
    GPUI_DISPLAY_NAMESPACE, GpuiDesktopObservation, GpuiDisplayFactsSource, GpuiDisplayObservation,
    observe_gpui_desktop, observe_gpui_displays, observe_gpui_windows, project_gpui_display,
    project_gpui_window,
};
pub use receipt::{
    GpuiApplyAttempt, GpuiApplyConvergence, GpuiApplyFailure, GpuiApplyFailureKind,
    GpuiApplyOutcome, GpuiApplyReadback, GpuiApplyReceipt, GpuiWindowCall,
};
pub use registry::{
    GpuiApplyEvidence, GpuiWindowRegistry, GpuiWindowRegistryError, ManagedGpuiWindow,
};
pub use scale::scale_factor_from_gpui;
pub use transfer::live_transfer_windows;
