use std::error::Error;

use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeCancellationOutcome,
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandDelivery,
    BridgeCommandEnvelope, BridgeCommandOutcome, BridgeCommandReply, BridgeConnectionReason,
    BridgeConnectionState, BridgeConnectionStatus, BridgeDeduplicationCapacity,
    BridgeDeduplicationSupport, BridgeDiagnostic, BridgeEventEnvelope, BridgeFailure,
    BridgeFailureMessage, BridgeFailurePhase, BridgeHelloRequest, BridgeHostDescriptor,
    BridgeHostForm, BridgeJobTerminalEvent, BridgeJobTerminalOutcome, BridgeJobTracker,
    BridgeNegotiationReceipt, BridgeProgressEvent, BridgeQueryEnvelope, BridgeQueryOutcome,
    BridgeQueryReply, BridgeRequestContext, BridgeRetryClass, BridgeSnapshotEnvelope,
    BridgeStreamCursor, BridgeStreamSequence, BridgeStreamTracker, DomainAuthorityDescriptor,
    DomainAvailability, DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority,
    WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeDiagnosticId, BridgeErrorCode, BridgeId,
    BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, DomainId, HostInstanceId,
    TransportFeatureId,
};
use serde::Serialize;
use serde_json::{Value, json, to_value};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryPayload {
    include_archived: bool,
}

#[derive(Clone, Serialize)]
struct CommandPayload {
    delta: i64,
}

#[derive(Clone, Serialize)]
struct SuccessPayload {
    value: i64,
}

#[derive(Clone, Serialize)]
struct FailureDetail {
    source: String,
}

#[derive(Serialize)]
struct SemanticTrace {
    command_retry: Vec<Value>,
    query_retry: Vec<Value>,
    listener_first: Vec<Value>,
    ordered_stream: Vec<Value>,
    job: Vec<Value>,
}

pub fn render() -> Result<String, Box<dyn Error>> {
    let hello = BridgeHelloRequest::new(
        BridgeId::new("bridge:fixture")?,
        vec![domain_id("example.workspace")],
    )?;
    let negotiation = negotiation()?;
    let query_request = BridgeQueryEnvelope::new(
        context("request:query"),
        QueryPayload {
            include_archived: false,
        },
    );
    let query_replies = vec![
        BridgeQueryReply::<SuccessPayload, FailureDetail>::new(
            request_id("request:query"),
            BridgeQueryOutcome::Success(SuccessPayload { value: 4 }),
        ),
        BridgeQueryReply::<SuccessPayload, FailureDetail>::new(
            request_id("request:query-failed"),
            BridgeQueryOutcome::Rejected(failure(
                BridgeRetryClass::AfterReconnect,
                BridgeFailurePhase::Transport,
            )?),
        ),
    ];
    let command_request = BridgeCommandEnvelope::new(
        context("request:command"),
        AuthorityEpoch::new(3)?,
        Some(AuthorityRevision::new(8)),
        Some(BridgeIdempotencyKey::new("idempotency:command")?),
        CommandPayload { delta: 2 },
    );
    let uncertain_command = BridgeCommandEnvelope::new(
        context("request:uncertain"),
        AuthorityEpoch::new(3)?,
        Some(AuthorityRevision::new(9)),
        None,
        CommandPayload { delta: 3 },
    );
    let command_replies = vec![
        BridgeCommandReply::<SuccessPayload, FailureDetail>::new(
            request_id("request:command"),
            Some(AuthorityRevision::new(9)),
            BridgeCommandOutcome::Applied(SuccessPayload { value: 6 }),
        ),
        BridgeCommandReply::<SuccessPayload, FailureDetail>::new(
            request_id("request:uncertain"),
            None,
            BridgeCommandOutcome::Indeterminate(failure(
                BridgeRetryClass::Never,
                BridgeFailurePhase::Response,
            )?),
        ),
    ];
    let snapshot =
        BridgeSnapshotEnvelope::new(cursor("session:fixture", 3, 8), SuccessPayload { value: 6 });
    let events = vec![
        BridgeEventEnvelope::new(cursor("session:fixture", 3, 9), CommandPayload { delta: 1 }),
        BridgeEventEnvelope::new(
            cursor("session:fixture", 3, 11),
            CommandPayload { delta: 2 },
        ),
    ];
    let progress = BridgeProgressEvent::new(
        request_id("request:scan"),
        job_id("job:scan"),
        json!({"completed": 2, "total": 4}),
    );
    let cancellation_request = BridgeCancellationRequest::new(
        context("request:cancel"),
        request_id("request:scan"),
        job_id("job:scan"),
    );
    let cancellation_receipt = BridgeCancellationReceipt::<FailureDetail>::new(
        request_id("request:cancel"),
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeCancellationOutcome::Accepted,
    );
    let terminal = BridgeJobTerminalEvent::<SuccessPayload, FailureDetail>::new(
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeJobTerminalOutcome::Succeeded(SuccessPayload { value: 12 }),
    );

    let fixture = json!({
        "protocolVersion": 1,
        "hello": to_value(hello)?,
        "negotiation": to_value(negotiation)?,
        "queryRequests": [to_value(&query_request)?],
        "queryReplies": to_value(query_replies)?,
        "commandRequests": [to_value(&command_request)?, to_value(&uncertain_command)?],
        "commandReplies": to_value(command_replies)?,
        "snapshot": to_value(snapshot)?,
        "events": to_value(events)?,
        "progress": to_value(progress)?,
        "cancellationRequest": to_value(cancellation_request)?,
        "cancellationReceipt": to_value(cancellation_receipt)?,
        "terminal": to_value(&terminal)?,
        "semanticTrace": to_value(semantic_trace(&query_request, &command_request, &uncertain_command, &terminal)?)?,
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownConnectionState": "futureState",
            "unknownRetryClass": "futureRetry",
            "unknownAuthorityShape": {
                "domainId": "example.workspace",
                "scopeId": "scope:workspace",
                "availability": "available",
                "readAuthority": "futureAuthority",
                "writeAuthority": "none",
                "executionAuthority": "none",
                "authorityEpoch": 1,
                "authoritativeRevision": null
            },
            "unknownQueryOutcome": {"future": {"value": 1}},
            "unknownCommandOutcome": {"future": {"value": 1}},
            "unknownTerminalOutcome": {"future": {"value": 1}}
        }
    });
    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}

fn negotiation() -> Result<BridgeNegotiationReceipt, Box<dyn Error>> {
    Ok(BridgeNegotiationReceipt::new(
        BridgeHostDescriptor {
            host_instance_id: HostInstanceId::new("host:fixture")?,
            form: BridgeHostForm::Direct,
        },
        session_id("session:fixture"),
        BridgeConnectionStatus::new(
            BridgeConnectionState::Ready,
            Some(BridgeConnectionReason::NegotiationAccepted),
        )?,
        AuthenticationPosture::NotRequired,
        vec![
            TransportFeatureId::new("request_reply")?,
            TransportFeatureId::new("ordered_streams")?,
            TransportFeatureId::new("job_execution")?,
        ],
        vec![DomainCapabilityDescriptor::new(
            domain_id("example.workspace"),
            vec![
                BridgeCapabilityId::new("query")?,
                BridgeCapabilityId::new("mutate")?,
                BridgeCapabilityId::new("subscribe")?,
                BridgeCapabilityId::new("start_job")?,
            ],
        )?],
        vec![DomainAuthorityDescriptor::new(
            domain_id("example.workspace"),
            AuthorityScopeId::new("scope:workspace")?,
            DomainAvailability::Available,
            ReadAuthority::Authoritative,
            WriteAuthority::Authoritative,
            ExecutionAuthority::Executor,
            AuthorityEpoch::new(3)?,
            Some(AuthorityRevision::new(8)),
        )?],
        // A negotiation that carries no diagnostic never exercises
        // `parseDiagnostic`, so the fixture published an empty array and the
        // element validator was proved by nothing.
        vec![BridgeDiagnostic::new(
            BridgeDiagnosticId::new("diagnostic:fixture")?,
            "fixture diagnostic",
        )?],
    )?)
}

fn semantic_trace(
    query: &BridgeQueryEnvelope<QueryPayload>,
    command: &BridgeCommandEnvelope<CommandPayload>,
    uncertain: &BridgeCommandEnvelope<CommandPayload>,
    terminal: &BridgeJobTerminalEvent<SuccessPayload, FailureDetail>,
) -> Result<SemanticTrace, Box<dyn Error>> {
    let finite = BridgeDeduplicationSupport::Finite(BridgeDeduplicationCapacity::new(8)?);
    let command_retry = vec![
        to_value(command.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ))?,
        to_value(uncertain.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            finite,
        ))?,
        to_value(command.classify_transport_failure(
            BridgeCommandDelivery::NotDispatched,
            BridgeRetryClass::AfterReconnect,
            finite,
        ))?,
    ];
    let query_retry = vec![
        to_value(query.classify_retry(BridgeRetryClass::AfterBackoff, true))?,
        to_value(query.classify_retry(BridgeRetryClass::AfterBackoff, false))?,
        to_value(query.classify_retry(BridgeRetryClass::Never, true))?,
    ];

    let mut listener_first = BridgeStreamTracker::new(
        session_id("session:fixture"),
        domain_id("example.workspace"),
    );
    let listener_first = vec![
        to_value(listener_first.classify_event(&cursor("session:fixture", 3, 9)))?,
        to_value(listener_first.accept_snapshot(cursor("session:fixture", 3, 8)))?,
        to_value(listener_first.accept_snapshot(cursor("session:fixture", 3, 9)))?,
    ];

    let mut ordered = BridgeStreamTracker::new(
        session_id("session:fixture"),
        domain_id("example.workspace"),
    );
    let ordered_stream = vec![
        to_value(ordered.accept_snapshot(cursor("session:fixture", 3, 8)))?,
        to_value(ordered.classify_event(&cursor("session:fixture", 3, 8)))?,
        to_value(ordered.classify_event(&cursor("session:fixture", 3, 7)))?,
        to_value(ordered.classify_event(&cursor("session:fixture", 3, 9)))?,
        to_value(ordered.classify_event(&cursor("session:fixture", 3, 11)))?,
        to_value(ordered.classify_event(&cursor("session:fixture", 4, 0)))?,
    ];

    let mut job = BridgeJobTracker::new(request_id("request:scan"), job_id("job:scan"));
    let progress = BridgeProgressEvent::new(
        request_id("request:scan"),
        job_id("job:scan"),
        json!({"completed": 1}),
    );
    let foreign = BridgeProgressEvent::new(
        request_id("request:other"),
        job_id("job:scan"),
        json!({"completed": 2}),
    );
    let job = vec![
        to_value(job.classify_progress(&progress))?,
        to_value(job.classify_progress(&foreign))?,
        to_value(job.classify_terminal(terminal))?,
        to_value(job.classify_progress(&progress))?,
        to_value(job.classify_terminal(terminal))?,
    ];

    Ok(SemanticTrace {
        command_retry,
        query_retry,
        listener_first,
        ordered_stream,
        job,
    })
}

fn context(request: &str) -> BridgeRequestContext {
    BridgeRequestContext::new(
        request_id(request),
        session_id("session:fixture"),
        domain_id("example.workspace"),
    )
}

fn cursor(session: &str, epoch: u64, sequence: u64) -> BridgeStreamCursor {
    BridgeStreamCursor::new(
        session_id(session),
        domain_id("example.workspace"),
        AuthorityEpoch::new(epoch).expect("fixture epoch is valid"),
        BridgeStreamSequence::new(sequence),
    )
}

fn failure(
    retry_class: BridgeRetryClass,
    phase: BridgeFailurePhase,
) -> Result<BridgeFailure<FailureDetail>, Box<dyn Error>> {
    Ok(BridgeFailure::new(
        BridgeErrorCode::new("workspace:unavailable")?,
        BridgeFailureMessage::new("workspace authority is unavailable")?,
        retry_class,
        phase,
        Some(FailureDetail {
            source: "fixture".into(),
        }),
    ))
}

fn request_id(value: &str) -> BridgeRequestId {
    BridgeRequestId::new(value).expect("fixture request id is valid")
}

fn session_id(value: &str) -> BridgeSessionId {
    BridgeSessionId::new(value).expect("fixture session id is valid")
}

fn job_id(value: &str) -> BridgeJobId {
    BridgeJobId::new(value).expect("fixture job id is valid")
}

fn domain_id(value: &str) -> DomainId {
    DomainId::new(value).expect("fixture domain id is valid")
}
