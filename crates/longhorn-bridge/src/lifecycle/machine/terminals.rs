use crate::{
    BridgeClock, BridgeConnectionReason, BridgeConnectionState, BridgeConnectionTransitionReceipt,
    BridgeLifecycleError,
};

use super::BridgeConnectionMachine;

impl BridgeConnectionMachine {
    /// Records exact-version incompatibility.
    pub fn incompatible(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(BridgeConnectionState::Incompatible)?;
        self.sequence.next()?;
        self.invalidate_authority();
        self.commit(
            BridgeConnectionState::Incompatible,
            BridgeConnectionReason::VersionMismatch,
            clock,
            None,
        )
    }

    /// Records endpoint admission or authentication rejection.
    pub fn unauthorized(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(BridgeConnectionState::Unauthorized)?;
        self.sequence.next()?;
        self.invalidate_authority();
        self.commit(
            BridgeConnectionState::Unauthorized,
            BridgeConnectionReason::AuthorizationRejected,
            clock,
            None,
        )
    }

    /// Records a terminal host or transport failure.
    pub fn fail(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(BridgeConnectionState::Failed)?;
        self.sequence.next()?;
        self.invalidate_authority();
        self.commit(
            BridgeConnectionState::Failed,
            BridgeConnectionReason::HostFailure,
            clock,
            None,
        )
    }

    /// Deliberately closes this connection and invalidates authority.
    pub fn close(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(BridgeConnectionState::Closed)?;
        self.sequence.next()?;
        self.invalidate_authority();
        self.commit(
            BridgeConnectionState::Closed,
            BridgeConnectionReason::Shutdown,
            clock,
            None,
        )
    }
}
