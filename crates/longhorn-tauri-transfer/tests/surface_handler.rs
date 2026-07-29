//! Optional whole-Surface commands over the same base handler authority.

#![cfg(feature = "surface-transfer")]

use std::sync::{Arc, Mutex};

use longhorn_core::{ScreenPoint, ScreenRect, ScreenSize, SurfaceId, TransferRequestId, WindowId};
use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferCommand, SurfaceTransferResponse,
};
use longhorn_tauri_transfer::{
    AssembledSurfaceTransferCommands, AssembledTransferCommands, ManagedTransferSnapshot,
    ManagedTransferWindow, PanelTransferAdapter, SurfaceTransferAdapter,
    SurfaceTransferCommandService, TransferCallerAuthority, TransferCommandService,
    TransferHandlerAssembly,
};
use longhorn_transfer::{
    DragSessionId, LiveTransferWindow, MonotonicClock, PanelSessionStartRequest,
    PanelTransferCommand, PanelTransferResponse, TargetSelector, TransferAbort,
    TransferCommitSelector, TransferCoordinator, TransferDuration, TransferErrorCode,
    TransferInstant, TransferLimits, TransferSessionResponse,
};
use longhorn_windowing::HostWindowHandle;

#[derive(Clone, Copy)]
struct Clock;

impl MonotonicClock for Clock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(100)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurfaceCall {
    window_id: WindowId,
    client_windows: usize,
    selector: Option<ScreenPoint>,
    live_windows: usize,
}

struct RecordingSurface {
    calls: Arc<Mutex<Vec<SurfaceCall>>>,
}

impl SurfaceTransferAdapter for RecordingSurface {
    fn start_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
    ) -> SurfaceSessionResponse {
        self.calls.lock().unwrap().push(SurfaceCall {
            window_id: caller.window_id().clone(),
            client_windows: coordinator.client_window_count(),
            selector: None,
            live_windows: 0,
        });
        SurfaceSessionResponse::Aborted {
            abort: SurfaceTransferAbort::host_transfer(
                request.request_id().clone(),
                TransferErrorCode::NoTarget,
                "recording adapter",
            ),
        }
    }

    fn commit_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        selector: TargetSelector,
        live_windows: Vec<LiveTransferWindow>,
    ) -> SurfaceTransferResponse {
        let selector = match selector {
            TargetSelector::ScreenPoint(point) => Some(point),
            TargetSelector::ExplicitZone(_) => None,
        };
        self.calls.lock().unwrap().push(SurfaceCall {
            window_id: caller.window_id().clone(),
            client_windows: coordinator.client_window_count(),
            selector,
            live_windows: live_windows.len(),
        });
        SurfaceTransferResponse::Aborted {
            abort: SurfaceTransferAbort::host_transfer(
                request.request_id().clone(),
                TransferErrorCode::NoTarget,
                "recording adapter",
            ),
        }
    }
}

struct NoopPanel;

impl PanelTransferAdapter for NoopPanel {
    fn start_panel(
        &mut self,
        _coordinator: &mut TransferCoordinator,
        _caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
    ) -> TransferSessionResponse {
        TransferSessionResponse::Aborted {
            abort: TransferAbort::host_transfer(
                request.request_id().clone(),
                TransferErrorCode::NoTarget,
                "unused",
            ),
        }
    }

    fn commit_panel(
        &mut self,
        _coordinator: &mut TransferCoordinator,
        _caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        _selector: TargetSelector,
        _live_windows: Vec<LiveTransferWindow>,
    ) -> PanelTransferResponse {
        PanelTransferResponse::Aborted {
            abort: TransferAbort::host_transfer(
                request.request_id().clone(),
                TransferErrorCode::NoTarget,
                "unused",
            ),
        }
    }
}

#[test]
fn optional_surface_commands_share_base_epoch_and_projection_authority() {
    let caller = HostWindowHandle::new("main").unwrap();
    let snapshot = ManagedTransferSnapshot::new(
        &caller,
        [ManagedTransferWindow::new(
            WindowId::new("window:main").unwrap(),
            caller.clone(),
            ScreenRect::new(ScreenPoint::new(90, 90), ScreenSize::new(1_000, 800)),
            ScreenRect::new(ScreenPoint::new(100, 120), ScreenSize::new(800, 600)),
        )],
    )
    .unwrap();
    let runtime = move |_caller: &HostWindowHandle| Ok(snapshot.clone());
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());
    let base = AssembledTransferCommands::new(handler, NoopPanel);
    TransferCommandService::snapshot(&base, &caller).unwrap();

    let calls = Arc::new(Mutex::new(Vec::new()));
    let surface = AssembledSurfaceTransferCommands::new(
        base.shared_handler(),
        RecordingSurface {
            calls: calls.clone(),
        },
    );
    assert!(matches!(
        surface
            .start_surface(
                &caller,
                SurfaceSessionStartRequest::new(
                    TransferRequestId::new("request:surface-start").unwrap(),
                    SurfaceId::new("surface:main").unwrap(),
                ),
            )
            .unwrap(),
        SurfaceSessionResponse::Aborted { .. }
    ));
    assert!(matches!(
        surface
            .commit_surface(
                &caller,
                SurfaceTransferCommand::new(
                    TransferRequestId::new("request:surface-commit").unwrap(),
                    DragSessionId::from_entropy([7; 16]),
                    TransferCommitSelector::ScreenPoint {
                        point: ScreenPoint::new(10, 15),
                    },
                ),
            )
            .unwrap(),
        SurfaceTransferResponse::Aborted { .. }
    ));

    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            SurfaceCall {
                window_id: WindowId::new("window:main").unwrap(),
                client_windows: 1,
                selector: None,
                live_windows: 0,
            },
            SurfaceCall {
                window_id: WindowId::new("window:main").unwrap(),
                client_windows: 1,
                selector: Some(ScreenPoint::new(10, 15)),
                live_windows: 1,
            },
        ]
    );
}

fn limits() -> TransferLimits {
    TransferLimits::new(
        8,
        4,
        4,
        8,
        100,
        TransferDuration::new(1_000),
        TransferDuration::new(250),
    )
    .unwrap()
}
