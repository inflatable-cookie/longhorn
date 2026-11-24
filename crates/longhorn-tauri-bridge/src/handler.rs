use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use longhorn_bridge::{
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandEnvelope,
    BridgeCommandReply, BridgeHelloRequest, BridgeHostForm, BridgeNegotiationReceipt,
    BridgeQueryEnvelope, BridgeQueryReply, BridgeRequestContext, BridgeSnapshotEnvelope,
    DomainAuthorityDescriptor,
};
use longhorn_core::{BridgeSessionId, DomainId};
use serde_json::Value;

use crate::{
    BridgeAuthorityProvider, BridgeDomainRegistry, BridgeEventSink, BridgeHostError,
    BridgeHostErrorCode, registration::RegisteredRoute,
};

mod authorization;
mod publication;

pub(crate) use authorization::caller_torn_down;
use authorization::{
    AuthorityNeed, RouteKind, authorize, invalid_authority, invalid_reply, invalid_session,
    unknown_route,
};

#[derive(Clone)]
struct SessionRecord {
    caller: String,
    request: BridgeHelloRequest,
    receipt: BridgeNegotiationReceipt,
}

#[derive(Debug)]
pub(crate) struct HelloTicket(u64);

struct SessionStore {
    sessions: BTreeMap<BridgeSessionId, SessionRecord>,
    next_hello_id: u64,
    in_flight_hellos: BTreeMap<u64, String>,
}

impl SessionStore {
    const fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            next_hello_id: 0,
            in_flight_hellos: BTreeMap::new(),
        }
    }

    fn begin_hello(&mut self, caller: &str) -> Result<HelloTicket, BridgeHostError> {
        let id = self.next_hello_id;
        self.next_hello_id = self
            .next_hello_id
            .checked_add(1)
            .ok_or_else(BridgeHostError::state_unavailable)?;
        self.in_flight_hellos.insert(id, caller.to_owned());
        Ok(HelloTicket(id))
    }

    fn is_hello_active(&self, caller: &str, ticket: &HelloTicket) -> bool {
        self.in_flight_hellos.get(&ticket.0).map(String::as_str) == Some(caller)
    }

    fn abandon_hello(&mut self, ticket: HelloTicket) {
        self.in_flight_hellos.remove(&ticket.0);
    }

    fn teardown(&mut self, caller: &str) {
        self.sessions.retain(|_, record| record.caller != caller);
        self.in_flight_hellos
            .retain(|_, in_flight_caller| in_flight_caller != caller);
    }
}

struct ActiveHelloGuard<'a, A: ?Sized> {
    assembly: &'a BridgeHandlerAssembly<A>,
    ticket: Option<HelloTicket>,
}

impl<A: ?Sized> Drop for ActiveHelloGuard<'_, A> {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            self.assembly.abandon_hello(ticket);
        }
    }
}

/// Shared registered-domain assembly used by direct and Tauri mock/real hosts.
pub struct BridgeHandlerAssembly<A: ?Sized> {
    registry: BridgeDomainRegistry,
    sessions: Mutex<SessionStore>,
    event_sink: Option<Arc<dyn BridgeEventSink>>,
    authority: Mutex<A>,
}

impl<A> BridgeHandlerAssembly<A>
where
    A: BridgeAuthorityProvider,
{
    /// Constructs a query-capable host with no event channel.
    #[must_use]
    pub const fn new(authority: A, registry: BridgeDomainRegistry) -> Self {
        Self {
            registry,
            sessions: Mutex::new(SessionStore::new()),
            event_sink: None,
            authority: Mutex::new(authority),
        }
    }

    /// Constructs a subscription-capable host with an injected event sink.
    #[must_use]
    pub fn with_event_sink(
        authority: A,
        registry: BridgeDomainRegistry,
        event_sink: Arc<dyn BridgeEventSink>,
    ) -> Self {
        Self {
            registry,
            sessions: Mutex::new(SessionStore::new()),
            event_sink: Some(event_sink),
            authority: Mutex::new(authority),
        }
    }
}

impl<A: ?Sized> BridgeHandlerAssembly<A> {
    pub(crate) fn abandon_hello(&self, ticket: HelloTicket) {
        if let Ok(mut store) = self.sessions.lock() {
            store.abandon_hello(ticket);
        }
    }
}

impl<A> BridgeHandlerAssembly<A>
where
    A: BridgeAuthorityProvider + ?Sized,
{
    /// Negotiates and records one caller-owned current session.
    pub fn hello(
        &self,
        caller: &str,
        request: BridgeHelloRequest,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        let ticket = self.begin_hello(caller)?;
        self.hello_with_ticket(caller, request, ticket)
    }

    pub(crate) fn begin_hello(&self, caller: &str) -> Result<HelloTicket, BridgeHostError> {
        self.sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .begin_hello(caller)
    }

    pub(crate) fn hello_with_ticket(
        &self,
        caller: &str,
        request: BridgeHelloRequest,
        ticket: HelloTicket,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        let mut guard = ActiveHelloGuard {
            assembly: self,
            ticket: Some(ticket),
        };
        let active = {
            let ticket_ref = guard
                .ticket
                .as_ref()
                .ok_or_else(BridgeHostError::state_unavailable)?;
            self.sessions
                .lock()
                .map_err(|_| BridgeHostError::state_unavailable())?
                .is_hello_active(caller, ticket_ref)
        };
        if !active {
            return Err(caller_torn_down(caller));
        }

        let domains: Vec<_> = self.registry.domains().cloned().collect();
        let receipt = self
            .authority
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .negotiate(caller, &request, &domains)?;
        self.validate_receipt(&request, &receipt)?;

        let ticket = guard
            .ticket
            .take()
            .ok_or_else(BridgeHostError::state_unavailable)?;
        self.commit_hello(caller, request, receipt, ticket)
    }

    fn commit_hello(
        &self,
        caller: &str,
        request: BridgeHelloRequest,
        receipt: BridgeNegotiationReceipt,
        ticket: HelloTicket,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        let mut store = self
            .sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?;
        if !store.is_hello_active(caller, &ticket) {
            store.abandon_hello(ticket);
            return Err(caller_torn_down(caller));
        }
        if store
            .sessions
            .get(receipt.session_id())
            .is_some_and(|record| record.caller != caller)
        {
            store.abandon_hello(ticket);
            return Err(invalid_authority(
                "authority provider reused a session id for another caller",
            ));
        }
        store.abandon_hello(ticket);
        store.sessions.retain(|_, record| record.caller != caller);
        store.sessions.insert(
            receipt.session_id().clone(),
            SessionRecord {
                caller: caller.to_owned(),
                request,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    /// Drops every session a caller owns and cancels any in-flight `hello`
    /// negotiation for that caller.
    ///
    /// Sessions end with their window: `bridge_plugin` wires this from Tauri's
    /// window-destroyed hook by default. Without it a destroyed window's
    /// session stays valid indefinitely, and each distinct caller label grows
    /// the map forever. Idempotent — tearing down a caller with no active
    /// session is not an error, and any in-flight `hello` that started before
    /// teardown is fenced from inserting a session once negotiation returns.
    pub fn teardown(&self, caller: &str) -> Result<(), BridgeHostError> {
        self.sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .teardown(caller);
        Ok(())
    }

    /// Refreshes current capability and authority facts for one caller session.
    pub fn authority(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        let current = self.session(caller, session_id)?;
        let receipt = self
            .authority
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .refresh(caller, &current.receipt)?;
        self.validate_receipt(&current.request, &receipt)?;
        if receipt.session_id() != session_id || receipt.host() != current.receipt.host() {
            return Err(invalid_authority(
                "authority refresh changed session or host identity",
            ));
        }

        let mut store = self
            .sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?;
        let Some(record) = store.sessions.get_mut(session_id) else {
            return Err(invalid_session(session_id));
        };
        if record.caller != caller {
            return Err(invalid_session(session_id));
        }
        record.receipt = receipt.clone();
        Ok(receipt)
    }

    /// Dispatches one already-registered typed query route.
    pub fn query(
        &self,
        caller: &str,
        route: &str,
        request: BridgeQueryEnvelope<Value>,
    ) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError> {
        let registration = self.route(route, request.context(), RouteKind::Query)?;
        self.authorize_route(caller, request.context(), registration, AuthorityNeed::Read)?;
        let crate::registration::RegisteredHandler::Query(handler) = &registration.handler else {
            return Err(unknown_route(route));
        };
        let request_id = request.context().request_id().clone();
        let reply = handler
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?(request)?;
        if reply.request_id() != &request_id {
            return Err(invalid_reply("query reply request id mismatch"));
        }
        Ok(reply)
    }

    /// Dispatches one already-registered typed authoritative command route.
    pub fn command(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCommandEnvelope<Value>,
    ) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError> {
        let registration = self.route(route, request.context(), RouteKind::Command)?;
        let authority = self.authorize_route(
            caller,
            request.context(),
            registration,
            AuthorityNeed::Write,
        )?;
        if request.authority_epoch() != authority.authority_epoch() {
            return Err(BridgeHostError::new(
                BridgeHostErrorCode::StaleAuthority,
                "command authority epoch is stale",
                true,
            ));
        }
        let crate::registration::RegisteredHandler::Command(handler) = &registration.handler else {
            return Err(unknown_route(route));
        };
        let request_id = request.context().request_id().clone();
        let reply = handler
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?(request)?;
        if reply.request_id() != &request_id {
            return Err(invalid_reply("command reply request id mismatch"));
        }
        Ok(reply)
    }

    /// Dispatches one already-registered typed cancellation route.
    pub fn cancel(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCancellationRequest,
    ) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError> {
        let registration = self.route(route, request.context(), RouteKind::Cancellation)?;
        self.authorize_route(
            caller,
            request.context(),
            registration,
            AuthorityNeed::Execution,
        )?;
        let crate::registration::RegisteredHandler::Cancellation(handler) = &registration.handler
        else {
            return Err(unknown_route(route));
        };
        let request_id = request.context().request_id().clone();
        let target_request_id = request.target_request_id().clone();
        let job_id = request.job_id().clone();
        let reply = handler
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?(request)?;
        if reply.request_id() != &request_id
            || reply.target_request_id() != &target_request_id
            || reply.job_id() != &job_id
        {
            return Err(invalid_reply("cancellation receipt correlation mismatch"));
        }
        Ok(reply)
    }

    /// Loads one registered authoritative snapshot after current read checks.
    pub fn resync(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError> {
        let Some(snapshot) = self.registry.snapshot(domain_id) else {
            return Err(BridgeHostError::new(
                BridgeHostErrorCode::UnknownDomain,
                format!("no snapshot handler is registered for domain {domain_id}"),
                false,
            ));
        };
        let receipt = self.session(caller, session_id)?.receipt;
        let authority = authorize(
            &receipt,
            domain_id,
            &snapshot.required_capability,
            AuthorityNeed::Read,
        )?;
        let value = snapshot
            .handler
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?(
            session_id, domain_id
        )?;
        let cursor = value.cursor();
        if cursor.session_id() != session_id
            || cursor.domain_id() != domain_id
            || cursor.authority_epoch() != authority.authority_epoch()
        {
            return Err(invalid_reply("snapshot authority metadata mismatch"));
        }
        Ok(value)
    }

    fn route(
        &self,
        route: &str,
        context: &BridgeRequestContext,
        kind: RouteKind,
    ) -> Result<&RegisteredRoute, BridgeHostError> {
        let Some(registration) = self.registry.route(route) else {
            return Err(unknown_route(route));
        };
        if !kind.matches(&registration.handler) || context.domain_id() != &registration.domain_id {
            return Err(unknown_route(route));
        }
        Ok(registration)
    }

    fn authorize_route(
        &self,
        caller: &str,
        context: &BridgeRequestContext,
        route: &RegisteredRoute,
        need: AuthorityNeed,
    ) -> Result<DomainAuthorityDescriptor, BridgeHostError> {
        let receipt = self.session(caller, context.session_id())?.receipt;
        authorize(
            &receipt,
            context.domain_id(),
            &route.required_capability,
            need,
        )
    }

    fn validate_receipt(
        &self,
        request: &BridgeHelloRequest,
        receipt: &BridgeNegotiationReceipt,
    ) -> Result<(), BridgeHostError> {
        if receipt.host().form != BridgeHostForm::TauriLocal {
            return Err(invalid_authority(
                "Tauri bridge provider returned a non-Tauri host form",
            ));
        }
        receipt
            .validate_for(request)
            .map_err(|error| invalid_authority(error.to_string()))?;
        for advertised in receipt.domain_capabilities() {
            if self.registry.domain(advertised.domain_id()) != Some(advertised) {
                return Err(invalid_authority(format!(
                    "provider capability does not match registered domain {}",
                    advertised.domain_id()
                )));
            }
        }
        Ok(())
    }

    fn session(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
    ) -> Result<SessionRecord, BridgeHostError> {
        let record = self.session_by_id(session_id)?;
        if record.caller != caller {
            return Err(invalid_session(session_id));
        }
        Ok(record)
    }

    fn session_by_id(
        &self,
        session_id: &BridgeSessionId,
    ) -> Result<SessionRecord, BridgeHostError> {
        self.sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| invalid_session(session_id))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use longhorn_bridge::{
        AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeConnectionReason,
        BridgeConnectionState, BridgeConnectionStatus, BridgeHelloRequest, BridgeHostDescriptor,
        BridgeHostForm, BridgeNegotiationReceipt, DomainAuthorityDescriptor, DomainAvailability,
        DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority, WriteAuthority,
    };
    use longhorn_core::{
        AuthorityScopeId, BridgeCapabilityId, BridgeId, BridgeSessionId, DomainId, HostInstanceId,
    };

    use super::*;

    struct SequentialAuthority {
        next: AtomicUsize,
        descriptor: DomainCapabilityDescriptor,
    }

    impl BridgeAuthorityProvider for SequentialAuthority {
        fn negotiate(
            &mut self,
            _caller: &str,
            _request: &BridgeHelloRequest,
            _registered_domains: &[DomainCapabilityDescriptor],
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            let n = self.next.fetch_add(1, Ordering::SeqCst);
            Ok(BridgeNegotiationReceipt::new(
                BridgeHostDescriptor {
                    host_instance_id: HostInstanceId::new("host:unit").unwrap(),
                    form: BridgeHostForm::TauriLocal,
                },
                BridgeSessionId::new(format!("session:gen-{n}")).unwrap(),
                BridgeConnectionStatus::new(
                    BridgeConnectionState::Ready,
                    Some(BridgeConnectionReason::NegotiationAccepted),
                )
                .unwrap(),
                AuthenticationPosture::NotRequired,
                Vec::new(),
                vec![self.descriptor.clone()],
                vec![
                    DomainAuthorityDescriptor::new(
                        DomainId::new("test.workspace").unwrap(),
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
            .unwrap())
        }

        fn refresh(
            &mut self,
            _caller: &str,
            current: &BridgeNegotiationReceipt,
        ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
            Ok(current.clone())
        }
    }

    #[test]
    fn queued_hello_ticket_from_destroyed_window_cannot_clobber_reopened_same_label_session() {
        let domain_id = DomainId::new("test.workspace").unwrap();
        let descriptor = DomainCapabilityDescriptor::new(
            domain_id,
            vec![BridgeCapabilityId::new("workspace.read").unwrap()],
        )
        .unwrap();
        let mut registry = BridgeDomainRegistry::new();
        registry.register_domain(descriptor.clone()).unwrap();

        let assembly = BridgeHandlerAssembly::new(
            SequentialAuthority {
                next: AtomicUsize::new(0),
                descriptor,
            },
            registry,
        );
        let req = || {
            BridgeHelloRequest::new(
                BridgeId::new("bridge:test").unwrap(),
                vec![DomainId::new("test.workspace").unwrap()],
            )
            .unwrap()
        };

        let stale_ticket = assembly.begin_hello("main").unwrap();
        assembly.teardown("main").unwrap();

        // A reopened window with the same label negotiates and owns "main".
        let live_receipt = assembly.hello("main", req()).unwrap();
        assert_eq!(live_receipt.session_id().as_str(), "session:gen-0");

        // The stale ticket queued before teardown fails and leaves live_receipt intact.
        let err = assembly
            .hello_with_ticket("main", req(), stale_ticket)
            .unwrap_err();
        assert_eq!(err.code, BridgeHostErrorCode::InvalidSession);
        assert!(assembly.session("main", live_receipt.session_id()).is_ok());
    }
}
