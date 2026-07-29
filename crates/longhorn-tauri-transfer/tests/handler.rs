//! Real handler-core behavior over deterministic managed-window readback.

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, LayoutContainerId, RegionId,
    ScreenPoint, ScreenRect, ScreenSize, TransferHostBindingId, TransferRequestId, WindowId,
};
use longhorn_tauri_transfer::{
    ManagedTransferSnapshot, ManagedTransferWindow, TransferHandlerAssembly, TransferHandlerError,
    TransferHandlerTeardownStatus, TransferProjectionError, TransferRuntimeError,
    project_client_point, project_client_rect,
};
use longhorn_transfer::{
    ClientDropZone, ClientEpoch, LeaseGeneration, MonotonicClock, TransferCapability,
    TransferDuration, TransferInstant, TransferLeaseRequest, TransferLeaseResponse, TransferLimits,
    TransferRevision, TransferTargetBinding,
};
use longhorn_windowing::HostWindowHandle;

#[derive(Clone, Copy)]
struct Clock;

impl MonotonicClock for Clock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(100)
    }
}

#[test]
fn projection_is_explicit_and_confined_to_current_content() {
    let window = managed_window("main", "window:main");
    let point = project_client_point(&window, ClientPoint::new(4.5, 8.49).unwrap()).unwrap();
    assert_eq!(point, ScreenPoint::new(105, 128));

    let rect = project_client_rect(
        &window,
        ClientRect::new(
            ClientPoint::new(10.25, 20.25).unwrap(),
            ClientSize::new(100.2, 50.2).unwrap(),
        ),
    )
    .unwrap();
    assert_eq!(
        rect,
        ScreenRect::new(ScreenPoint::new(110, 140), ScreenSize::new(101, 51))
    );

    assert_eq!(
        project_client_point(&window, ClientPoint::new(-1.0, 0.0).unwrap()),
        Err(TransferProjectionError::PointOutsideContent)
    );
    assert_eq!(
        project_client_rect(
            &window,
            ClientRect::new(
                ClientPoint::new(-1.0, 0.0).unwrap(),
                ClientSize::new(10.0, 10.0).unwrap(),
            ),
        ),
        Err(TransferProjectionError::RectangleOutsideContent)
    );
}

#[test]
fn mock_and_real_runtimes_share_epoch_lease_and_teardown_core() {
    let caller = handle("main");
    let snapshot = ManagedTransferSnapshot::new(
        &caller,
        [
            managed_window("main", "window:main"),
            managed_window("workspace", "window:workspace"),
        ],
    )
    .unwrap();
    let runtime = move |_caller: &HostWindowHandle| Ok(snapshot.clone());
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());

    let authority = handler.snapshot(&caller).unwrap();
    assert_eq!(authority.client_epoch(), ClientEpoch::new(1));

    let valid = lease_request(
        "request:lease-1",
        authority.client_id().clone(),
        authority.client_epoch(),
        1,
        ClientRect::new(
            ClientPoint::new(10.0, 10.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, valid).unwrap(),
        TransferLeaseResponse::Published { .. }
    ));

    let invalid = lease_request(
        "request:lease-2-bad",
        authority.client_id().clone(),
        authority.client_epoch(),
        2,
        ClientRect::new(
            ClientPoint::new(-20.0, 10.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, invalid).unwrap(),
        TransferLeaseResponse::Aborted { .. }
    ));

    let valid_second = lease_request(
        "request:lease-2",
        authority.client_id().clone(),
        authority.client_epoch(),
        2,
        ClientRect::new(
            ClientPoint::new(20.0, 20.0).unwrap(),
            ClientSize::new(100.0, 40.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, valid_second).unwrap(),
        TransferLeaseResponse::Published { .. }
    ));

    let teardown = handler.teardown().unwrap();
    assert_eq!(teardown.status(), TransferHandlerTeardownStatus::TornDown);
    assert_eq!(teardown.client_windows(), 1);
    assert_eq!(teardown.leases(), 1);
    assert_eq!(
        handler.teardown().unwrap().status(),
        TransferHandlerTeardownStatus::AlreadyTornDown
    );
    assert_eq!(
        handler.snapshot(&caller),
        Err(TransferHandlerError::Inactive)
    );
}

#[test]
fn each_snapshot_advances_epoch_and_invalidates_prior_renderer_authority() {
    let caller = handle("main");
    let snapshot =
        ManagedTransferSnapshot::new(&caller, [managed_window("main", "window:main")]).unwrap();
    let runtime = move |_caller: &HostWindowHandle| Ok(snapshot.clone());
    let handler = TransferHandlerAssembly::new(runtime, Clock, limits());

    let first = handler.snapshot(&caller).unwrap();
    handler
        .destroy_window(&WindowId::new("window:main").unwrap())
        .unwrap();
    let second = handler.snapshot(&caller).unwrap();
    assert_eq!(first.client_epoch(), ClientEpoch::new(1));
    assert_eq!(second.client_epoch(), ClientEpoch::new(2));
    assert_ne!(first.client_id(), second.client_id());

    let stale = lease_request(
        "request:stale",
        first.client_id().clone(),
        first.client_epoch(),
        1,
        ClientRect::new(
            ClientPoint::new(10.0, 10.0).unwrap(),
            ClientSize::new(20.0, 20.0).unwrap(),
        ),
    );
    assert!(matches!(
        handler.publish_lease(&caller, stale).unwrap(),
        TransferLeaseResponse::Aborted { .. }
    ));
}

#[test]
fn snapshot_validation_rejects_spoofed_or_ambiguous_callers() {
    let main = managed_window("main", "window:main");
    assert_eq!(
        ManagedTransferSnapshot::new(&handle("other"), [main.clone()]),
        Err(TransferRuntimeError::UnmanagedCaller(handle("other")))
    );
    assert_eq!(
        ManagedTransferSnapshot::new(&handle("main"), [main.clone(), main]),
        Err(TransferRuntimeError::DuplicateTransportHandle(handle(
            "main"
        )))
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

fn lease_request(
    request_id: &str,
    client_id: longhorn_core::TransferClientId,
    epoch: ClientEpoch,
    generation: u64,
    bounds: ClientRect,
) -> TransferLeaseRequest {
    TransferLeaseRequest::new(
        TransferRequestId::new(request_id).unwrap(),
        client_id,
        epoch,
        LeaseGeneration::new(generation),
        [ClientDropZone::new(
            DropZoneId::new("zone:primary").unwrap(),
            bounds,
            None,
            TransferCapability::MovePanel,
            TransferTargetBinding::PanelRegion {
                host_binding_id: TransferHostBindingId::new("binding:main").unwrap(),
                document_id: DomainId::new("layout.main").unwrap(),
                revision: TransferRevision::new(1),
                container_id: LayoutContainerId::new("container:main").unwrap(),
                region_id: RegionId::new("region:main").unwrap(),
            },
        )],
    )
}

fn managed_window(handle_value: &str, window_id: &str) -> ManagedTransferWindow {
    ManagedTransferWindow::new(
        WindowId::new(window_id).unwrap(),
        handle(handle_value),
        ScreenRect::new(ScreenPoint::new(90, 90), ScreenSize::new(1_000, 800)),
        ScreenRect::new(ScreenPoint::new(100, 120), ScreenSize::new(800, 600)),
    )
}

fn handle(value: &str) -> HostWindowHandle {
    HostWindowHandle::new(value).unwrap()
}
