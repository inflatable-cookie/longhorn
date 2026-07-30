use std::{error::Error, fmt};

use longhorn_core::{BridgeSessionId, DomainId};
use serde::{Deserialize, Serialize};

use crate::{BridgeConnectionStatus, BridgeRetryClass};

/// Caller-supplied monotonic milliseconds from an arbitrary process-local epoch.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeMonotonicMillis(u64);

impl BridgeMonotonicMillis {
    /// Constructs a monotonic timestamp.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the caller's monotonic value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn checked_add(self, delay: BridgeDelayMillis) -> Option<Self> {
        self.0.checked_add(delay.get()).map(Self)
    }
}

/// Nonnegative backoff interval selected by an injected policy.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeDelayMillis(u64);

impl BridgeDelayMillis {
    /// Constructs a delay.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the delay in milliseconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One-based retry attempt inside a bounded retry controller.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeRetryAttempt(u32);

impl BridgeRetryAttempt {
    pub(crate) const fn new(value: u32) -> Self {
        debug_assert!(value > 0);
        Self(value)
    }

    /// Returns the one-based attempt number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Monotonic connection transition sequence.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeTransitionSequence(u64);

impl BridgeTransitionSequence {
    pub(crate) fn next(self) -> Result<Self, BridgeLifecycleError> {
        self.0.checked_add(1).map(Self).ok_or_else(|| {
            BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::SequenceExhausted,
                "bridge transition sequence exhausted",
            )
        })
    }

    /// Returns the serialized sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Scheduled retry evidence returned to an adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeReconnectSchedule {
    attempt: BridgeRetryAttempt,
    retry_class: BridgeRetryClass,
    not_before: BridgeMonotonicMillis,
}

impl BridgeReconnectSchedule {
    pub(crate) const fn new(
        attempt: BridgeRetryAttempt,
        retry_class: BridgeRetryClass,
        not_before: BridgeMonotonicMillis,
    ) -> Self {
        Self {
            attempt,
            retry_class,
            not_before,
        }
    }

    /// Returns the one-based retry attempt.
    #[must_use]
    pub const fn attempt(self) -> BridgeRetryAttempt {
        self.attempt
    }

    /// Returns the failure timing class that admitted this retry.
    #[must_use]
    pub const fn retry_class(self) -> BridgeRetryClass {
        self.retry_class
    }

    /// Returns the earliest monotonic retry time.
    #[must_use]
    pub const fn not_before(self) -> BridgeMonotonicMillis {
        self.not_before
    }
}

/// One committed connection-state change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeConnectionTransitionReceipt {
    sequence: BridgeTransitionSequence,
    at: BridgeMonotonicMillis,
    previous: BridgeConnectionStatus,
    current: BridgeConnectionStatus,
    session_id: Option<BridgeSessionId>,
    reconnect: Option<BridgeReconnectSchedule>,
}

impl BridgeConnectionTransitionReceipt {
    pub(crate) const fn new(
        sequence: BridgeTransitionSequence,
        at: BridgeMonotonicMillis,
        previous: BridgeConnectionStatus,
        current: BridgeConnectionStatus,
        session_id: Option<BridgeSessionId>,
        reconnect: Option<BridgeReconnectSchedule>,
    ) -> Self {
        Self {
            sequence,
            at,
            previous,
            current,
            session_id,
            reconnect,
        }
    }

    /// Returns the transition sequence.
    #[must_use]
    pub const fn sequence(&self) -> BridgeTransitionSequence {
        self.sequence
    }

    /// Returns the committed current state.
    #[must_use]
    pub const fn current(&self) -> BridgeConnectionStatus {
        self.current
    }

    /// Returns the negotiated current session, if any.
    #[must_use]
    pub const fn session_id(&self) -> Option<&BridgeSessionId> {
        self.session_id.as_ref()
    }

    /// Returns a scheduled reconnect, if this transition admitted one.
    #[must_use]
    pub const fn reconnect(&self) -> Option<BridgeReconnectSchedule> {
        self.reconnect
    }
}

/// Authority posture a consumer requires before the connection becomes ready.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeRequiredAuthority {
    /// The domain must be online.
    Available,
    /// Any read authority is required.
    Readable,
    /// Authoritative reads are required.
    AuthoritativeRead,
    /// Current write authority is required.
    Writable,
    /// Current execution authority is required.
    Executable,
}

/// Required domain authority checked before accepting negotiation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeAuthorityRequirement {
    domain_id: DomainId,
    authority: BridgeRequiredAuthority,
}

impl BridgeAuthorityRequirement {
    /// Constructs one consumer-declared readiness requirement.
    #[must_use]
    pub const fn new(domain_id: DomainId, authority: BridgeRequiredAuthority) -> Self {
        Self {
            domain_id,
            authority,
        }
    }

    /// Returns the required domain.
    #[must_use]
    pub const fn domain_id(&self) -> &DomainId {
        &self.domain_id
    }

    /// Returns the required authority posture.
    #[must_use]
    pub const fn authority(&self) -> BridgeRequiredAuthority {
        self.authority
    }
}

/// Classification of session and authority-epoch evidence against current negotiation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeAuthorityCursorDecision {
    /// The cursor belongs to the current negotiated authority.
    Current,
    /// The cursor belongs to a superseded or unknown session.
    SupersededSession,
    /// The cursor carries an older authority tenure.
    StaleAuthority,
    /// The cursor carries a newer unnegotiated authority tenure.
    RefreshAuthority,
    /// The negotiated receipt did not advertise this domain.
    UnknownDomain,
}

/// Stable lifecycle failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeLifecycleErrorCode {
    /// The requested state transition is not admitted.
    InvalidTransition,
    /// Negotiation did not provide required domain authority.
    RequiredAuthorityUnavailable,
    /// Monotonic deadline arithmetic overflowed.
    DeadlineOverflow,
    /// A reconnect transport became ready before its admitted backoff elapsed.
    RetryNotDue,
    /// Transition generation exhausted its integer domain.
    SequenceExhausted,
}

/// Pure connection-lifecycle validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeLifecycleError {
    code: BridgeLifecycleErrorCode,
    detail: String,
}

impl BridgeLifecycleError {
    pub(crate) fn new(code: BridgeLifecycleErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn code(&self) -> BridgeLifecycleErrorCode {
        self.code
    }
}

impl fmt::Display for BridgeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for BridgeLifecycleError {}
