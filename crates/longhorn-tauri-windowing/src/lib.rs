//! Checked Tauri display and managed-window observation.

mod apply;
mod composition;
mod error;
mod lifecycle;
mod mapping;
mod model;
mod probe;
mod restore;
mod scale;

pub use apply::{
    ApplyConvergence, ApplyReadback, ManagedDesktopReadback, ManagedWindowRegistry,
    ManagedWindowRegistryError, NativeWindowCall, NativeWindowMutationError, NoWindowFactory,
    ProgrammaticApplyEvidence, TauriApplyError, TauriApplyOutcome, TauriApplyReceipt,
    TauriDesktopReadback, TauriDispatchError, TauriWindowFactory, TauriWindowMutationBackend,
    WindowApplyAttempt, WindowApplyFailure, WindowApplyFailureKind, WindowApplyOutcome,
    WindowFactoryError, WindowMutationBackend, dispatch_tauri_window_apply,
    execute_tauri_window_apply, execute_tauri_window_apply_in_place, tauri_deferred_settlement,
    tauri_host_capabilities,
};
pub use composition::{
    PredeclaredTauriWindow, TauriWindowHost, TauriWindowHostApplyReceipt, TauriWindowHostError,
    TauriWindowHostInitialization, TauriWindowHostInitializationError,
    TauriWindowHostInitializationReceipt, TauriWindowHostInitializationStatus,
    TauriWindowHostTeardownReceipt, TauriWindowHostTeardownStatus, TauriWindowRegistrationReceipt,
    assemble_tauri_single_window_lifecycle_host, assemble_tauri_window_host,
};
pub use error::{
    DesktopMappingError, DesktopObservationError, HostProbeOperation, ProbeTarget,
    TauriObservationError, TauriProbeError, TauriScaleFactorError,
};
pub use lifecycle::{
    CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement,
    NoopWindowLifecycleReporter, NoopWindowUserCloseHandler, ProcessMonotonicClock,
    ProgrammaticApplyObserver, ScheduledWindowLifecycleWake, TauriAsyncWindowLifecycleScheduler,
    TauriWindowCaptureBackend, TauriWindowLifecycleAction, TauriWindowLifecycleError,
    TauriWindowLifecycleHost, TauriWindowLifecycleReceipt, TauriWindowLifecycleServices,
    TauriWindowRevealBackend, UniformWindowGeometryMapper, WindowCaptureBackend,
    WindowFlushOutcome, WindowFlushRequest, WindowFlushScope, WindowFlushTarget,
    WindowGeometryMapper, WindowLifecycleClock, WindowLifecycleReport, WindowLifecycleReporter,
    WindowLifecycleScheduler, WindowLifecycleWakeHandler, WindowPlacementFlushCompletion,
    WindowPlacementFlushTicket, WindowPlacementSink, WindowRevealBackend, WindowRevealReceipt,
    WindowRevealStatus, WindowScaleGeometryMapper, WindowShutdownReceipt, WindowUserCloseHandler,
    translate_tauri_window_event,
};
pub use mapping::{
    DesktopCoordinateMapper, LogicalLayoutMapper, MappedDesktopGeometry, MappedDisplayGeometry,
    MappedWindowGeometry, PlatformDesktopMapper, UniformScaleMapper, project_desktop,
};
pub use model::{
    DefaultDisplayMetadata, DesktopObservation, DisplayMetadataProvider,
    DisplayObservationMetadata, ManagedWebviewWindow, PhysicalDesktopSnapshot,
    PhysicalDisplayObservation, PhysicalLiveWindowObservation, PhysicalMonitorFacts,
};
pub use probe::{
    observe_tauri_desktop, observe_tauri_desktop_with, probe_managed_windows, probe_tauri_desktop,
};
pub use restore::{
    TauriWindowRestore, TauriWindowRestoreError, plan_tauri_window_restore,
    plan_window_restore_from_observation,
};
pub use scale::scale_factor_from_tauri;
