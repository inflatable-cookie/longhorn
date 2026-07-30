use longhorn_bridge::{BridgeEventEnvelope, BridgeJobTerminalEvent, BridgeProgressEvent};
use longhorn_core::{BridgeSessionId, DomainId};
use serde::Serialize;

use crate::{
    BRIDGE_DOMAIN_EVENT, BRIDGE_PROGRESS_EVENT, BRIDGE_TERMINAL_EVENT, BridgeAuthorityProvider,
    BridgeHostError, BridgeHostErrorCode,
};

use super::{
    BridgeHandlerAssembly,
    authorization::{authority_for, ensure_execution},
};

impl<A> BridgeHandlerAssembly<A>
where
    A: BridgeAuthorityProvider,
{
    /// Publishes one checked current-session domain event.
    pub fn publish_domain_event<Payload: Serialize>(
        &self,
        event: &BridgeEventEnvelope<Payload>,
    ) -> Result<(), BridgeHostError> {
        let cursor = event.cursor();
        let record = self.session_by_id(cursor.session_id())?;
        let authority = authority_for(&record.receipt, cursor.domain_id())?;
        if cursor.authority_epoch() != authority.authority_epoch() {
            return Err(BridgeHostError::new(
                BridgeHostErrorCode::StaleAuthority,
                "event authority epoch is stale",
                false,
            ));
        }
        self.emit(BRIDGE_DOMAIN_EVENT, event)
    }

    /// Publishes one request-correlated progress event for current execution authority.
    pub fn publish_progress<Payload: Serialize>(
        &self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
        event: &BridgeProgressEvent<Payload>,
    ) -> Result<(), BridgeHostError> {
        let record = self.session_by_id(session_id)?;
        ensure_execution(authority_for(&record.receipt, domain_id)?)?;
        self.emit(BRIDGE_PROGRESS_EVENT, event)
    }

    /// Publishes one request-correlated terminal event for current execution authority.
    pub fn publish_terminal<Success: Serialize, Detail: Serialize>(
        &self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
        event: &BridgeJobTerminalEvent<Success, Detail>,
    ) -> Result<(), BridgeHostError> {
        let record = self.session_by_id(session_id)?;
        ensure_execution(authority_for(&record.receipt, domain_id)?)?;
        self.emit(BRIDGE_TERMINAL_EVENT, event)
    }

    fn emit<Payload: Serialize>(
        &self,
        event: &'static str,
        payload: &Payload,
    ) -> Result<(), BridgeHostError> {
        let Some(sink) = self.event_sink.as_ref() else {
            return Err(BridgeHostError::new(
                BridgeHostErrorCode::EventUnavailable,
                "bridge host has no event channel",
                false,
            ));
        };
        let value = serde_json::to_value(payload).map_err(|error| {
            BridgeHostError::new(BridgeHostErrorCode::PayloadCodec, error.to_string(), false)
        })?;
        sink.emit(event, value)
    }
}
