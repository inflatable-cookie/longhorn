use longhorn_bridge::{BridgeEventEnvelope, BridgeJobTerminalEvent, BridgeProgressEvent};
use longhorn_core::{BridgeSessionId, DomainId};
use serde::Serialize;

use crate::{
    BRIDGE_DOMAIN_EVENT, BRIDGE_PROGRESS_EVENT, BRIDGE_TERMINAL_EVENT, BridgeAuthorityProvider,
    BridgeHostError, BridgeHostErrorCode,
};

use super::{
    BridgeHandlerAssembly,
    authorization::{authority_for, ensure_execution, ensure_read},
};

impl<A> BridgeHandlerAssembly<A>
where
    A: BridgeAuthorityProvider + ?Sized,
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
        // Delivery is targeted at the session's own window, so a full
        // payload crosses only where the session may read.
        ensure_read(authority)?;
        let target = record.caller.clone();
        self.emit(&target, BRIDGE_DOMAIN_EVENT, event)
    }

    /// Publishes one request-correlated progress event to a session holding
    /// current execution and read authority.
    pub fn publish_progress<Payload: Serialize>(
        &self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
        event: &BridgeProgressEvent<Payload>,
    ) -> Result<(), BridgeHostError> {
        let target = self.job_event_target(session_id, domain_id)?;
        self.emit(&target, BRIDGE_PROGRESS_EVENT, event)
    }

    /// Publishes one request-correlated terminal event to a session holding
    /// current execution and read authority.
    pub fn publish_terminal<Success: Serialize, Detail: Serialize>(
        &self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
        event: &BridgeJobTerminalEvent<Success, Detail>,
    ) -> Result<(), BridgeHostError> {
        let target = self.job_event_target(session_id, domain_id)?;
        self.emit(&target, BRIDGE_TERMINAL_EVENT, event)
    }

    /// The window a job event may cross into. Execution authority admits the
    /// job; read authority admits its payload.
    fn job_event_target(
        &self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<String, BridgeHostError> {
        let record = self.session_by_id(session_id)?;
        let authority = authority_for(&record.receipt, domain_id)?;
        ensure_execution(authority)?;
        ensure_read(authority)?;
        Ok(record.caller)
    }

    fn emit<Payload: Serialize>(
        &self,
        target: &str,
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
        sink.emit(target, event, value)
    }
}
