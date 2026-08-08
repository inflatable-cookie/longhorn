use std::collections::BTreeMap;

use longhorn_core::{BridgeSessionId, DomainId};

use crate::{
    BridgeConnectionState, BridgeConnectionStatus, BridgeRetryLimit, BridgeTransitionSequence,
};

/// Pure validated state machine for one selected bridge host.
#[derive(Clone, Debug)]
pub struct BridgeConnectionMachine {
    pub(crate) status: BridgeConnectionStatus,
    pub(crate) sequence: BridgeTransitionSequence,
    pub(crate) current_session_id: Option<BridgeSessionId>,
    pub(crate) authority_epochs: BTreeMap<DomainId, crate::AuthorityEpoch>,
    pub(crate) reconnect_limit: BridgeRetryLimit,
    pub(crate) reconnect_attempts: u32,
    pub(crate) reconnect_not_before: Option<crate::BridgeMonotonicMillis>,
}

impl BridgeConnectionMachine {
    /// Constructs an idle machine with an explicit reconnect ceiling.
    #[must_use]
    pub fn new(reconnect_limit: BridgeRetryLimit) -> Self {
        Self {
            status: BridgeConnectionStatus::new(BridgeConnectionState::Idle, None)
                .expect("idle connection status is valid"),
            sequence: BridgeTransitionSequence::default(),
            current_session_id: None,
            authority_epochs: BTreeMap::new(),
            reconnect_limit,
            reconnect_attempts: 0,
            reconnect_not_before: None,
        }
    }

    /// Returns current checked connection status.
    #[must_use]
    pub const fn status(&self) -> BridgeConnectionStatus {
        self.status
    }

    /// Returns current negotiated session, if ready or degraded.
    #[must_use]
    pub const fn current_session_id(&self) -> Option<&BridgeSessionId> {
        self.current_session_id.as_ref()
    }
}
