use std::sync::Arc;

use longhorn_bridge::{
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandEnvelope,
    BridgeCommandReply, BridgeHelloRequest, BridgeNegotiationReceipt, BridgeQueryEnvelope,
    BridgeQueryReply, BridgeSnapshotEnvelope,
};
use longhorn_core::{BridgeSessionId, DomainId};
use serde_json::Value;
use tauri::{Runtime, State, WebviewWindow};

use crate::{BridgeAuthorityProvider, BridgeHandlerAssembly, BridgeHostError};

/// Object-safe bridge command surface retained in Tauri managed state.
pub trait BridgeCommandService: Send + Sync {
    /// Negotiates one caller-owned bridge session.
    fn hello(
        &self,
        caller: &str,
        request: BridgeHelloRequest,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError>;

    /// Refreshes authority for one current caller session.
    fn authority(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError>;

    /// Dispatches one registered query route.
    fn query(
        &self,
        caller: &str,
        route: &str,
        request: BridgeQueryEnvelope<Value>,
    ) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError>;

    /// Dispatches one registered authoritative command route.
    fn command(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCommandEnvelope<Value>,
    ) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError>;

    /// Dispatches one registered cancellation route.
    fn cancel(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCancellationRequest,
    ) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError>;

    /// Loads one registered authoritative snapshot.
    fn resync(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError>;
}

impl<A> BridgeCommandService for BridgeHandlerAssembly<A>
where
    A: BridgeAuthorityProvider,
{
    fn hello(
        &self,
        caller: &str,
        request: BridgeHelloRequest,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Self::hello(self, caller, request)
    }

    fn authority(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
        Self::authority(self, caller, session_id)
    }

    fn query(
        &self,
        caller: &str,
        route: &str,
        request: BridgeQueryEnvelope<Value>,
    ) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError> {
        Self::query(self, caller, route, request)
    }

    fn command(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCommandEnvelope<Value>,
    ) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError> {
        Self::command(self, caller, route, request)
    }

    fn cancel(
        &self,
        caller: &str,
        route: &str,
        request: BridgeCancellationRequest,
    ) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError> {
        Self::cancel(self, caller, route, request)
    }

    fn resync(
        &self,
        caller: &str,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError> {
        Self::resync(self, caller, session_id, domain_id)
    }
}

/// Type-erased bridge commands installed once in Tauri managed state.
pub struct TauriBridgeState {
    service: Arc<dyn BridgeCommandService>,
}

impl TauriBridgeState {
    /// Wraps one explicitly injected real or mock host assembly.
    #[must_use]
    pub fn new(service: Arc<dyn BridgeCommandService>) -> Self {
        Self { service }
    }
}

/// Negotiates one bridge session for the invoking window.
#[tauri::command]
pub fn longhorn_bridge_hello<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    request: BridgeHelloRequest,
) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
    state.service.hello(window.label(), request)
}

/// Refreshes capability and authority facts for the invoking window.
#[tauri::command]
pub fn longhorn_bridge_authority<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    session_id: BridgeSessionId,
) -> Result<BridgeNegotiationReceipt, BridgeHostError> {
    state.service.authority(window.label(), &session_id)
}

/// Dispatches one registered query route.
#[tauri::command]
pub fn longhorn_bridge_query<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeQueryEnvelope<Value>,
) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError> {
    state.service.query(window.label(), &route, request)
}

/// Dispatches one registered authoritative command route.
#[tauri::command]
pub fn longhorn_bridge_command<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeCommandEnvelope<Value>,
) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError> {
    state.service.command(window.label(), &route, request)
}

/// Dispatches one registered cancellation route.
#[tauri::command]
pub fn longhorn_bridge_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    route: String,
    request: BridgeCancellationRequest,
) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError> {
    state.service.cancel(window.label(), &route, request)
}

/// Loads one registered authoritative domain snapshot.
#[tauri::command]
pub fn longhorn_bridge_resync<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, TauriBridgeState>,
    session_id: BridgeSessionId,
    domain_id: DomainId,
) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError> {
    state
        .service
        .resync(window.label(), &session_id, &domain_id)
}
