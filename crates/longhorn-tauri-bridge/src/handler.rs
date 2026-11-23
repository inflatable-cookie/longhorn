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

/// Shared registered-domain assembly used by direct and Tauri mock/real hosts.
pub struct BridgeHandlerAssembly<A: ?Sized> {
    registry: BridgeDomainRegistry,
    sessions: Mutex<BTreeMap<BridgeSessionId, SessionRecord>>,
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
            sessions: Mutex::new(BTreeMap::new()),
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
            sessions: Mutex::new(BTreeMap::new()),
            event_sink: Some(event_sink),
            authority: Mutex::new(authority),
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
        let domains: Vec<_> = self.registry.domains().cloned().collect();
        let receipt = self
            .authority
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?
            .negotiate(caller, &request, &domains)?;
        self.validate_receipt(&request, &receipt)?;

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?;
        if sessions
            .get(receipt.session_id())
            .is_some_and(|record| record.caller != caller)
        {
            return Err(invalid_authority(
                "authority provider reused a session id for another caller",
            ));
        }
        sessions.retain(|_, record| record.caller != caller);
        sessions.insert(
            receipt.session_id().clone(),
            SessionRecord {
                caller: caller.to_owned(),
                request,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    /// Drops every session a caller owns.
    ///
    /// Sessions end with their window: `bridge_plugin` wires this from Tauri's
    /// window-destroyed hook by default. Without it a destroyed window's
    /// session stays valid indefinitely, and each distinct caller label grows
    /// the map forever. Idempotent — tearing down a caller with no sessions is
    /// not an error, because destroy ordering against in-flight hellos is the
    /// host's to race.
    pub fn teardown(&self, caller: &str) -> Result<(), BridgeHostError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?;
        sessions.retain(|_, record| record.caller != caller);
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

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| BridgeHostError::state_unavailable())?;
        let Some(record) = sessions.get_mut(session_id) else {
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
            .get(session_id)
            .cloned()
            .ok_or_else(|| invalid_session(session_id))
    }
}
