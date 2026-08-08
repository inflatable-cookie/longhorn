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

    pub(crate) fn commit(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(state)?;
        self.commit_unchecked(state, reason, clock, reconnect)
    }

    pub(crate) fn commit_unchecked(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.commit_at(state, reason, clock.now(), reconnect)
    }

    pub(crate) fn commit_at(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        at: crate::BridgeMonotonicMillis,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(state)?;
        let previous = self.status;
        let current = BridgeConnectionStatus::new(state, Some(reason)).map_err(|error| {
            BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::InvalidTransition,
                error.to_string(),
            )
        })?;
        let sequence = self.sequence.next()?;
        self.status = current;
        self.sequence = sequence;
        Ok(BridgeConnectionTransitionReceipt::new(
            sequence,
            at,
            previous,
            current,
            self.current_session_id.clone(),
            reconnect,
        ))
    }

    pub(crate) fn require_transition(
        &self,
        next: BridgeConnectionState,
    ) -> Result<(), BridgeLifecycleError> {
        let current = self.status.state();
        let admitted = matches!(
            (current, next),
            (
                BridgeConnectionState::Idle,
                BridgeConnectionState::Connecting
            ) | (
                BridgeConnectionState::Connecting | BridgeConnectionState::Reconnecting,
                BridgeConnectionState::Negotiating
            ) | (
                BridgeConnectionState::Negotiating | BridgeConnectionState::Degraded,
                BridgeConnectionState::Ready
            ) | (
                BridgeConnectionState::Ready,
                BridgeConnectionState::Degraded
            ) | (
                BridgeConnectionState::Connecting
                    | BridgeConnectionState::Negotiating
                    | BridgeConnectionState::Ready
                    | BridgeConnectionState::Degraded
                    | BridgeConnectionState::Reconnecting,
                BridgeConnectionState::Reconnecting | BridgeConnectionState::Offline
            ) | (
                BridgeConnectionState::Offline,
                BridgeConnectionState::Reconnecting
            ) | (
                BridgeConnectionState::Negotiating,
                BridgeConnectionState::Incompatible | BridgeConnectionState::Unauthorized
            ) | (
                BridgeConnectionState::Connecting
                    | BridgeConnectionState::Negotiating
                    | BridgeConnectionState::Ready
                    | BridgeConnectionState::Degraded
                    | BridgeConnectionState::Reconnecting
                    | BridgeConnectionState::Offline,
                BridgeConnectionState::Failed
            ) | (
                BridgeConnectionState::Idle
                    | BridgeConnectionState::Connecting
                    | BridgeConnectionState::Negotiating
                    | BridgeConnectionState::Ready
                    | BridgeConnectionState::Degraded
                    | BridgeConnectionState::Reconnecting
                    | BridgeConnectionState::Offline
                    | BridgeConnectionState::Incompatible
                    | BridgeConnectionState::Unauthorized
                    | BridgeConnectionState::Failed,
                BridgeConnectionState::Closed
            )
        );
        if admitted {
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
