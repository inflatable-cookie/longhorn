use std::cell::Cell;

use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, BridgeBackoffPolicy, BridgeClock,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus, BridgeDelayMillis,
    BridgeHostDescriptor, BridgeHostForm, BridgeMonotonicMillis, BridgeNegotiationReceipt,
    BridgeRetryAttempt, BridgeRetryClass, DomainAuthorityDescriptor, DomainAvailability,
    DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority, WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeSessionId, DomainId, HostInstanceId,
};

pub struct Clock(Cell<u64>);

impl Clock {
    pub const fn new(value: u64) -> Self {
        Self(Cell::new(value))
    }

    pub fn set(&self, value: u64) {
        self.0.set(value);
    }
}

impl BridgeClock for Clock {
    fn now(&self) -> BridgeMonotonicMillis {
        BridgeMonotonicMillis::new(self.0.get())
    }
}

pub struct LinearBackoff;

impl BridgeBackoffPolicy for LinearBackoff {
    fn delay(
        &self,
        _retry_class: BridgeRetryClass,
        attempt: BridgeRetryAttempt,
    ) -> BridgeDelayMillis {
        BridgeDelayMillis::new(u64::from(attempt.get()) * 25)
    }
}

pub fn domain(value: &str) -> DomainId {
    DomainId::new(value).unwrap()
}

pub fn session(value: &str) -> BridgeSessionId {
    BridgeSessionId::new(value).unwrap()
}

pub fn receipt(session_id: &str, epoch: u64, writable: bool) -> BridgeNegotiationReceipt {
    let domain_id = domain("example.workspace");
    BridgeNegotiationReceipt::new(
        BridgeHostDescriptor {
            host_instance_id: HostInstanceId::new("host:workspace").unwrap(),
            form: BridgeHostForm::LocalService,
        },
        session(session_id),
        BridgeConnectionStatus::new(
            BridgeConnectionState::Ready,
            Some(BridgeConnectionReason::NegotiationAccepted),
        )
        .unwrap(),
        AuthenticationPosture::Authenticated,
        Vec::new(),
        vec![
            DomainCapabilityDescriptor::new(
                domain_id.clone(),
                vec![BridgeCapabilityId::new("workspace:read").unwrap()],
            )
            .unwrap(),
        ],
        vec![
            DomainAuthorityDescriptor::new(
                domain_id,
                AuthorityScopeId::new("scope:workspace").unwrap(),
                DomainAvailability::Available,
                ReadAuthority::Authoritative,
                if writable {
                    WriteAuthority::Authoritative
                } else {
                    WriteAuthority::None
                },
                ExecutionAuthority::None,
                AuthorityEpoch::new(epoch).unwrap(),
                None,
            )
            .unwrap(),
        ],
        Vec::new(),
    )
    .unwrap()
}
