use crate::{
    BridgeAuthorityCursorDecision, BridgeAuthorityRequirement, BridgeBackoffPolicy, BridgeClock,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionTransitionReceipt,
    BridgeLifecycleError, BridgeLifecycleErrorCode, BridgeNegotiationReceipt,
    BridgeReconnectSchedule, BridgeRetryClass, BridgeStreamCursor,
};

use super::BridgeConnectionMachine;
use super::authority::validate_requirements;
use super::transitions::invalid_transition;

impl BridgeConnectionMachine {
    /// Begins the initial connection attempt.
    pub fn connect(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.commit(
            BridgeConnectionState::Connecting,
            BridgeConnectionReason::ConnectRequested,
            clock,
        )
    }

    /// Records transport availability and begins negotiation.
    pub fn transport_ready(
        &mut self,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        let now = clock.now();
        if self.status.state() == BridgeConnectionState::Reconnecting
            && self
                .reconnect_not_before
                .is_some_and(|not_before| now < not_before)
        {
            return Err(BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::RetryNotDue,
                "bridge reconnect backoff has not elapsed",
            ));
        }
        let receipt = self.prepare_transition(
            BridgeConnectionState::Negotiating,
            BridgeConnectionReason::TransportReady,
            self.current_session_id.as_ref(),
            || Ok((now, None)),
        )?;
        self.reconnect_not_before = None;
        Ok(self.apply_transition(receipt))
    }

    /// Accepts checked negotiation only when all required authority is present.
    pub fn accept_negotiation(
        &mut self,
        receipt: &BridgeNegotiationReceipt,
        requirements: &[BridgeAuthorityRequirement],
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        let transition = self.prepare_transition(
            BridgeConnectionState::Ready,
            BridgeConnectionReason::NegotiationAccepted,
            Some(receipt.session_id()),
            || {
                validate_requirements(receipt, requirements)?;
                Ok((clock.now(), None))
            },
        )?;
        self.current_session_id = Some(receipt.session_id().clone());
        self.authority_epochs = receipt
            .domain_authorities()
            .iter()
            .map(|authority| (authority.domain_id().clone(), authority.authority_epoch()))
            .collect();
        self.reconnect_attempts = 0;
        self.reconnect_not_before = None;
        Ok(self.apply_transition(transition))
    }

    /// Records reduced-but-usable posture.
    pub fn degrade(
        &mut self,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        if !matches!(
            reason,
            BridgeConnectionReason::CapabilityChanged
                | BridgeConnectionReason::TransportLost
                | BridgeConnectionReason::HostFailure
        ) {
            return Err(invalid_transition(
                self.status.state(),
                BridgeConnectionState::Degraded,
            ));
        }
        self.commit(BridgeConnectionState::Degraded, reason, clock)
    }

    /// Invalidates current authority and schedules a bounded reconnect.
    pub fn reconnect(
        &mut self,
        retry_class: BridgeRetryClass,
        clock: &impl BridgeClock,
        backoff: &impl BridgeBackoffPolicy,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        let admitted_attempt = if retry_class == BridgeRetryClass::Never {
            None
        } else {
            self.reconnect_limit.admit(self.reconnect_attempts)
        };
        let (state, reason) = match admitted_attempt {
            Some(_) => (
                BridgeConnectionState::Reconnecting,
                BridgeConnectionReason::RetryScheduled,
            ),
            None if retry_class == BridgeRetryClass::Never => (
                BridgeConnectionState::Offline,
                BridgeConnectionReason::RetryForbidden,
            ),
            None => (
                BridgeConnectionState::Offline,
                BridgeConnectionReason::TransportLost,
            ),
        };
        let receipt = self.prepare_transition(state, reason, None, || {
            let now = clock.now();
            let Some(attempt) = admitted_attempt else {
                return Ok((now, None));
            };
            let not_before = now
                .checked_add(backoff.delay(retry_class, attempt))
                .ok_or_else(|| {
                    BridgeLifecycleError::new(
                        BridgeLifecycleErrorCode::DeadlineOverflow,
                        "bridge reconnect deadline overflow",
                    )
                })?;
            Ok((
                now,
                Some(BridgeReconnectSchedule::new(
                    attempt,
                    retry_class,
                    not_before,
                )),
            ))
        })?;
        self.invalidate_authority();
        if let Some(schedule) = receipt.reconnect() {
            self.reconnect_attempts = schedule.attempt().get();
        }
        self.reconnect_not_before = receipt.reconnect().map(BridgeReconnectSchedule::not_before);
        Ok(self.apply_transition(receipt))
    }

    /// Classifies cursor session and authority tenure against current negotiation.
    #[must_use]
    pub fn classify_cursor(&self, cursor: &BridgeStreamCursor) -> BridgeAuthorityCursorDecision {
        if self.current_session_id.as_ref() != Some(cursor.session_id()) {
            return BridgeAuthorityCursorDecision::SupersededSession;
        }
        let Some(current_epoch) = self.authority_epochs.get(cursor.domain_id()) else {
            return BridgeAuthorityCursorDecision::UnknownDomain;
        };
        match cursor.authority_epoch().cmp(current_epoch) {
            std::cmp::Ordering::Less => BridgeAuthorityCursorDecision::StaleAuthority,
            std::cmp::Ordering::Equal => BridgeAuthorityCursorDecision::Current,
            std::cmp::Ordering::Greater => BridgeAuthorityCursorDecision::RefreshAuthority,
        }
    }
}
