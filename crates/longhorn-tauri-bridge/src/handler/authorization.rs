use longhorn_bridge::{
    BridgeNegotiationReceipt, DomainAuthorityDescriptor, DomainAvailability, ExecutionAuthority,
    ReadAuthority, WriteAuthority,
};
use longhorn_core::{BridgeCapabilityId, BridgeSessionId, DomainId};

use crate::{BridgeHostError, BridgeHostErrorCode, registration::RegisteredHandler};

#[derive(Clone, Copy)]
pub(super) enum RouteKind {
    Query,
    Command,
    Cancellation,
}

impl RouteKind {
    pub(super) fn matches(self, handler: &RegisteredHandler) -> bool {
        matches!(
            (self, handler),
            (Self::Query, RegisteredHandler::Query(_))
                | (Self::Command, RegisteredHandler::Command(_))
                | (Self::Cancellation, RegisteredHandler::Cancellation(_))
        )
    }
}

#[derive(Clone, Copy)]
pub(super) enum AuthorityNeed {
    Read,
    Write,
    Execution,
}

pub(super) fn authorize(
    receipt: &BridgeNegotiationReceipt,
    domain_id: &DomainId,
    capability: &BridgeCapabilityId,
    need: AuthorityNeed,
) -> Result<DomainAuthorityDescriptor, BridgeHostError> {
    let capability_available = receipt.domain_capabilities().iter().any(|descriptor| {
        descriptor.domain_id() == domain_id && descriptor.capabilities().contains(capability)
    });
    if !capability_available {
        return Err(BridgeHostError::new(
            BridgeHostErrorCode::CapabilityUnavailable,
            format!("capability {capability} is unavailable for domain {domain_id}"),
            false,
        ));
    }
    let authority = authority_for(receipt, domain_id)?.clone();
    if authority.availability() == DomainAvailability::Offline {
        return Err(denied_for(need, domain_id));
    }
    match need {
        AuthorityNeed::Read if authority.read_authority() == ReadAuthority::None => {
            Err(denied(BridgeHostErrorCode::ReadDenied, domain_id))
        }
        AuthorityNeed::Write if authority.write_authority() != WriteAuthority::Authoritative => {
            Err(denied(BridgeHostErrorCode::WriteDenied, domain_id))
        }
        AuthorityNeed::Execution
            if authority.execution_authority() != ExecutionAuthority::Executor =>
        {
            Err(denied(BridgeHostErrorCode::ExecutionDenied, domain_id))
        }
        _ => Ok(authority),
    }
}

pub(super) fn authority_for<'a>(
    receipt: &'a BridgeNegotiationReceipt,
    domain_id: &DomainId,
) -> Result<&'a DomainAuthorityDescriptor, BridgeHostError> {
    receipt
        .domain_authorities()
        .iter()
        .find(|authority| authority.domain_id() == domain_id)
        .ok_or_else(|| {
            BridgeHostError::new(
                BridgeHostErrorCode::UnknownDomain,
                format!("no negotiated authority for domain {domain_id}"),
                false,
            )
        })
}

pub(super) fn ensure_execution(
    authority: &DomainAuthorityDescriptor,
) -> Result<(), BridgeHostError> {
    if authority.availability() == DomainAvailability::Offline
        || authority.execution_authority() != ExecutionAuthority::Executor
    {
        Err(denied(
            BridgeHostErrorCode::ExecutionDenied,
            authority.domain_id(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn invalid_session(session_id: &BridgeSessionId) -> BridgeHostError {
    BridgeHostError::new(
        BridgeHostErrorCode::InvalidSession,
        format!("bridge session is not current for caller: {session_id}"),
        false,
    )
}

pub(super) fn unknown_route(route: &str) -> BridgeHostError {
    BridgeHostError::new(
        BridgeHostErrorCode::UnknownRoute,
        format!("bridge route is not registered for this operation: {route}"),
        false,
    )
}

pub(super) fn invalid_authority(message: impl Into<String>) -> BridgeHostError {
    BridgeHostError::new(BridgeHostErrorCode::InvalidAuthority, message, false)
}

pub(super) fn invalid_reply(message: impl Into<String>) -> BridgeHostError {
    BridgeHostError::new(BridgeHostErrorCode::InvalidReply, message, false)
}

fn denied_for(need: AuthorityNeed, domain_id: &DomainId) -> BridgeHostError {
    denied(
        match need {
            AuthorityNeed::Read => BridgeHostErrorCode::ReadDenied,
            AuthorityNeed::Write => BridgeHostErrorCode::WriteDenied,
            AuthorityNeed::Execution => BridgeHostErrorCode::ExecutionDenied,
        },
        domain_id,
    )
}

fn denied(code: BridgeHostErrorCode, domain_id: &DomainId) -> BridgeHostError {
    BridgeHostError::new(
        code,
        format!("bridge authority denied domain {domain_id}"),
        false,
    )
}
