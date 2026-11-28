//! Registered-domain fixtures shared by the bridge host tests.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeCancellationOutcome,
    BridgeCancellationReceipt, BridgeCommandEnvelope, BridgeCommandOutcome, BridgeCommandReply,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus, BridgeHelloRequest,
    BridgeHostDescriptor, BridgeHostForm, BridgeJobTerminalEvent, BridgeJobTerminalOutcome,
    BridgeNegotiationReceipt, BridgeProgressEvent, BridgeQueryEnvelope, BridgeQueryOutcome,
    BridgeQueryReply, BridgeRequestContext, BridgeSnapshotEnvelope, BridgeStreamCursor,
    BridgeStreamSequence, DomainAuthorityDescriptor, DomainAvailability,
    DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority, WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeId, BridgeJobId, BridgeRequestId, BridgeSessionId,
    DomainId, HostInstanceId,
};
use longhorn_tauri_bridge::{BridgeAuthorityProvider, BridgeDomainRegistry, BridgeHostError};
use serde_json::{Value, json};

pub const DOMAIN: &str = "example.workspace";
pub const READ: &str = "workspace.read";
pub const WRITE: &str = "workspace.write";
pub const CANCEL: &str = "workspace.cancel";

#[derive(Clone)]
pub struct FixedAuthority {
    pub receipt: BridgeNegotiationReceipt,
}

impl BridgeAuthorityProvider for FixedAuthority {
    fn negotiate(
        &mut self,
        _caller: &str,
        _request: &BridgeHelloRequest,
        _registered_domains: &[DomainCapabilityDescriptor],
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Ok(self.receipt.clone())
    }

    fn refresh(
        &mut self,
        _caller: &str,
        _current: &BridgeNegotiationReceipt,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Ok(self.receipt.clone())
    }
}

pub fn registered_domains(dispatches: Arc<AtomicUsize>) -> BridgeDomainRegistry {
    let mut registry = BridgeDomainRegistry::new();
    registry.register_domain(capabilities()).unwrap();
    let query_count = Arc::clone(&dispatches);
    registry
        .register_query::<Value, Value, Value, _>(
            domain(),
            "workspace.read",
            capability(READ),
            move |request: BridgeQueryEnvelope<Value>| {
                query_count.fetch_add(1, Ordering::SeqCst);
                Ok(BridgeQueryReply::new(
                    request.context().request_id().clone(),
                    BridgeQueryOutcome::Success(request.into_payload()),
                ))
            },
        )
        .unwrap();

    let command_count = Arc::clone(&dispatches);
    registry
        .register_command::<Value, Value, Value, _>(
            domain(),
            "workspace.write",
            capability(WRITE),
            move |request: BridgeCommandEnvelope<Value>| {
                command_count.fetch_add(1, Ordering::SeqCst);
                Ok(BridgeCommandReply::new(
                    request.context().request_id().clone(),
                    Some(AuthorityRevision::new(2)),
                    BridgeCommandOutcome::Applied(request.into_payload()),
                ))
            },
        )
        .unwrap();

    let cancel_count = Arc::clone(&dispatches);
    registry
        .register_cancellation::<Value, _>(
            domain(),
            "workspace.cancel",
            capability(CANCEL),
            move |request: longhorn_bridge::BridgeCancellationRequest| {
                cancel_count.fetch_add(1, Ordering::SeqCst);
                Ok(BridgeCancellationReceipt::new(
                    request.context().request_id().clone(),
                    request.target_request_id().clone(),
                    request.job_id().clone(),
                    BridgeCancellationOutcome::Accepted,
                ))
            },
        )
        .unwrap();

    registry
        .register_snapshot::<Value, _>(
            domain(),
            capability(READ),
            |session_id: &BridgeSessionId, _domain_id: &DomainId| {
                Ok(BridgeSnapshotEnvelope::new(
                    BridgeStreamCursor::new(
                        session_id.clone(),
                        domain(),
                        AuthorityEpoch::new(1).unwrap(),
                        BridgeStreamSequence::new(1),
                    ),
                    json!({ "items": [] }),
                ))
            },
        )
        .unwrap();
    registry
}

pub fn authority_receipt(
    capabilities: DomainCapabilityDescriptor,
    write: WriteAuthority,
) -> BridgeNegotiationReceipt {
    authority_receipt_for(
        "session:current",
        capabilities,
        ReadAuthority::Authoritative,
        write,
    )
}

pub fn authority_receipt_for(
    session: &str,
    capabilities: DomainCapabilityDescriptor,
    read: ReadAuthority,
    write: WriteAuthority,
) -> BridgeNegotiationReceipt {
    authority_receipt_with(
        session,
        capabilities,
        read,
        write,
        ExecutionAuthority::Executor,
    )
}

pub fn authority_receipt_with(
    session: &str,
    capabilities: DomainCapabilityDescriptor,
    read: ReadAuthority,
    write: WriteAuthority,
    execution: ExecutionAuthority,
) -> BridgeNegotiationReceipt {
    BridgeNegotiationReceipt::new(
        BridgeHostDescriptor {
            host_instance_id: HostInstanceId::new("host:tauri-fixture").unwrap(),
            form: BridgeHostForm::TauriLocal,
        },
        BridgeSessionId::new(session).unwrap(),
        BridgeConnectionStatus::new(
            BridgeConnectionState::Ready,
            Some(BridgeConnectionReason::NegotiationAccepted),
        )
        .unwrap(),
        AuthenticationPosture::NotRequired,
        Vec::new(),
        vec![capabilities],
        vec![
            DomainAuthorityDescriptor::new(
                domain(),
                AuthorityScopeId::new("scope:workspace").unwrap(),
                DomainAvailability::Available,
                read,
                write,
                execution,
                AuthorityEpoch::new(1).unwrap(),
                // A revision exists only when something is authoritative.
                (read == ReadAuthority::Authoritative || write == WriteAuthority::Authoritative)
                    .then(|| AuthorityRevision::new(1)),
            )
            .unwrap(),
        ],
        Vec::new(),
    )
    .unwrap()
}

pub fn hello() -> BridgeHelloRequest {
    BridgeHelloRequest::new(BridgeId::new("bridge:test").unwrap(), vec![domain()]).unwrap()
}

pub fn query_request(request: &str, session: &str, domain_id: &str) -> BridgeQueryEnvelope<Value> {
    BridgeQueryEnvelope::new(
        context(request, session, domain_id),
        json!({ "query": true }),
    )
}

pub fn context(request: &str, session: &str, domain_id: &str) -> BridgeRequestContext {
    BridgeRequestContext::new(
        BridgeRequestId::new(request).unwrap(),
        BridgeSessionId::new(session).unwrap(),
        DomainId::new(domain_id).unwrap(),
    )
}

pub fn capabilities() -> DomainCapabilityDescriptor {
    DomainCapabilityDescriptor::new(
        domain(),
        vec![capability(READ), capability(WRITE), capability(CANCEL)],
    )
    .unwrap()
}

pub fn domain() -> DomainId {
    DomainId::new(DOMAIN).unwrap()
}

pub fn capability(value: &str) -> BridgeCapabilityId {
    BridgeCapabilityId::new(value).unwrap()
}

pub fn progress_event() -> BridgeProgressEvent<Value> {
    BridgeProgressEvent::new(
        BridgeRequestId::new("request:job").unwrap(),
        BridgeJobId::new("job:scan").unwrap(),
        json!({ "completed": 1 }),
    )
}

pub fn terminal_event() -> BridgeJobTerminalEvent<Value, Value> {
    BridgeJobTerminalEvent::new(
        BridgeRequestId::new("request:job").unwrap(),
        BridgeJobId::new("job:scan").unwrap(),
        BridgeJobTerminalOutcome::Succeeded(json!({ "found": 1 })),
    )
}
