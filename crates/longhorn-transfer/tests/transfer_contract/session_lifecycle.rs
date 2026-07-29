use longhorn_transfer::{
    ClientEpoch, SessionCancellationStatus, TargetSelector, TransferDuration, TransferErrorCode,
    TransferSessionRequest,
};

use super::support::{
    FakeClock, SequenceAllocator, bind, client, coordinator, limits, panel_source, window,
};

#[test]
fn expiry_clock_regression_cancellation_and_destroy_are_typed() {
    let clock = FakeClock::new(0);
    let mut coordinator = coordinator();
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    let mut allocator = SequenceAllocator::new([[1; 16], [2; 16], [3; 16]]);

    let cancelled = create_session(&mut coordinator, &clock, &mut allocator, 10);
    assert_eq!(
        coordinator
            .cancel_session(&clock, cancelled)
            .unwrap()
            .status(),
        SessionCancellationStatus::Cancelled
    );
    assert_eq!(
        coordinator
            .cancel_session(&clock, cancelled)
            .unwrap()
            .status(),
        SessionCancellationStatus::AlreadyCancelled
    );
    assert_eq!(
        coordinator
            .attempt_target_resolution(
                &clock,
                cancelled,
                TargetSelector::ScreenPoint(longhorn_core::ScreenPoint::new(1, 1)),
                &[],
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::SessionCancelled
    );

    let expiring = create_session(&mut coordinator, &clock, &mut allocator, 10);
    clock.set(10);
    assert_eq!(
        coordinator
            .cancel_session(&clock, expiring)
            .unwrap_err()
            .code(),
        TransferErrorCode::SessionExpired
    );

    let destroyed = create_session(&mut coordinator, &clock, &mut allocator, 10);
    let receipt = coordinator.destroy_window(&window("window:source"));
    assert!(receipt.removed_client_binding());
    assert_eq!(receipt.invalidated_source_sessions(), 1);
    assert_eq!(
        coordinator
            .cancel_session(&clock, destroyed)
            .unwrap_err()
            .code(),
        TransferErrorCode::SourceWindowDestroyed
    );

    clock.set(9);
    assert_eq!(
        coordinator
            .bind_client_epoch(
                &clock,
                window("window:other"),
                client("client:other"),
                ClientEpoch::new(1),
            )
            .unwrap_err()
            .code(),
        TransferErrorCode::ClockRegressed
    );
}

#[test]
fn advancing_source_client_epoch_invalidates_active_sessions() {
    let clock = FakeClock::new(0);
    let mut coordinator = coordinator();
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    let mut allocator = SequenceAllocator::new([[4; 16]]);
    let session = create_session(&mut coordinator, &clock, &mut allocator, 20);
    coordinator
        .bind_client_epoch(
            &clock,
            window("window:source"),
            client("client:source"),
            ClientEpoch::new(2),
        )
        .unwrap();
    assert_eq!(
        coordinator
            .cancel_session(&clock, session)
            .unwrap_err()
            .code(),
        TransferErrorCode::SourceClientChanged
    );
}

#[test]
fn expired_capacity_is_reclaimed_and_shutdown_discards_only_process_state() {
    let clock = FakeClock::new(0);
    let mut coordinator = longhorn_transfer::TransferCoordinator::new(limits(1, 1, 1, 1));
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    let mut allocator = SequenceAllocator::new([[5; 16], [6; 16]]);
    create_session(&mut coordinator, &clock, &mut allocator, 5);
    clock.set(5);
    create_session(&mut coordinator, &clock, &mut allocator, 5);
    assert_eq!(coordinator.session_count(), 1);

    let discarded = coordinator.discard_all();
    assert_eq!(discarded.sessions(), 1);
    assert_eq!(discarded.client_windows(), 1);
    assert_eq!(discarded.leases(), 0);
    assert_eq!(coordinator.session_count(), 0);
    assert_eq!(coordinator.client_window_count(), 0);
}

fn create_session(
    coordinator: &mut longhorn_transfer::TransferCoordinator,
    clock: &FakeClock,
    allocator: &mut SequenceAllocator,
    lifetime: u64,
) -> longhorn_transfer::DragSessionId {
    coordinator
        .create_session(
            clock,
            allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(lifetime),
            ),
        )
        .unwrap()
        .payload()
        .session_id()
}
