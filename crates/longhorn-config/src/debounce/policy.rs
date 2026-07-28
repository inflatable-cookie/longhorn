use std::{
    error::Error,
    fmt,
    time::{Duration, Instant},
};

use crate::{ConfigDomain, DomainIssue, MutationOptions};

/// Monotonic time source used by debounce scheduling.
pub trait DebounceClock {
    /// Returns elapsed monotonic time from this clock's private origin.
    fn now(&self) -> Duration;
}

/// Standard monotonic clock backed by [`Instant`].
#[derive(Clone, Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// Starts a clock at the current monotonic instant.
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl DebounceClock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Timing, memory, and publication policy for one debounce lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebouncePolicy {
    pub(super) delay: Duration,
    pub(super) max_pending_weight: usize,
    pub(super) mutation: MutationOptions,
}

impl DebouncePolicy {
    /// Constructs a policy with a non-zero pending-weight limit.
    pub fn new(
        delay: Duration,
        max_pending_weight: usize,
        mutation: MutationOptions,
    ) -> Result<Self, DebouncePolicyError> {
        if max_pending_weight == 0 {
            return Err(DebouncePolicyError::ZeroPendingWeight);
        }
        Ok(Self {
            delay,
            max_pending_weight,
            mutation,
        })
    }

    /// Returns the trailing-edge delay.
    #[must_use]
    pub const fn delay(&self) -> Duration {
        self.delay
    }

    /// Returns the maximum accepted coalesced intent weight.
    #[must_use]
    pub const fn max_pending_weight(&self) -> usize {
        self.max_pending_weight
    }

    /// Returns the coordinated mutation options used at flush.
    #[must_use]
    pub const fn mutation_options(&self) -> MutationOptions {
        self.mutation
    }
}

/// Invalid debounce policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebouncePolicyError {
    /// A zero limit cannot admit any pending intent.
    ZeroPendingWeight,
}

impl fmt::Display for DebouncePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroPendingWeight => {
                formatter.write_str("maximum pending intent weight must be non-zero")
            }
        }
    }
}

impl Error for DebouncePolicyError {}

/// Consumer-owned intent behavior for one configuration domain.
pub trait DebounceStrategy<D: ConfigDomain> {
    /// Owned staged change representation.
    type Intent;

    /// Coalesces two accepted intents while preserving their application order.
    fn coalesce(
        &self,
        previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue>;

    /// Applies one coalesced intent to a freshly loaded domain value.
    fn apply(&self, intent: &Self::Intent, value: &mut D::Value) -> Result<(), DomainIssue>;

    /// Returns the deterministic memory/cost weight of one intent.
    fn pending_weight(&self, intent: &Self::Intent) -> usize;
}
