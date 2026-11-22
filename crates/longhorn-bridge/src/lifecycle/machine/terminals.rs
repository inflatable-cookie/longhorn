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
        self.terminate(
            BridgeConnectionState::Incompatible,
            BridgeConnectionReason::VersionMismatch,
            clock,
        )
    }

    /// Records endpoint admission or authentication rejection.
    pub fn unauthorized(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.terminate(
            BridgeConnectionState::Unauthorized,
            BridgeConnectionReason::AuthorizationRejected,
            clock,
        )
    }

    /// Records a terminal host or transport failure.
    pub fn fail(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.terminate(
            BridgeConnectionState::Failed,
            BridgeConnectionReason::HostFailure,
            clock,
        )
    }

    /// Deliberately closes this connection and invalidates authority.
    pub fn close(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.terminate(
            BridgeConnectionState::Closed,
            BridgeConnectionReason::Shutdown,
            clock,
        )
    }

    fn terminate(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        let receipt = self.prepare_transition(state, reason, None, || Ok((clock.now(), None)))?;
        self.invalidate_authority();
        self.reconnect_not_before = None;
        Ok(self.apply_transition(receipt))
    }
}
