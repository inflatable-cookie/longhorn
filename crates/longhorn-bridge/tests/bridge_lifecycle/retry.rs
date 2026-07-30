use longhorn_bridge::{
    AuthorityEpoch, BridgeCommandDelivery, BridgeCommandEnvelope, BridgeCommandRetryDecision,
    BridgeDeduplicationCapacity, BridgeDeduplicationSupport, BridgeQueryEnvelope,
    BridgeQueryRetryController, BridgeQueryRetryDecision, BridgeRequestContext, BridgeRetryClass,
    BridgeRetryLimit,
};
use longhorn_core::{BridgeIdempotencyKey, BridgeRequestId, BridgeSessionId, DomainId};

use super::support::{Clock, LinearBackoff};

#[test]
fn query_retry_is_explicit_bounded_and_resettable() {
    let clock = Clock::new(1_000);
    let mut controller = BridgeQueryRetryController::new(BridgeRetryLimit::new(2).unwrap());

    let first = controller
        .schedule(
            BridgeQueryRetryDecision::Retry,
            BridgeRetryClass::AfterBackoff,
            &clock,
            &LinearBackoff,
        )
        .unwrap()
        .unwrap();
    let second = controller
        .schedule(
            BridgeQueryRetryDecision::Retry,
            BridgeRetryClass::AfterBackoff,
            &clock,
            &LinearBackoff,
        )
        .unwrap()
        .unwrap();
    assert_eq!(first.not_before().get(), 1_025);
    assert_eq!(second.not_before().get(), 1_050);
    assert!(
        controller
            .schedule(
                BridgeQueryRetryDecision::Retry,
                BridgeRetryClass::AfterBackoff,
                &clock,
                &LinearBackoff,
            )
            .unwrap()
            .is_none()
    );

    controller.reset();
    assert_eq!(
        controller
            .schedule(
                BridgeQueryRetryDecision::Retry,
                BridgeRetryClass::AfterBackoff,
                &clock,
                &LinearBackoff,
            )
            .unwrap()
            .unwrap()
            .attempt()
            .get(),
        1
    );
}

#[test]
fn command_replay_needs_durable_key_and_finite_deduplication() {
    let context = BridgeRequestContext::new(
        BridgeRequestId::new("request:one").unwrap(),
        BridgeSessionId::new("session:one").unwrap(),
        DomainId::new("example.workspace").unwrap(),
    );
    let epoch = AuthorityEpoch::new(1).unwrap();
    let no_key = BridgeCommandEnvelope::new(context.clone(), epoch, None, None, 1_u8);
    let durable = BridgeCommandEnvelope::new(
        context,
        epoch,
        None,
        Some(BridgeIdempotencyKey::new("command:durable").unwrap()),
        1_u8,
    );
    let finite = BridgeDeduplicationSupport::Finite(BridgeDeduplicationCapacity::new(32).unwrap());

    assert_eq!(
        no_key.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!(
        durable.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            BridgeDeduplicationSupport::Unsupported,
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!(
        durable.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::RetrySameRequest
    );
    assert_eq!(
        durable.classify_transport_failure(
            BridgeCommandDelivery::NotDispatched,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::DoNotRetry
    );
}

#[test]
fn denied_or_never_query_retry_is_not_scheduled() {
    let clock = Clock::new(0);
    let mut controller = BridgeQueryRetryController::new(BridgeRetryLimit::new(2).unwrap());
    for (decision, class) in [
        (
            BridgeQueryRetryDecision::DoNotRetry,
            BridgeRetryClass::AfterBackoff,
        ),
        (BridgeQueryRetryDecision::Retry, BridgeRetryClass::Never),
    ] {
        assert!(
            controller
                .schedule(decision, class, &clock, &LinearBackoff)
                .unwrap()
                .is_none()
        );
    }

    let _query = BridgeQueryEnvelope::new(
        BridgeRequestContext::new(
            BridgeRequestId::new("request:query").unwrap(),
            BridgeSessionId::new("session:one").unwrap(),
            DomainId::new("example.workspace").unwrap(),
        ),
        (),
    );
}
