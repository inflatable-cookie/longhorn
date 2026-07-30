use longhorn_bridge::{
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus, BridgeHelloRequest,
    BridgeNegotiationErrorCode, BridgeProtocolVersion, MAXIMUM_REQUESTED_DOMAINS,
};
use longhorn_core::BridgeId;
use serde_json::json;

use super::support::domain;

#[test]
fn negotiation_accepts_exact_v1_and_rejects_every_other_version() {
    assert_eq!(BridgeProtocolVersion::new(1).unwrap().get(), 1);
    for actual in [0, 2, u16::MAX] {
        assert_eq!(
            BridgeProtocolVersion::new(actual).unwrap_err().code(),
            BridgeNegotiationErrorCode::IncompatibleProtocol
        );
    }

    let valid = json!({
        "protocolVersion": 1,
        "bridgeId": "bridge:bovine",
        "requestedDomains": ["bovine.workspace"]
    });
    let request: BridgeHelloRequest = serde_json::from_value(valid).unwrap();
    assert_eq!(request.protocol_version(), BridgeProtocolVersion::CURRENT);

    let incompatible = json!({
        "protocolVersion": 2,
        "bridgeId": "bridge:bovine",
        "requestedDomains": ["bovine.workspace"]
    });
    assert!(serde_json::from_value::<BridgeHelloRequest>(incompatible).is_err());
}

#[test]
fn hello_domain_set_is_bounded_unique_and_explicit() {
    let duplicate = BridgeHelloRequest::new(
        BridgeId::new("bridge:test").unwrap(),
        vec![domain("example.workspace"), domain("example.workspace")],
    )
    .unwrap_err();
    assert_eq!(
        duplicate.code(),
        BridgeNegotiationErrorCode::DuplicateRequestedDomain
    );

    let excessive = (0..=MAXIMUM_REQUESTED_DOMAINS)
        .map(|index| domain(&format!("example.domain-{index}")))
        .collect();
    assert_eq!(
        BridgeHelloRequest::new(BridgeId::new("bridge:test").unwrap(), excessive)
            .unwrap_err()
            .code(),
        BridgeNegotiationErrorCode::LimitExceeded
    );
}

#[test]
fn connection_state_reason_matrix_is_checked() {
    let valid = [
        (BridgeConnectionState::Idle, None, false),
        (
            BridgeConnectionState::Connecting,
            Some(BridgeConnectionReason::ConnectRequested),
            false,
        ),
        (
            BridgeConnectionState::Negotiating,
            Some(BridgeConnectionReason::TransportReady),
            false,
        ),
        (
            BridgeConnectionState::Ready,
            Some(BridgeConnectionReason::NegotiationAccepted),
            false,
        ),
        (
            BridgeConnectionState::Degraded,
            Some(BridgeConnectionReason::CapabilityChanged),
            true,
        ),
        (
            BridgeConnectionState::Reconnecting,
            Some(BridgeConnectionReason::RetryScheduled),
            true,
        ),
        (
            BridgeConnectionState::Offline,
            Some(BridgeConnectionReason::TransportLost),
            true,
        ),
        (
            BridgeConnectionState::Incompatible,
            Some(BridgeConnectionReason::VersionMismatch),
            false,
        ),
        (
            BridgeConnectionState::Unauthorized,
            Some(BridgeConnectionReason::AuthorizationRejected),
            false,
        ),
        (
            BridgeConnectionState::Failed,
            Some(BridgeConnectionReason::HostFailure),
            false,
        ),
        (
            BridgeConnectionState::Closed,
            Some(BridgeConnectionReason::Shutdown),
            false,
        ),
    ];

    for (state, reason, reconnect_permitted) in valid {
        let status = BridgeConnectionStatus::new(state, reason).unwrap();
        assert_eq!(status.reconnect_permitted(), reconnect_permitted);
    }

    let invalid = BridgeConnectionStatus::new(
        BridgeConnectionState::Ready,
        Some(BridgeConnectionReason::TransportReady),
    )
    .unwrap_err();
    assert_eq!(
        invalid.code(),
        BridgeNegotiationErrorCode::InvalidConnectionStatus
    );
}
