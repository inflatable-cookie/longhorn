//! Real Tauri window-host and transfer-handler assembly.

use std::sync::Arc;

use longhorn_core::{ScreenPoint, ScreenSize, WindowId, WindowPlacement};
#[cfg(feature = "surface-mode")]
use longhorn_tauri_transfer::{AssembledSurfaceTransferCommands, SurfaceTransferCommandService};
use longhorn_tauri_transfer::{
    AssembledTransferCommands, TauriTransferRuntime, TransferCommandService,
    TransferHandlerAssembly,
};
use longhorn_tauri_windowing::{
    NoopWindowLifecycleReporter, NoopWindowUserCloseHandler, PredeclaredTauriWindow,
    ProcessMonotonicClock, TauriAsyncWindowLifecycleScheduler, TauriWindowCaptureBackend,
    TauriWindowHost, TauriWindowLifecycleServices, TauriWindowRevealBackend, UniformScaleMapper,
    UniformWindowGeometryMapper, WindowFlushRequest, WindowPlacementFlushTicket,
    WindowPlacementSink, assemble_tauri_window_host, scale_factor_from_tauri,
};
use longhorn_transfer::{TransferDuration, TransferLimits};
use longhorn_windowing::{WindowLifecycleDuration, WindowLifecyclePolicy};
use tauri::{AppHandle, Manager, Wry};

use crate::domain::{
    ProofClock, ProofDomains, ProofPanelAdapter, SOURCE_WINDOW_ID, TARGET_WINDOW_ID,
};

pub(crate) struct ProofHost {
    pub(crate) window_host: Arc<TauriWindowHost<Wry>>,
    pub(crate) transfer: Arc<dyn TransferCommandService>,
    #[cfg(feature = "surface-mode")]
    pub(crate) surface_transfer: Arc<dyn SurfaceTransferCommandService>,
    #[cfg(feature = "surface-mode")]
    pub(crate) screen_policy: crate::surface::ScreenPolicy,
}

#[derive(Default)]
struct NoopPlacementSink;

impl WindowPlacementSink for NoopPlacementSink {
    fn stage(
        &self,
        _placement: &longhorn_tauri_windowing::CapturedWindowPlacement,
    ) -> Result<(), String> {
        Ok(())
    }

    fn request_flush(
        &self,
        _request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        Ok(WindowPlacementFlushTicket::completed())
    }
}

pub(crate) fn assemble(
    app: &AppHandle<Wry>,
    domains: Arc<ProofDomains>,
    transfer_clock: ProofClock,
) -> Result<ProofHost, String> {
    let source = app
        .get_webview_window(SOURCE_WINDOW_ID)
        .ok_or_else(|| "predeclared source window is missing".to_string())?;
    let target = app
        .get_webview_window(TARGET_WINDOW_ID)
        .ok_or_else(|| "predeclared target window is missing".to_string())?;
    let scale = scale_factor_from_tauri(source.scale_factor().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mapper = Arc::new(UniformWindowGeometryMapper::new(scale));
    let lifecycle_clock = Arc::new(ProcessMonotonicClock::new());
    let services = TauriWindowLifecycleServices::new(
        lifecycle_clock.clone(),
        Arc::new(TauriAsyncWindowLifecycleScheduler::new(lifecycle_clock)),
        mapper.clone(),
        Arc::new(TauriWindowCaptureBackend::new(mapper)),
        Arc::new(NoopPlacementSink),
        Arc::new(NoopWindowUserCloseHandler),
        Arc::new(NoopWindowLifecycleReporter),
        Arc::new(TauriWindowRevealBackend),
    );
    let initialization = assemble_tauri_window_host(
        app,
        WindowLifecyclePolicy::new(
            WindowLifecycleDuration::from_millis(100),
            WindowLifecycleDuration::from_millis(100),
            WindowLifecycleDuration::from_millis(250),
            WindowLifecycleDuration::from_millis(250),
            WindowLifecycleDuration::from_millis(1_000),
        ),
        services,
        [
            PredeclaredTauriWindow::new(window_id(SOURCE_WINDOW_ID), source)
                .with_initial_normal(placement(40, 80)),
            PredeclaredTauriWindow::new(window_id(TARGET_WINDOW_ID), target)
                .with_initial_normal(placement(660, 80)),
        ],
        None,
    )
    .map_err(|error| format!("{error:?}"))?;
    let (window_host, _) = initialization.into_parts();
    let runtime = TauriTransferRuntime::new(window_host.clone(), UniformScaleMapper);
    let handler = TransferHandlerAssembly::new(runtime, transfer_clock.clone(), transfer_limits());
    let base = Arc::new(AssembledTransferCommands::new(
        handler,
        ProofPanelAdapter::new(domains.clone(), transfer_clock.clone()),
    ));
    #[cfg(feature = "surface-mode")]
    let screen_policy = crate::surface::ScreenPolicy::from_app(app)?;
    #[cfg(feature = "surface-mode")]
    let surface_transfer: Arc<dyn SurfaceTransferCommandService> =
        Arc::new(AssembledSurfaceTransferCommands::new(
            base.shared_handler(),
            crate::surface::ProofSurfaceAdapter::new(
                app.clone(),
                window_host.clone(),
                domains,
                transfer_clock,
                &screen_policy,
            ),
        ));
    let transfer: Arc<dyn TransferCommandService> = base;
    Ok(ProofHost {
        window_host,
        transfer,
        #[cfg(feature = "surface-mode")]
        surface_transfer,
        #[cfg(feature = "surface-mode")]
        screen_policy,
    })
}

fn transfer_limits() -> TransferLimits {
    TransferLimits::new(
        32,
        8,
        16,
        32,
        256,
        TransferDuration::new(5_000),
        TransferDuration::new(2_000),
    )
    .expect("proof transfer limits are finite and valid")
}

fn placement(x: i32, y: i32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(560, 500))
}

fn window_id(value: &str) -> WindowId {
    WindowId::new(value).expect("proof window ids use the opaque-id grammar")
}
