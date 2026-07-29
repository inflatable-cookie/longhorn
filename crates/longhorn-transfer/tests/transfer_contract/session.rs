use longhorn_transfer::{TransferDuration, TransferErrorCode, TransferSessionRequest};

use super::support::{FakeClock, SequenceAllocator, bind, coordinator, limits, panel_source};

#[test]
fn payload_contains_only_version_and_exact_128_bit_session_id() {
    let clock = FakeClock::new(20);
    let mut coordinator = coordinator();
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    let mut allocator = SequenceAllocator::new([[0xab; 16]]);
    let receipt = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(
                panel_source("window:source", "client:source", 1),
                TransferDuration::new(10),
            ),
        )
        .unwrap();

    assert_eq!(
        serde_json::to_value(receipt.payload()).unwrap(),
        serde_json::json!({
            "protocol_version": 1,
            "session_id": "abababababababababababababababab"
        })
    );
    assert_eq!(
        serde_json::from_value::<longhorn_transfer::TransferPayload>(
            serde_json::to_value(receipt.payload()).unwrap()
        )
        .unwrap(),
        receipt.payload()
    );
    assert!(
        serde_json::from_value::<longhorn_transfer::TransferPayload>(serde_json::json!({
            "protocol_version": 1,
            "session_id": "ABABABABABABABABABABABABABABABAB"
        }))
        .is_err()
    );
    assert_eq!(receipt.expires_at().get(), 30);
}

#[test]
fn invalid_lifetime_allocator_failure_collision_and_capacity_allocate_nothing() {
    let clock = FakeClock::new(0);
    let mut coordinator = longhorn_transfer::TransferCoordinator::new(limits(2, 2, 2, 10));
    bind(
        &mut coordinator,
        &clock,
        "window:source",
        "client:source",
        1,
    );
    let source = panel_source("window:source", "client:source", 1);

    let mut unused = SequenceAllocator::new([[9; 16]]);
    let invalid = coordinator
        .create_session(
            &clock,
            &mut unused,
            TransferSessionRequest::new(source.clone(), TransferDuration::new(0)),
        )
        .unwrap_err();
    assert_eq!(invalid.code(), TransferErrorCode::InvalidLifetime);
    assert_eq!(unused.calls(), 0);
    assert_eq!(coordinator.session_count(), 0);

    let unknown_source = coordinator
        .create_session(
            &clock,
            &mut unused,
            TransferSessionRequest::new(
                panel_source("window:missing", "client:source", 1),
                TransferDuration::new(10),
            ),
        )
        .unwrap_err();
    assert_eq!(unknown_source.code(), TransferErrorCode::UnknownClientEpoch);
    assert_eq!(unused.calls(), 0);
    assert_eq!(coordinator.session_count(), 0);

    let mut failing = SequenceAllocator::failing();
    let failed = coordinator
        .create_session(
            &clock,
            &mut failing,
            TransferSessionRequest::new(source.clone(), TransferDuration::new(10)),
        )
        .unwrap_err();
    assert_eq!(failed.code(), TransferErrorCode::SessionIdAllocation);
    assert_eq!(coordinator.session_count(), 0);

    let mut allocator = SequenceAllocator::new([[1; 16], [1; 16], [2; 16], [3; 16]]);
    coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(source.clone(), TransferDuration::new(10)),
        )
        .unwrap();
    let collision = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(source.clone(), TransferDuration::new(10)),
        )
        .unwrap_err();
    assert_eq!(collision.code(), TransferErrorCode::SessionIdCollision);
    coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(source.clone(), TransferDuration::new(10)),
        )
        .unwrap();
    let calls_before_capacity = allocator.calls();
    let capacity = coordinator
        .create_session(
            &clock,
            &mut allocator,
            TransferSessionRequest::new(source, TransferDuration::new(10)),
        )
        .unwrap_err();
    assert_eq!(capacity.code(), TransferErrorCode::SessionCapacity);
    assert_eq!(allocator.calls(), calls_before_capacity);
    assert_eq!(coordinator.session_count(), 2);
}

#[test]
fn finite_limits_reject_zero_excess_and_unknown_serialized_fields() {
    use longhorn_transfer::{TransferLimits, TransferLimitsError};

    assert_eq!(
        TransferLimits::new(
            0,
            1,
            1,
            1,
            1,
            TransferDuration::new(1),
            TransferDuration::new(1),
        ),
        Err(TransferLimitsError::Zero {
            field: "maximum_sessions"
        })
    );
    assert!(matches!(
        TransferLimits::new(
            4_097,
            1,
            1,
            1,
            1,
            TransferDuration::new(1),
            TransferDuration::new(1),
        ),
        Err(TransferLimitsError::ExceedsHardMaximum {
            field: "maximum_sessions",
            ..
        })
    ));
    let valid = limits(2, 2, 2, 10);
    let mut value = serde_json::to_value(valid).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("future".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<TransferLimits>(value).is_err());
}
