use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeConnectionReason,
    BridgeConnectionState, BridgeConnectionStatus, BridgeHostDescriptor, BridgeHostForm,
    BridgeNegotiationReceipt, BridgeStreamCursor, BridgeStreamSequence, DomainAuthorityDescriptor,
    DomainAvailability, DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority,
    WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeSessionId, DomainId, HostInstanceId,
    TransportFeatureId,
};

pub(crate) fn domain(value: &str) -> DomainId {
    DomainId::new(value).unwrap()
}

pub(crate) fn capability(value: &str) -> BridgeCapabilityId {
    BridgeCapabilityId::new(value).unwrap()
}

pub(crate) fn cursor(
    session_id: &str,
    domain_id: &str,
    epoch: u64,
    sequence: u64,
) -> BridgeStreamCursor {
    BridgeStreamCursor::new(
        BridgeSessionId::new(session_id).unwrap(),
        domain(domain_id),
        AuthorityEpoch::new(epoch).unwrap(),
        BridgeStreamSequence::new(sequence),
    )
}

pub(crate) fn ready() -> BridgeConnectionStatus {
    BridgeConnectionStatus::new(
        BridgeConnectionState::Ready,
        Some(BridgeConnectionReason::NegotiationAccepted),
    )
    .unwrap()
}

pub(crate) fn host(identity: &str, form: BridgeHostForm) -> BridgeHostDescriptor {
    BridgeHostDescriptor {
        host_instance_id: HostInstanceId::new(identity).unwrap(),
        form,
    }
}

pub(crate) fn capabilities(
    domain_id: &str,
    advertised: &[&str],
) -> Result<DomainCapabilityDescriptor, longhorn_bridge::BridgeNegotiationError> {
    DomainCapabilityDescriptor::new(
        domain(domain_id),
        advertised.iter().map(|value| capability(value)).collect(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn authority(
    domain_id: &str,
    scope_id: &str,
    read: ReadAuthority,
    write: WriteAuthority,
    execution: ExecutionAuthority,
    epoch: u64,
    revision: Option<u64>,
) -> DomainAuthorityDescriptor {
    authority_with_availability(
        domain_id,
        scope_id,
        DomainAvailability::Available,
        read,
        write,
        execution,
        epoch,
        revision,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn authority_with_availability(
    domain_id: &str,
    scope_id: &str,
    availability: DomainAvailability,
    read: ReadAuthority,
    write: WriteAuthority,
    execution: ExecutionAuthority,
    epoch: u64,
    revision: Option<u64>,
) -> DomainAuthorityDescriptor {
    DomainAuthorityDescriptor::new(
        domain(domain_id),
        AuthorityScopeId::new(scope_id).unwrap(),
        availability,
        read,
        write,
        execution,
        AuthorityEpoch::new(epoch).unwrap(),
        revision.map(AuthorityRevision::new),
    )
    .unwrap()
}

pub(crate) fn receipt(
    host: BridgeHostDescriptor,
    session_id: &str,
    features: &[&str],
    domain_capabilities: Vec<DomainCapabilityDescriptor>,
    domain_authorities: Vec<DomainAuthorityDescriptor>,
) -> Result<BridgeNegotiationReceipt, longhorn_bridge::BridgeNegotiationError> {
    BridgeNegotiationReceipt::new(
        host,
        BridgeSessionId::new(session_id).unwrap(),
        ready(),
        AuthenticationPosture::NotRequired,
        features
            .iter()
            .map(|value| TransportFeatureId::new(*value).unwrap())
            .collect(),
        domain_capabilities,
        domain_authorities,
        Vec::new(),
    )
}
