use std::{error::Error, fmt};

use crate::{
    BridgeDelayMillis, BridgeMonotonicMillis, BridgeQueryRetryDecision, BridgeReconnectSchedule,
    BridgeRetryAttempt, BridgeRetryClass,
};

/// Defensive ceiling for one automatic retry controller.
pub const MAXIMUM_AUTOMATIC_RETRIES: u32 = 64;

/// Injected process-local monotonic clock.
pub trait BridgeClock {
    /// Returns milliseconds from the clock's arbitrary epoch.
    fn now(&self) -> BridgeMonotonicMillis;
}

/// Injected backoff selection. Retry admission remains in Longhorn.
pub trait BridgeBackoffPolicy {
    /// Returns the delay for an already-admitted one-based attempt.
    fn delay(
        &self,
        retry_class: BridgeRetryClass,
        attempt: BridgeRetryAttempt,
    ) -> BridgeDelayMillis;
}

/// Bounded automatic retry count. Zero disables automatic retry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BridgeRetryLimit(u32);

impl BridgeRetryLimit {
    /// Validates a bounded automatic retry count.
    pub fn new(value: u32) -> Result<Self, BridgeRetryPolicyError> {
        if value > MAXIMUM_AUTOMATIC_RETRIES {
            Err(BridgeRetryPolicyError::InvalidLimit {
                maximum: MAXIMUM_AUTOMATIC_RETRIES,
                actual: value,
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the maximum admitted automatic retry attempts.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) const fn admit(self, completed: u32) -> Option<BridgeRetryAttempt> {
        let next = completed.saturating_add(1);
        if next <= self.0 {
            Some(BridgeRetryAttempt::new(next))
        } else {
            None
        }
    }
}

/// Bounded retry scheduler for query operations.
#[derive(Clone, Copy, Debug)]
pub struct BridgeQueryRetryController {
    limit: BridgeRetryLimit,
    scheduled: u32,
}

impl BridgeQueryRetryController {
    /// Constructs an empty controller with an explicit retry ceiling.
    #[must_use]
    pub const fn new(limit: BridgeRetryLimit) -> Self {
        Self {
            limit,
            scheduled: 0,
        }
    }

    /// Schedules a checked query retry or returns `None` when denied/exhausted.
    pub fn schedule(
        &mut self,
        decision: BridgeQueryRetryDecision,
        retry_class: BridgeRetryClass,
        clock: &impl BridgeClock,
        backoff: &impl BridgeBackoffPolicy,
    ) -> Result<Option<BridgeReconnectSchedule>, BridgeRetryPolicyError> {
        if decision != BridgeQueryRetryDecision::Retry || retry_class == BridgeRetryClass::Never {
            return Ok(None);
        }
        let Some(attempt) = self.limit.admit(self.scheduled) else {
            return Ok(None);
        };
        let delay = backoff.delay(retry_class, attempt);
        let not_before = clock
            .now()
            .checked_add(delay)
            .ok_or(BridgeRetryPolicyError::DeadlineOverflow)?;
        self.scheduled = attempt.get();
        Ok(Some(BridgeReconnectSchedule::new(
            attempt,
            retry_class,
            not_before,
        )))
    }

    /// Resets attempts after one successful query.
    pub fn reset(&mut self) {
        self.scheduled = 0;
    }
}

/// Retry policy validation or scheduling failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeRetryPolicyError {
    /// The retry ceiling exceeded Longhorn's defensive bound.
    InvalidLimit {
        /// Maximum supported automatic retries.
        maximum: u32,
        /// Supplied retry count.
        actual: u32,
    },
    /// Monotonic deadline arithmetic overflowed.
    DeadlineOverflow,
}

impl fmt::Display for BridgeRetryPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { maximum, actual } => {
                write!(formatter, "retry limit is {actual}; maximum is {maximum}")
            }
            Self::DeadlineOverflow => formatter.write_str("retry deadline overflow"),
        }
    }
}

impl Error for BridgeRetryPolicyError {}
