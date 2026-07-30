use longhorn_bridge::{
    AuthorityEpoch, AuthorityRevision, BridgeCommandDelivery, BridgeCommandEnvelope,
    BridgeCommandOutcome, BridgeCommandReply, BridgeCommandRetryDecision,
    BridgeDeduplicationCapacity, BridgeDeduplicationError, BridgeDeduplicationLedger,
    BridgeDeduplicationSupport, BridgeFailureMessage, BridgeFailureMessageError,
    BridgeFailurePhase, BridgeQueryEnvelope, BridgeQueryOutcome, BridgeQueryReply,
    BridgeQueryRetryDecision, BridgeRetryClass, MAXIMUM_FAILURE_MESSAGE_BYTES,
};

use crate::support::{
    CommandPayload, FailureDetail, QueryPayload, SuccessPayload, context, failure, idempotency_key,
    request_id,
};

#[test]
fn generic_requests_replies_and_failures_round_trip_without_payload_authority() {
    let query = BridgeQueryEnvelope::new(
        context("request:query"),
        QueryPayload {
            include_archived: false,
        },
    );
    let query_reply = BridgeQueryReply::<SuccessPayload, FailureDetail>::new(
        request_id("request:query"),
        BridgeQueryOutcome::Success(SuccessPayload { value: 4 }),
    );
    let command = BridgeCommandEnvelope::new(
        context("request:command"),
        AuthorityEpoch::new(3).unwrap(),
        Some(AuthorityRevision::new(8)),
        Some(idempotency_key("idempotency:command")),
        CommandPayload { delta: 2 },
    );
    let command_reply = BridgeCommandReply::<SuccessPayload, FailureDetail>::new(
        request_id("request:command"),
        Some(AuthorityRevision::new(9)),
        BridgeCommandOutcome::Applied(SuccessPayload { value: 6 }),
    );

    let query_json = serde_json::to_string(&query).unwrap();
    let query_reply_json = serde_json::to_string(&query_reply).unwrap();
    let command_json = serde_json::to_string(&command).unwrap();
    let command_reply_json = serde_json::to_string(&command_reply).unwrap();

    assert_eq!(
        serde_json::from_str::<BridgeQueryEnvelope<QueryPayload>>(&query_json).unwrap(),
        query
    );
    assert_eq!(
        serde_json::from_str::<BridgeQueryReply<SuccessPayload, FailureDetail>>(&query_reply_json)
            .unwrap(),
        query_reply
    );
    assert_eq!(
        serde_json::from_str::<BridgeCommandEnvelope<CommandPayload>>(&command_json).unwrap(),
        command
    );
    assert_eq!(
        serde_json::from_str::<BridgeCommandReply<SuccessPayload, FailureDetail>>(
            &command_reply_json
        )
        .unwrap(),
        command_reply
    );
    assert!(command_json.contains("\"requestId\":\"request:command\""));
    assert!(command_json.contains("\"idempotencyKey\":\"idempotency:command\""));
}

#[test]
fn coded_failure_keeps_phase_retry_and_typed_detail() {
    let rejection = BridgeQueryReply::<SuccessPayload, FailureDetail>::new(
        request_id("request:failure"),
        BridgeQueryOutcome::Rejected(failure(
            BridgeRetryClass::AfterReconnect,
            BridgeFailurePhase::Transport,
        )),
    );
    let encoded = serde_json::to_string(&rejection).unwrap();
    let decoded: BridgeQueryReply<SuccessPayload, FailureDetail> =
        serde_json::from_str(&encoded).unwrap();

    let BridgeQueryOutcome::Rejected(failure) = decoded.outcome() else {
        panic!("expected rejection");
    };
    assert_eq!(failure.code().as_str(), "workspace:unavailable");
    assert_eq!(failure.phase(), BridgeFailurePhase::Transport);
    assert_eq!(failure.retry_class(), BridgeRetryClass::AfterReconnect);
    assert_eq!(failure.details().unwrap().source, "fixture");
}

#[test]
fn failure_message_is_nonempty_bounded_and_checked_on_deserialization() {
    assert_eq!(
        BridgeFailureMessage::new("").unwrap_err(),
        BridgeFailureMessageError::Empty
    );
    assert_eq!(
        BridgeFailureMessage::new("x".repeat(MAXIMUM_FAILURE_MESSAGE_BYTES + 1)).unwrap_err(),
        BridgeFailureMessageError::TooLong {
            maximum: MAXIMUM_FAILURE_MESSAGE_BYTES,
            actual: MAXIMUM_FAILURE_MESSAGE_BYTES + 1,
        }
    );
    assert!(serde_json::from_str::<BridgeFailureMessage>("\"\"").is_err());
}

#[test]
fn command_retry_requires_key_deduplication_and_non_never_class() {
    let without_key = BridgeCommandEnvelope::new(
        context("request:no-key"),
        AuthorityEpoch::new(1).unwrap(),
        None,
        None,
        CommandPayload { delta: 1 },
    );
    let with_key = BridgeCommandEnvelope::new(
        context("request:with-key"),
        AuthorityEpoch::new(1).unwrap(),
        None,
        Some(idempotency_key("idempotency:with-key")),
        CommandPayload { delta: 1 },
    );
    let finite = BridgeDeduplicationSupport::Finite(BridgeDeduplicationCapacity::new(8).unwrap());

    assert_eq!(
        without_key.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!(
        with_key.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            BridgeDeduplicationSupport::Unsupported,
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!(
        with_key.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::Never,
            finite,
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!(
        with_key.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::RetrySameRequest
    );
    assert_eq!(
        with_key.classify_transport_failure(
            BridgeCommandDelivery::NotDispatched,
            BridgeRetryClass::AfterReconnect,
            finite,
        ),
        BridgeCommandRetryDecision::DoNotRetry
    );
}

#[test]
fn query_retry_remains_explicit_adapter_policy() {
    let query = BridgeQueryEnvelope::new(
        context("request:query-retry"),
        QueryPayload {
            include_archived: true,
        },
    );

    assert_eq!(
        query.classify_retry(BridgeRetryClass::AfterBackoff, true),
        BridgeQueryRetryDecision::Retry
    );
    assert_eq!(
        query.classify_retry(BridgeRetryClass::AfterBackoff, false),
        BridgeQueryRetryDecision::DoNotRetry
    );
    assert_eq!(
        query.classify_retry(BridgeRetryClass::Never, true),
        BridgeQueryRetryDecision::DoNotRetry
    );
}

#[test]
fn finite_deduplication_evidence_never_evicts_into_false_freshness() {
    let mut ledger = BridgeDeduplicationLedger::new(BridgeDeduplicationCapacity::new(1).unwrap());
    let key = idempotency_key("idempotency:one");
    ledger
        .record(key.clone(), request_id("request:first"), 42_u64)
        .unwrap();

    let retained = ledger.lookup(&key).unwrap();
    assert_eq!(retained.original_request_id().as_str(), "request:first");
    assert_eq!(*retained.outcome(), 42);
    assert_eq!(
        ledger
            .record(key, request_id("request:duplicate"), 43)
            .unwrap_err(),
        BridgeDeduplicationError::DuplicateKey
    );
    assert_eq!(
        ledger
            .record(
                idempotency_key("idempotency:two"),
                request_id("request:second"),
                44,
            )
            .unwrap_err(),
        BridgeDeduplicationError::Full
    );
    assert_eq!(ledger.len(), 1);
}

#[test]
fn query_only_shape_needs_no_stream_or_job_metadata() {
    let query = BridgeQueryEnvelope::new(
        context("request:bovine-query"),
        QueryPayload {
            include_archived: false,
        },
    );
    let encoded = serde_json::to_value(query).unwrap();

    assert!(encoded.get("context").is_some());
    assert!(encoded.get("payload").is_some());
    assert!(encoded.get("cursor").is_none());
    assert!(encoded.get("jobId").is_none());
    assert!(encoded.get("idempotencyKey").is_none());
}
