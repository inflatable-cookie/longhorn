//! Contract 020: cross-window transfer.
//!
//! The last claim the contract recorded as unproven on either backend. The
//! host's whole contribution to a cross-window drag is where every managed
//! window currently is; `TransferCoordinator` takes that list and decides the
//! rest. So the question this answers is narrow and exact: **can the GPUI
//! host produce a window list the transfer core resolves a drop against, and
//! does a point in one window resolve to that window rather than the source?**
//!
//! What is still not proved is a real drag: GPUI mouse events bound to a
//! session, moving under the cursor, released over another window. That wants
//! a target application, and the ceiling is stated rather than papered over.

use longhorn_core::{
    DomainId, LayoutContainerId, RegionId, ScreenPoint, ScreenRect, ScreenSize, WindowId,
};
use longhorn_gpui_windowing::{
    GpuiLogicalRect, GpuiLogicalSize, GpuiWindowKey, live_transfer_windows,
};
use longhorn_transfer::{
    ClientEpoch, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone, DropZoneId,
    LeaseGeneration, LeasePublication, MonotonicClock, TargetSelector, TerminalTransferResolution,
    TransferCapability, TransferClientId, TransferCoordinator, TransferDuration,
    TransferHostBindingId, TransferInstant, TransferLimits, TransferRevision,
    TransferSessionRequest, TransferSourceAuthority, TransferSubjectId, TransferTargetBinding,
};

use super::support::FakeGpuiHost;

struct ZeroClock;

impl MonotonicClock for ZeroClock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(0)
    }
}

struct OneId;

impl DragSessionIdAllocator for OneId {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        Ok([3; 16])
    }
}

fn window(value: &str) -> WindowId {
    WindowId::new(value).expect("window id")
}

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

/// Two windows side by side, as a GPUI host would report them.
fn host_with_two_windows() -> (FakeGpuiHost, Vec<(WindowId, GpuiWindowKey)>) {
    let (host, source) = FakeGpuiHost::new().with_existing_window(
        GpuiLogicalRect::new(0.0, 0.0, 400.0, 300.0),
        GpuiLogicalSize::new(400.0, 300.0),
        false,
    );
    let (host, target) = host.with_existing_window(
        GpuiLogicalRect::new(500.0, 0.0, 400.0, 300.0),
        GpuiLogicalSize::new(400.0, 300.0),
        false,
    );

    (
        host,
        vec![
            (window("window:source"), source),
            (window("window:target"), target),
        ],
    )
}

#[test]
fn observed_windows_become_the_list_the_transfer_core_reads() {
    let (mut host, windows) = host_with_two_windows();

    let live = live_transfer_windows(&mut host, windows.iter().map(|(id, key)| (id, *key)))
        .expect("every window observes");

    assert_eq!(live.len(), 2);
    assert_eq!(live[0].window_id(), &window("window:source"));
    assert_eq!(live[0].outer_bounds(), rect(0, 0, 400, 300));
    assert_eq!(live[1].window_id(), &window("window:target"));
    assert_eq!(live[1].outer_bounds(), rect(500, 0, 400, 300));
}

#[test]
fn one_unobservable_window_fails_the_whole_list() {
    // A silently short list resolves a drop against a desktop missing a
    // window, which reads as "no target" and loses the transfer with no
    // diagnostic. Failing loudly is the only honest answer.
    let (mut host, mut windows) = host_with_two_windows();
    windows.push((window("window:ghost"), GpuiWindowKey::new(9_999)));

    let error = live_transfer_windows(&mut host, windows.iter().map(|(id, key)| (id, *key)))
        .expect_err("an unknown key is refused");
    assert!(!error.detail().is_empty());
}

#[test]
fn a_point_in_another_window_resolves_to_that_window() {
    // The cross-window claim itself: a drag that started in the source window
    // and ended over the target window resolves to the target's zone, using
    // only geometry the GPUI host observed.
    let (mut host, windows) = host_with_two_windows();
    let live = live_transfer_windows(&mut host, windows.iter().map(|(id, key)| (id, *key)))
        .expect("every window observes");

    let clock = ZeroClock;
    let mut coordinator = TransferCoordinator::new(
        TransferLimits::new(
            4,
            4,
            4,
            4,
            8,
            TransferDuration::new(100),
            TransferDuration::new(50),
        )
        .expect("limits"),
    );

    for (window_id, client_id) in [
        ("window:source", "client:source"),
        ("window:target", "client:target"),
    ] {
        coordinator
            .bind_client_epoch(
                &clock,
                window(window_id),
                TransferClientId::new(client_id).expect("client id"),
                ClientEpoch::new(1),
            )
            .expect("epoch binds");
    }

    // The target window publishes a zone in its own coordinates. Note the
    // origin: 520 is inside the target window and outside the source, which is
    // the whole point of the test.
    coordinator
        .publish_lease(
            &clock,
            LeasePublication::new(
                window("window:target"),
                TransferClientId::new("client:target").expect("client id"),
                ClientEpoch::new(1),
                LeaseGeneration::new(1),
                TransferDuration::new(30),
                rect(500, 0, 400, 300),
                vec![DropZone::new(
                    DropZoneId::new("zone:main").expect("zone id"),
                    rect(520, 20, 200, 200),
                    None,
                    TransferCapability::MovePanel,
                    TransferTargetBinding::PanelRegion {
                        host_binding_id: TransferHostBindingId::new("host:target")
                            .expect("binding"),
                        document_id: DomainId::new("layout.workspace").expect("document"),
                        revision: TransferRevision::new(9),
                        container_id: LayoutContainerId::new("container:target")
                            .expect("container"),
                        region_id: RegionId::new("region:main").expect("region"),
                    },
                )],
            ),
        )
        .expect("lease publishes");

    let session = coordinator
        .create_session(
            &clock,
            &mut OneId,
            TransferSessionRequest::new(
                TransferSourceAuthority::Panel {
                    client_id: TransferClientId::new("client:source").expect("client id"),
                    client_epoch: ClientEpoch::new(1),
                    source_window_id: window("window:source"),
                    subject_id: TransferSubjectId::new("panel:inspector").expect("subject"),
                    host_binding_id: TransferHostBindingId::new("host:source").expect("binding"),
                    document_id: DomainId::new("layout.workspace").expect("document"),
                    revision: TransferRevision::new(7),
                    container_id: LayoutContainerId::new("container:source").expect("container"),
                    region_id: RegionId::new("region:tools").expect("region"),
                },
                TransferDuration::new(50),
            ),
        )
        .expect("session is created")
        .payload()
        .session_id();

    let resolution = coordinator
        .attempt_target_or_empty_display(
            &clock,
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(600, 100)),
            &live,
        )
        .expect("the point resolves");

    let TerminalTransferResolution::Target(attempt) = resolution else {
        panic!("a point inside the target window resolved to an empty display");
    };
    assert_eq!(attempt.target().window_id(), &window("window:target"));
}

#[test]
fn a_point_outside_every_observed_window_is_an_empty_display() {
    // The other half: the host's window list is what tells the core a drop
    // landed on bare desktop. A list missing a window would make this arm fire
    // for a point that was actually inside one.
    let (mut host, windows) = host_with_two_windows();
    let live = live_transfer_windows(&mut host, windows.iter().map(|(id, key)| (id, *key)))
        .expect("every window observes");

    let clock = ZeroClock;
    let mut coordinator = TransferCoordinator::new(
        TransferLimits::new(
            4,
            4,
            4,
            4,
            8,
            TransferDuration::new(100),
            TransferDuration::new(50),
        )
        .expect("limits"),
    );
    coordinator
        .bind_client_epoch(
            &clock,
            window("window:source"),
            TransferClientId::new("client:source").expect("client id"),
            ClientEpoch::new(1),
        )
        .expect("epoch binds");

    let session = coordinator
        .create_session(
            &clock,
            &mut OneId,
            TransferSessionRequest::new(
                TransferSourceAuthority::Panel {
                    client_id: TransferClientId::new("client:source").expect("client id"),
                    client_epoch: ClientEpoch::new(1),
                    source_window_id: window("window:source"),
                    subject_id: TransferSubjectId::new("panel:inspector").expect("subject"),
                    host_binding_id: TransferHostBindingId::new("host:source").expect("binding"),
                    document_id: DomainId::new("layout.workspace").expect("document"),
                    revision: TransferRevision::new(7),
                    container_id: LayoutContainerId::new("container:source").expect("container"),
                    region_id: RegionId::new("region:tools").expect("region"),
                },
                TransferDuration::new(50),
            ),
        )
        .expect("session is created")
        .payload()
        .session_id();

    // Between the two windows, on the desktop.
    let resolution = coordinator
        .attempt_target_or_empty_display(
            &clock,
            session,
            TargetSelector::ScreenPoint(ScreenPoint::new(450, 100)),
            &live,
        )
        .expect("the point resolves");

    assert!(matches!(
        resolution,
        TerminalTransferResolution::EmptyDisplay(_)
    ));
}
