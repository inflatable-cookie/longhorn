//! Registered bridge host rejection and Tauri mock-runtime proofs.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeCancellationOutcome,
    BridgeCancellationReceipt, BridgeCommandEnvelope, BridgeCommandOutcome, BridgeCommandReply,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus, BridgeEventEnvelope,
    BridgeHelloRequest, BridgeHostDescriptor, BridgeHostForm, BridgeNegotiationReceipt,
    BridgeQueryEnvelope, BridgeQueryOutcome, BridgeQueryReply, BridgeRequestContext,
    BridgeSnapshotEnvelope, BridgeStreamCursor, BridgeStreamSequence, DomainAuthorityDescriptor,
    DomainAvailability, DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority,
    WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeId, BridgeRequestId, BridgeSessionId, DomainId,
    HostInstanceId,
};
use longhorn_tauri_bridge::{
    BRIDGE_DOMAIN_EVENT, BridgeAuthorityProvider, BridgeDomainRegistry, BridgeHandlerAssembly,
    BridgeHostError, BridgeHostErrorCode, TauriBridgeState, bridge_plugin,
};
use serde_json::{Value, json};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const DOMAIN: &str = "fixture.workspace";
const READ: &str = "workspace.read";
const WRITE: &str = "workspace.write";
const CANCEL: &str = "workspace.cancel";

#[derive(Clone)]
struct FixedAuthority {
    receipt: BridgeNegotiationReceipt,
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

#[test]
fn rejects_before_dispatch_when_session_authority_or_metadata_is_invalid() {
    let dispatches = Arc::new(AtomicUsize::new(0));
    let assembly = query_only_assembly(Arc::clone(&dispatches), WriteAuthority::None);
    let receipt = assembly.hello("main", hello()).unwrap();

    let wrong_session = query_request("request:wrong-session", "session:wrong", DOMAIN);
    assert_eq!(
        assembly
            .query("main", "workspace.read", wrong_session)
            .unwrap_err()
            .code,
        BridgeHostErrorCode::InvalidSession
    );

    let wrong_domain = query_request(
        "request:wrong-domain",
        receipt.session_id().as_str(),
        "fixture.other",
    );
    assert_eq!(
        assembly
            .query("main", "workspace.read", wrong_domain)
            .unwrap_err()
            .code,
        BridgeHostErrorCode::UnknownRoute
    );

    let command = BridgeCommandEnvelope::new(
        context(
            "request:write-denied",
            receipt.session_id().as_str(),
            DOMAIN,
        ),
        AuthorityEpoch::new(1).unwrap(),
        None,
        None,
        json!({ "delta": 1 }),
    );
    assert_eq!(
        assembly
            .command("main", "workspace.write", command)
            .unwrap_err()
            .code,
        BridgeHostErrorCode::WriteDenied
    );
    assert_eq!(dispatches.load(Ordering::SeqCst), 0);
}

#[test]
fn registered_typed_handlers_and_cancellation_preserve_correlation() {
    let dispatches = Arc::new(AtomicUsize::new(0));
    let assembly = query_only_assembly(Arc::clone(&dispatches), WriteAuthority::Authoritative);
    let receipt = assembly.hello("main", hello()).unwrap();
    let session = receipt.session_id().as_str();

    let reply = assembly
        .query(
            "main",
            "workspace.read",
            query_request("request:query", session, DOMAIN),
        )
        .unwrap();
    assert_eq!(reply.request_id().as_str(), "request:query");

    let cancellation: longhorn_bridge::BridgeCancellationRequest = serde_json::from_value(json!({
        "context": {
            "requestId": "request:cancel",
            "sessionId": session,
            "domainId": DOMAIN
        },
        "targetRequestId": "request:job",
        "jobId": "job:scan"
    }))
    .unwrap();
    let receipt = assembly
        .cancel("main", "workspace.cancel", cancellation)
        .unwrap();
    assert_eq!(receipt.request_id().as_str(), "request:cancel");
    assert_eq!(receipt.target_request_id().as_str(), "request:job");
    assert_eq!(receipt.job_id().as_str(), "job:scan");
    assert_eq!(dispatches.load(Ordering::SeqCst), 2);
}

#[test]
fn query_only_rejects_events_while_subscription_assembly_emits_checked_payloads() {
    let query_only = query_only_assembly(Arc::new(AtomicUsize::new(0)), WriteAuthority::None);
    query_only.hello("main", hello()).unwrap();
    let event = BridgeEventEnvelope::new(cursor(2), json!({ "changed": true }));
    assert_eq!(
        query_only.publish_domain_event(&event).unwrap_err().code,
        BridgeHostErrorCode::EventUnavailable
    );

    let emissions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = Arc::clone(&emissions);
    let descriptor = capabilities();
    let subscription = BridgeHandlerAssembly::with_event_sink(
        FixedAuthority {
            receipt: authority_receipt(descriptor, WriteAuthority::None),
        },
        registered_domains(Arc::new(AtomicUsize::new(0))),
        Arc::new(move |_target: &str, name: &'static str, payload: Value| {
            captured.lock().unwrap().push((name, payload));
            Ok(())
        }),
    );
    subscription.hello("main", hello()).unwrap();
    subscription.publish_domain_event(&event).unwrap();
    assert_eq!(emissions.lock().unwrap()[0].0, BRIDGE_DOMAIN_EVENT);
}

#[test]
fn a_torn_down_callers_sessions_stop_validating() {
    // Sessions end with their window. After teardown the old session refuses,
    // a second teardown is not an error, and a fresh hello negotiates clean.
    let dispatches = Arc::new(AtomicUsize::new(0));
    let assembly = query_only_assembly(Arc::clone(&dispatches), WriteAuthority::None);
    let receipt = assembly.hello("main", hello()).unwrap();
    let session = receipt.session_id().as_str();

    assembly
        .query(
            "main",
            "workspace.read",
            query_request("request:before", session, DOMAIN),
        )
        .unwrap();

    assembly.teardown("main").unwrap();
    assert_eq!(
        assembly
            .query(
                "main",
                "workspace.read",
                query_request("request:after", session, DOMAIN),
            )
            .unwrap_err()
            .code,
        BridgeHostErrorCode::InvalidSession
    );
    assert_eq!(
        dispatches.load(Ordering::SeqCst),
        1,
        "the stale query did not dispatch"
    );

    assembly.teardown("main").unwrap();
    let renegotiated = assembly.hello("main", hello()).unwrap();
    assembly
        .query(
            "main",
            "workspace.read",
            query_request("request:fresh", renegotiated.session_id().as_str(), DOMAIN),
        )
        .unwrap();
}

#[test]
fn events_are_delivered_to_the_session_owning_window_only() {
    // Two windows, two sessions, one domain. An event published under one
    // session must reach that session's window and no other — broadcast was
    // delivery without a consumer (the client drops foreign-session cursors)
    // and a read-authority hole beside the per-caller model.
    struct PerCaller {
        descriptor: DomainCapabilityDescriptor,
    }

    impl BridgeAuthorityProvider for PerCaller {
        fn negotiate(
            &mut self,
            caller: &str,
            _request: &BridgeHelloRequest,
            _registered_domains: &[DomainCapabilityDescriptor],
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(authority_receipt_for(
                &format!("session:{caller}"),
                self.descriptor.clone(),
                ReadAuthority::Authoritative,
                WriteAuthority::None,
            ))
        }

        fn refresh(
            &mut self,
            caller: &str,
            _current: &BridgeNegotiationReceipt,
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(authority_receipt_for(
                &format!("session:{caller}"),
                self.descriptor.clone(),
                ReadAuthority::Authoritative,
                WriteAuthority::None,
            ))
        }
    }

    let emissions = Arc::new(std::sync::Mutex::new(Vec::<(String, &'static str)>::new()));
    let captured = Arc::clone(&emissions);
    let assembly = BridgeHandlerAssembly::with_event_sink(
        PerCaller {
            descriptor: capabilities(),
        },
        registered_domains(Arc::new(AtomicUsize::new(0))),
        Arc::new(move |target: &str, name: &'static str, _payload: Value| {
            captured.lock().unwrap().push((target.to_owned(), name));
            Ok(())
        }),
    );

    let main = assembly.hello("main", hello()).unwrap();
    let secondary = assembly.hello("secondary", hello()).unwrap();
    let cursor_for = |receipt: &BridgeNegotiationReceipt, sequence: u64| {
        BridgeStreamCursor::new(
            receipt.session_id().clone(),
            domain(),
            AuthorityEpoch::new(1).unwrap(),
            BridgeStreamSequence::new(sequence),
        )
    };

    assembly
        .publish_domain_event(&BridgeEventEnvelope::new(
            cursor_for(&main, 1),
            json!({ "changed": true }),
        ))
        .unwrap();
    assembly
        .publish_domain_event(&BridgeEventEnvelope::new(
            cursor_for(&secondary, 1),
            json!({ "changed": true }),
        ))
        .unwrap();

    let seen = emissions.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], ("main".to_owned(), BRIDGE_DOMAIN_EVENT));
    assert_eq!(seen[1], ("secondary".to_owned(), BRIDGE_DOMAIN_EVENT));
}

#[test]
fn a_session_without_read_authority_publishes_nothing() {
    // Delivery is targeted at the session's own window, so a full payload
    // may cross only where the session may read.
    struct NoRead {
        descriptor: DomainCapabilityDescriptor,
    }

    impl BridgeAuthorityProvider for NoRead {
        fn negotiate(
            &mut self,
            _caller: &str,
            _request: &BridgeHelloRequest,
            _registered_domains: &[DomainCapabilityDescriptor],
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(authority_receipt_for(
                "session:no-read",
                self.descriptor.clone(),
                ReadAuthority::None,
                WriteAuthority::None,
            ))
        }

        fn refresh(
            &mut self,
            _caller: &str,
            _current: &BridgeNegotiationReceipt,
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(authority_receipt_for(
                "session:no-read",
                self.descriptor.clone(),
                ReadAuthority::None,
                WriteAuthority::None,
            ))
        }
    }

    let emissions = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let captured = Arc::clone(&emissions);
    let assembly = BridgeHandlerAssembly::with_event_sink(
        NoRead {
            descriptor: capabilities(),
        },
        registered_domains(Arc::new(AtomicUsize::new(0))),
        Arc::new(move |target: &str, _name: &'static str, _payload: Value| {
            captured.lock().unwrap().push(target.to_owned());
            Ok(())
        }),
    );

    assembly.hello("main", hello()).unwrap();
    let event = BridgeEventEnvelope::new(
        BridgeStreamCursor::new(
            BridgeSessionId::new("session:no-read").unwrap(),
            domain(),
            AuthorityEpoch::new(1).unwrap(),
            BridgeStreamSequence::new(1),
        ),
        json!({ "changed": true }),
    );

    assert_eq!(
        assembly.publish_domain_event(&event).unwrap_err().code,
        BridgeHostErrorCode::ReadDenied
    );
    assert!(emissions.lock().unwrap().is_empty());
}

#[test]
fn direct_and_tauri_mock_hosts_use_the_same_assembly() {
    let direct = query_only_assembly(Arc::new(AtomicUsize::new(0)), WriteAuthority::None);
    let direct_receipt = direct.hello("main", hello()).unwrap();
    let direct_reply = direct
        .query(
            "main",
            "workspace.read",
            query_request("request:parity", "session:current", DOMAIN),
        )
        .unwrap();

    let mock = query_only_assembly(Arc::new(AtomicUsize::new(0)), WriteAuthority::None);
    let app = tauri::test::mock_builder()
        .plugin(bridge_plugin(mock))
        .invoke_handler(tauri::generate_handler![
            longhorn_tauri_bridge::longhorn_bridge_hello,
            longhorn_tauri_bridge::longhorn_bridge_query
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
        .build()
        .unwrap();
    let response = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "longhorn_bridge_hello".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({ "request": hello() })),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .unwrap()
    .deserialize::<BridgeNegotiationReceipt>()
    .unwrap();
    let query_response = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "longhorn_bridge_query".into(),
            callback: tauri::ipc::CallbackFn(2),
            error: tauri::ipc::CallbackFn(3),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({
                "route": "workspace.read",
                "request": query_request("request:parity", "session:current", DOMAIN)
            })),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .unwrap()
    .deserialize::<BridgeQueryReply<Value, Value>>()
    .unwrap();

    assert_eq!(response, direct_receipt);
    assert_eq!(query_response, direct_reply);
    assert!(app.try_state::<TauriBridgeState>().is_some());
}

fn query_only_assembly(
    dispatches: Arc<AtomicUsize>,
    write: WriteAuthority,
) -> Arc<BridgeHandlerAssembly<FixedAuthority>> {
    let descriptor = capabilities();
    let registry = registered_domains(dispatches);
    Arc::new(BridgeHandlerAssembly::new(
        FixedAuthority {
            receipt: authority_receipt(descriptor, write),
        },
        registry,
    ))
}

fn registered_domains(dispatches: Arc<AtomicUsize>) -> BridgeDomainRegistry {
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

fn authority_receipt(
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

fn authority_receipt_for(
    session: &str,
    capabilities: DomainCapabilityDescriptor,
    read: ReadAuthority,
    write: WriteAuthority,
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
                ExecutionAuthority::Executor,
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

fn hello() -> BridgeHelloRequest {
    BridgeHelloRequest::new(BridgeId::new("bridge:test").unwrap(), vec![domain()]).unwrap()
}

fn query_request(request: &str, session: &str, domain_id: &str) -> BridgeQueryEnvelope<Value> {
    BridgeQueryEnvelope::new(
        context(request, session, domain_id),
        json!({ "query": true }),
    )
}

fn context(request: &str, session: &str, domain_id: &str) -> BridgeRequestContext {
    BridgeRequestContext::new(
        BridgeRequestId::new(request).unwrap(),
        BridgeSessionId::new(session).unwrap(),
        DomainId::new(domain_id).unwrap(),
    )
}

fn capabilities() -> DomainCapabilityDescriptor {
    DomainCapabilityDescriptor::new(
        domain(),
        vec![capability(READ), capability(WRITE), capability(CANCEL)],
    )
    .unwrap()
}

fn domain() -> DomainId {
    DomainId::new(DOMAIN).unwrap()
}

fn capability(value: &str) -> BridgeCapabilityId {
    BridgeCapabilityId::new(value).unwrap()
}

fn cursor(sequence: u64) -> BridgeStreamCursor {
    BridgeStreamCursor::new(
        BridgeSessionId::new("session:current").unwrap(),
        domain(),
        AuthorityEpoch::new(1).unwrap(),
        BridgeStreamSequence::new(sequence),
    )
}

#[test]
fn authority_refresh_refuses_session_or_host_identity_change() {
    struct DriftingRefreshAuthority {
        initial: BridgeNegotiationReceipt,
        refreshed: BridgeNegotiationReceipt,
    }

    impl BridgeAuthorityProvider for DriftingRefreshAuthority {
        fn negotiate(
            &mut self,
            _caller: &str,
            _request: &BridgeHelloRequest,
            _registered_domains: &[DomainCapabilityDescriptor],
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(self.initial.clone())
        }

        fn refresh(
            &mut self,
            _caller: &str,
            _current: &BridgeNegotiationReceipt,
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(self.refreshed.clone())
        }
    }

    // Session identity change is refused.
    let initial = authority_receipt(capabilities(), WriteAuthority::None);
    let changed_session = authority_receipt_for(
        "session:different",
        capabilities(),
        ReadAuthority::Authoritative,
        WriteAuthority::None,
    );
    let session_drift = BridgeHandlerAssembly::new(
        DriftingRefreshAuthority {
            initial: initial.clone(),
            refreshed: changed_session,
        },
        registered_domains(Arc::new(AtomicUsize::new(0))),
    );
    let receipt = session_drift.hello("main", hello()).unwrap();
    let error = session_drift
        .authority("main", receipt.session_id())
        .unwrap_err();
    assert_eq!(error.code, BridgeHostErrorCode::InvalidAuthority);

    // Host identity change is refused.
    let changed_host = BridgeNegotiationReceipt::new(
        BridgeHostDescriptor {
            host_instance_id: HostInstanceId::new("host:other-instance").unwrap(),
            form: BridgeHostForm::TauriLocal,
        },
        receipt.session_id().clone(),
        BridgeConnectionStatus::new(
            BridgeConnectionState::Ready,
            Some(BridgeConnectionReason::NegotiationAccepted),
        )
        .unwrap(),
        AuthenticationPosture::NotRequired,
        Vec::new(),
        vec![capabilities()],
        vec![
            DomainAuthorityDescriptor::new(
                domain(),
                AuthorityScopeId::new("scope:workspace").unwrap(),
                DomainAvailability::Available,
                ReadAuthority::Authoritative,
                WriteAuthority::None,
                ExecutionAuthority::Executor,
                AuthorityEpoch::new(1).unwrap(),
                Some(AuthorityRevision::new(1)),
            )
            .unwrap(),
        ],
        Vec::new(),
    )
    .unwrap();
    let host_drift = BridgeHandlerAssembly::new(
        DriftingRefreshAuthority {
            initial,
            refreshed: changed_host,
        },
        registered_domains(Arc::new(AtomicUsize::new(0))),
    );
    let receipt = host_drift.hello("main", hello()).unwrap();
    let error = host_drift
        .authority("main", receipt.session_id())
        .unwrap_err();
    assert_eq!(error.code, BridgeHostErrorCode::InvalidAuthority);
}

#[test]
fn command_rejects_stale_authority_epoch_before_dispatch() {
    let dispatches = Arc::new(AtomicUsize::new(0));
    let assembly = query_only_assembly(Arc::clone(&dispatches), WriteAuthority::Authoritative);
    let receipt = assembly.hello("main", hello()).unwrap();

    let stale = BridgeCommandEnvelope::new(
        context("request:stale-epoch", receipt.session_id().as_str(), DOMAIN),
        AuthorityEpoch::new(99).unwrap(),
        None,
        None,
        json!({ "delta": 1 }),
    );
    let error = assembly
        .command("main", "workspace.write", stale)
        .unwrap_err();
    assert_eq!(error.code, BridgeHostErrorCode::StaleAuthority);
    assert!(error.retryable);
    assert_eq!(dispatches.load(Ordering::SeqCst), 0);
}

#[test]
fn rejects_mismatched_reply_request_id_and_cancellation_correlation() {
    let mut registry = BridgeDomainRegistry::new();
    registry.register_domain(capabilities()).unwrap();
    registry
        .register_query::<Value, Value, Value, _>(
            domain(),
            "workspace.read",
            capability(READ),
            |_request: BridgeQueryEnvelope<Value>| {
                Ok(BridgeQueryReply::new(
                    BridgeRequestId::new("request:wrong-query-reply").unwrap(),
                    BridgeQueryOutcome::Success(json!({})),
                ))
            },
        )
        .unwrap();
    registry
        .register_command::<Value, Value, Value, _>(
            domain(),
            "workspace.write",
            capability(WRITE),
            |_request: BridgeCommandEnvelope<Value>| {
                Ok(BridgeCommandReply::new(
                    BridgeRequestId::new("request:wrong-command-reply").unwrap(),
                    Some(AuthorityRevision::new(2)),
                    BridgeCommandOutcome::Applied(json!({})),
                ))
            },
        )
        .unwrap();
    registry
        .register_cancellation::<Value, _>(
            domain(),
            "workspace.cancel",
            capability(CANCEL),
            |request: longhorn_bridge::BridgeCancellationRequest| {
                Ok(BridgeCancellationReceipt::new(
                    request.context().request_id().clone(),
                    BridgeRequestId::new("request:wrong-target").unwrap(),
                    request.job_id().clone(),
                    BridgeCancellationOutcome::Accepted,
                ))
            },
        )
        .unwrap();

    let assembly = BridgeHandlerAssembly::new(
        FixedAuthority {
            receipt: authority_receipt(capabilities(), WriteAuthority::Authoritative),
        },
        registry,
    );
    let receipt = assembly.hello("main", hello()).unwrap();
    let session = receipt.session_id().as_str();

    let query_err = assembly
        .query(
            "main",
            "workspace.read",
            query_request("request:q1", session, DOMAIN),
        )
        .unwrap_err();
    assert_eq!(query_err.code, BridgeHostErrorCode::InvalidReply);

    let cmd_err = assembly
        .command(
            "main",
            "workspace.write",
            BridgeCommandEnvelope::new(
                context("request:c1", session, DOMAIN),
                AuthorityEpoch::new(1).unwrap(),
                None,
                None,
                json!({}),
            ),
        )
        .unwrap_err();
    assert_eq!(cmd_err.code, BridgeHostErrorCode::InvalidReply);

    let cancel_err = assembly
        .cancel(
            "main",
            "workspace.cancel",
            longhorn_bridge::BridgeCancellationRequest::new(
                context("request:cancel-1", session, DOMAIN),
                BridgeRequestId::new("request:target-1").unwrap(),
                longhorn_core::BridgeJobId::new("job:cancel-1").unwrap(),
            ),
        )
        .unwrap_err();
    assert_eq!(cancel_err.code, BridgeHostErrorCode::InvalidReply);
}

#[test]
fn handler_panic_poisons_route_and_reports_non_retryable_state_unavailable() {
    let mut registry = BridgeDomainRegistry::new();
    registry.register_domain(capabilities()).unwrap();
    registry
        .register_query::<Value, Value, Value, _>(
            domain(),
            "workspace.read",
            capability(READ),
            |_request: BridgeQueryEnvelope<Value>| -> Result<BridgeQueryReply<Value, Value>, BridgeHostError> {
                panic!("boom inside bridge query handler");
            },
        )
        .unwrap();

    let assembly = Arc::new(BridgeHandlerAssembly::new(
        FixedAuthority {
            receipt: authority_receipt(capabilities(), WriteAuthority::None),
        },
        registry,
    ));
    let receipt = assembly.hello("main", hello()).unwrap();
    let session = receipt.session_id().as_str().to_owned();

    let panicking = Arc::clone(&assembly);
    let panicking_session = session.clone();
    assert!(
        std::thread::spawn(move || {
            let _ = panicking.query(
                "main",
                "workspace.read",
                query_request("request:panic", &panicking_session, DOMAIN),
            );
        })
        .join()
        .is_err()
    );

    let error = assembly
        .query(
            "main",
            "workspace.read",
            query_request("request:after-panic", &session, DOMAIN),
        )
        .unwrap_err();
    assert_eq!(error.code, BridgeHostErrorCode::StateUnavailable);
    assert!(!error.retryable);
}
