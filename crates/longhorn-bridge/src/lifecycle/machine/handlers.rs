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
            None,
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
        let receipt = self.commit_at(
            BridgeConnectionState::Negotiating,
            BridgeConnectionReason::TransportReady,
            now,
            None,
        )?;
        self.reconnect_not_before = None;
        Ok(receipt)
    }

    /// Accepts checked negotiation only when all required authority is present.
    pub fn accept_negotiation(
        &mut self,
        receipt: &BridgeNegotiationReceipt,
        requirements: &[BridgeAuthorityRequirement],
        clock: &impl BridgeClock,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(BridgeConnectionState::Ready)?;
        self.sequence.next()?;
        validate_requirements(receipt, requirements)?;
        self.current_session_id = Some(receipt.session_id().clone());
        self.authority_epochs = receipt
            .domain_authorities()
            .iter()
            .map(|authority| (authority.domain_id().clone(), authority.authority_epoch()))
            .collect();
        self.reconnect_attempts = 0;
        self.reconnect_not_before = None;
        self.commit_unchecked(
            BridgeConnectionState::Ready,
            BridgeConnectionReason::NegotiationAccepted,
            clock,
            None,
        )
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
        self.commit(BridgeConnectionState::Degraded, reason, clock, None)
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
        let next_state = if admitted_attempt.is_some() {
            BridgeConnectionState::Reconnecting
        } else {
            BridgeConnectionState::Offline
        };
        self.require_transition(next_state)?;
        self.sequence.next()?;
        let Some(attempt) = admitted_attempt else {
            self.invalidate_authority();
            self.reconnect_not_before = None;
            return self.commit(
                BridgeConnectionState::Offline,
                BridgeConnectionReason::TransportLost,
                clock,
                None,
            );
        };
        let now = clock.now();
        let not_before = now
            .checked_add(backoff.delay(retry_class, attempt))
            .ok_or_else(|| {
                BridgeLifecycleError::new(
                    BridgeLifecycleErrorCode::DeadlineOverflow,
                    "bridge reconnect deadline overflow",
                )
            })?;
        self.invalidate_authority();
        self.reconnect_attempts = attempt.get();
        self.reconnect_not_before = Some(not_before);
        self.commit_at(
            BridgeConnectionState::Reconnecting,
            BridgeConnectionReason::RetryScheduled,
            now,
            Some(BridgeReconnectSchedule::new(
                attempt,
                retry_class,
                not_before,
            )),
        )
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
