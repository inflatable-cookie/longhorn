//! Optional whole-Surface commands over the same base handler authority.

#![cfg(feature = "surface-transfer")]

use std::sync::{Arc, Mutex};

use longhorn_core::{
    ScreenPoint, ScreenRect, ScreenSize, SurfaceId, TransferHostBindingId, TransferRequestId,
    TransferSubjectId, WindowId,
};
use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferCommand, SurfaceTransferResponse,
};
use longhorn_tauri_transfer::{
    AssembledSurfaceTransferCommands, AssembledTransferCommands, ManagedTransferSnapshot,
    ManagedTransferWindow, PanelTransferAdapter, SurfaceTransferAdapter, TransferCallerAuthority,
    TransferHandlerAssembly, TransferSessionRegistrar,
};
use longhorn_transfer::{
    DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator, MonotonicClock,
    PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse, TerminalTransferAttempt,
    TerminalTransferResolution, TransferAbort, TransferCommitSelector, TransferDuration,
    TransferErrorCode, TransferInstant, TransferLimits, TransferRevision, TransferSessionRequest,
    TransferSessionResponse, TransferSessionStarted, TransferSourceAuthority,
};
use longhorn_windowing::HostWindowHandle;

#[derive(Clone, Copy)]
struct Clock;

impl MonotonicClock for Clock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(100)
    }
}

struct FixedAllocator;

impl DragSessionIdAllocator for FixedAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        Ok([7; 16])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurfaceCall {
    window_id: WindowId,
    selector: Option<ScreenPoint>,
}

struct RecordingSurface {
    calls: Arc<Mutex<Vec<SurfaceCall>>>,
}

impl SurfaceTransferAdapter for RecordingSurface {
    fn start_surface(
        &self,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
        sessions: &mut TransferSessionRegistrar<'_>,
    ) -> SurfaceSessionResponse {
        self.calls.lock().unwrap().push(SurfaceCall {
            window_id: caller.window_id().clone(),
            selector: None,
        });
        let source = TransferSourceAuthority::Surface {
            client_id: caller.client_id().clone(),
            client_epoch: caller.client_epoch(),
            source_window_id: caller.window_id().clone(),
            subject_id: TransferSubjectId::from(request.surface_id()),
            host_binding_id: TransferHostBindingId::new("binding:main").unwrap(),
            document_id: longhorn_core::DomainId::new("layout.main").unwrap(),
            revision: TransferRevision::new(1),
        };
        let mut allocator = FixedAllocator;
        let receipt = sessions
            .create_session(
                &mut allocator,
                TransferSessionRequest::new(source, caller.session_lifetime()),
            )
            .unwrap()
            .unwrap();
        SurfaceSessionResponse::Started {
            session: TransferSessionStarted::from_domain(request.request_id().clone(), receipt),
        }
    }

    fn commit_surface(
        &self,
        caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        resolution: TerminalTransferResolution,
    ) -> SurfaceTransferResponse {
        let selector = match resolution {
            TerminalTransferResolution::EmptyDisplay(attempt) => Some(attempt.screen_point()),
            TerminalTransferResolution::Target(_) => None,
        };
        self.calls.lock().unwrap().push(SurfaceCall {
            window_id: caller.window_id().clone(),
            selector,
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
        &self,
        _caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
        _sessions: &mut TransferSessionRegistrar<'_>,
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
        &self,
        _caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        _attempt: TerminalTransferAttempt,
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
    base.snapshot(&caller).unwrap();

    let calls = Arc::new(Mutex::new(Vec::new()));
    let surface = AssembledSurfaceTransferCommands::new(
        base.shared_handler(),
        RecordingSurface {
            calls: calls.clone(),
        },
    );
    let started = surface
        .start_surface(
            &caller,
            SurfaceSessionStartRequest::new(
                TransferRequestId::new("request:surface-start").unwrap(),
                SurfaceId::new("surface:main").unwrap(),
            ),
        )
        .unwrap();
    let SurfaceSessionResponse::Started { .. } = started else {
        panic!("expected started surface session");
    };
    let session_id = DragSessionId::from_entropy([7; 16]);

    assert!(matches!(
        surface
            .commit_surface(
                &caller,
                SurfaceTransferCommand::new(
                    TransferRequestId::new("request:surface-commit").unwrap(),
                    session_id,
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
                selector: None,
            },
            SurfaceCall {
                window_id: WindowId::new("window:main").unwrap(),
                selector: Some(ScreenPoint::new(10, 15)),
            },
        ]
    );
}

#[test]
fn surface_commit_reports_consumed_session_when_target_resolution_fails() {
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
    base.snapshot(&caller).unwrap();

    let surface = AssembledSurfaceTransferCommands::new(
        base.shared_handler(),
        RecordingSurface {
            calls: Arc::new(Mutex::new(Vec::new())),
        },
    );
    let started = surface
        .start_surface(
            &caller,
            SurfaceSessionStartRequest::new(
                TransferRequestId::new("request:failed-surface-start").unwrap(),
                SurfaceId::new("surface:failed").unwrap(),
            ),
        )
        .unwrap();
    let SurfaceSessionResponse::Started { .. } = started else {
        panic!("expected started surface session");
    };

    let response = surface
        .commit_surface(
            &caller,
            SurfaceTransferCommand::new(
                TransferRequestId::new("request:failed-surface-commit").unwrap(),
                DragSessionId::from_entropy([7; 16]),
                TransferCommitSelector::ExplicitZone {
                    drop_zone_id: longhorn_core::DropZoneId::new("zone:missing").unwrap(),
                },
            ),
        )
        .unwrap();
    let SurfaceTransferResponse::Aborted { abort } = response else {
        panic!("expected failed target resolution to abort");
    };

    assert!(abort.session_consumed());
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
