use crate::{
    BridgeClock, BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus,
    BridgeConnectionTransitionReceipt, BridgeLifecycleError, BridgeLifecycleErrorCode,
    BridgeReconnectSchedule,
};

use super::BridgeConnectionMachine;

impl BridgeConnectionMachine {
    pub(crate) fn invalidate_authority(&mut self) {
        self.current_session_id = None;
        self.authority_epochs.clear();
    }

    pub(super) fn commit(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        let receipt =
            self.prepare_transition(state, reason, self.current_session_id.as_ref(), || {
                Ok((clock.now(), None))
            })?;
        Ok(self.apply_transition(receipt))
    }

    pub(super) fn prepare_transition(
        &self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        session_id: Option<&longhorn_core::BridgeSessionId>,
        timing: impl FnOnce() -> Result<
            (
                crate::BridgeMonotonicMillis,
                Option<BridgeReconnectSchedule>,
            ),
            BridgeLifecycleError,
        >,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(state)?;
        let current = BridgeConnectionStatus::new(state, Some(reason)).map_err(|error| {
            BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::InvalidTransition,
                error.to_string(),
            )
        })?;
        let sequence = self.sequence.next()?;
        let (at, reconnect) = timing()?;
        Ok(BridgeConnectionTransitionReceipt::new(
            sequence,
            at,
            self.status,
            current,
            session_id.cloned(),
            reconnect,
        ))
    }

    pub(super) fn apply_transition(
        &mut self,
        receipt: BridgeConnectionTransitionReceipt,
    ) -> BridgeConnectionTransitionReceipt {
        self.status = receipt.current();
        self.sequence = receipt.sequence();
        receipt
    }

    fn require_transition(&self, next: BridgeConnectionState) -> Result<(), BridgeLifecycleError> {
        let current = self.status.state();
        if BridgeConnectionState::ADMITTED_TRANSITIONS.contains(&(current, next)) {
            Ok(())
        } else {
            Err(invalid_transition(current, next))
        }
    }
}

pub(crate) fn invalid_transition(
    current: BridgeConnectionState,
    next: BridgeConnectionState,
) -> BridgeLifecycleError {
    BridgeLifecycleError::new(
        BridgeLifecycleErrorCode::InvalidTransition,
        format!("bridge connection cannot transition from {current:?} to {next:?}"),
    )
}
