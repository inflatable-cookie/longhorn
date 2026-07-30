use std::collections::BTreeMap;

use longhorn_core::{BridgeSessionId, DomainId};

use crate::{
    BridgeAuthorityCursorDecision, BridgeAuthorityRequirement, BridgeBackoffPolicy, BridgeClock,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus,
    BridgeConnectionTransitionReceipt, BridgeLifecycleError, BridgeLifecycleErrorCode,
    BridgeNegotiationReceipt, BridgeReconnectSchedule, BridgeRequiredAuthority, BridgeRetryClass,
    BridgeRetryLimit, BridgeStreamCursor, BridgeTransitionSequence, DomainAvailability,
    ExecutionAuthority, ReadAuthority, WriteAuthority,
};

/// Pure validated state machine for one selected bridge host.
#[derive(Clone, Debug)]
pub struct BridgeConnectionMachine {
    status: BridgeConnectionStatus,
    sequence: BridgeTransitionSequence,
    current_session_id: Option<BridgeSessionId>,
    authority_epochs: BTreeMap<DomainId, crate::AuthorityEpoch>,
    reconnect_limit: BridgeRetryLimit,
    reconnect_attempts: u32,
    reconnect_not_before: Option<crate::BridgeMonotonicMillis>,
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

    fn invalidate_authority(&mut self) {
        self.current_session_id = None;
        self.authority_epochs.clear();
    }

    fn commit(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.require_transition(state)?;
        self.commit_unchecked(state, reason, clock, reconnect)
    }

    fn commit_unchecked(
        &mut self,
        state: BridgeConnectionState,
        reason: BridgeConnectionReason,
        clock: &impl BridgeClock,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Result<BridgeConnectionTransitionReceipt, BridgeLifecycleError> {
        self.commit_at(state, reason, clock.now(), reconnect)
    }

    fn commit_at(
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

    fn require_transition(&self, next: BridgeConnectionState) -> Result<(), BridgeLifecycleError> {
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

fn validate_requirements(
    receipt: &BridgeNegotiationReceipt,
    requirements: &[BridgeAuthorityRequirement],
) -> Result<(), BridgeLifecycleError> {
    for requirement in requirements {
        let authority = receipt
            .domain_authorities()
            .iter()
            .find(|authority| authority.domain_id() == requirement.domain_id());
        let available = authority.is_some_and(|authority| {
            authority.availability() != DomainAvailability::Offline
                && match requirement.authority() {
                    BridgeRequiredAuthority::Available => true,
                    BridgeRequiredAuthority::Readable => {
                        authority.read_authority() != ReadAuthority::None
                    }
                    BridgeRequiredAuthority::AuthoritativeRead => {
                        authority.read_authority() == ReadAuthority::Authoritative
                    }
                    BridgeRequiredAuthority::Writable => {
                        authority.write_authority() == WriteAuthority::Authoritative
                    }
                    BridgeRequiredAuthority::Executable => {
                        authority.execution_authority() == ExecutionAuthority::Executor
                    }
                }
        });
        if !available {
            return Err(BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::RequiredAuthorityUnavailable,
                format!(
                    "required {:?} authority unavailable for domain {}",
                    requirement.authority(),
                    requirement.domain_id()
                ),
            ));
        }
    }
    Ok(())
}

fn invalid_transition(
    current: BridgeConnectionState,
    next: BridgeConnectionState,
) -> BridgeLifecycleError {
    BridgeLifecycleError::new(
        BridgeLifecycleErrorCode::InvalidTransition,
        format!("bridge connection cannot transition from {current:?} to {next:?}"),
    )
}
