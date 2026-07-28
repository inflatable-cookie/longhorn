use std::{error::Error, fmt, time::Duration};

use longhorn_core::DomainId;

use crate::{
    CoordinationFailureKind, DomainIssue, MutationError, MutationReceipt, PublicationFailure,
};

/// How an accepted stage changed the pending lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageDisposition {
    /// The stage opened an empty lane.
    Opened,
    /// The stage coalesced with an earlier pending generation.
    Coalesced {
        /// Last accepted generation before this stage.
        previous_generation: u64,
    },
}

/// Process-local acknowledgement for one accepted stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageReceipt {
    /// Staged domain.
    pub domain: DomainId,
    /// Monotonic accepted generation.
    pub generation: u64,
    /// Monotonic time when the trailing-edge stage becomes due.
    pub due_at: Duration,
    /// Weight of the complete coalesced pending intent.
    pub pending_weight: usize,
    /// Whether the stage opened or coalesced the lane.
    pub disposition: StageDisposition,
}

/// Rejected stage that leaves existing pending state unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageError {
    /// Consumer coalescing rejected the new intent.
    Coalescing {
        /// Target domain.
        domain: DomainId,
        /// Consumer issue.
        issue: DomainIssue,
    },
    /// The candidate intent exceeded its configured weight limit.
    PendingWeightExceeded {
        /// Target domain.
        domain: DomainId,
        /// Candidate weight.
        attempted: usize,
        /// Configured maximum weight.
        maximum: usize,
    },
    /// The monotonic deadline could not be represented.
    DeadlineOverflow {
        /// Target domain.
        domain: DomainId,
        /// Clock value at staging.
        now: Duration,
        /// Configured trailing-edge delay.
        delay: Duration,
    },
    /// No later generation can be represented.
    GenerationExhausted {
        /// Target domain.
        domain: DomainId,
    },
}

impl fmt::Display for StageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coalescing { domain, issue } => {
                write!(
                    formatter,
                    "cannot coalesce debounce intent for {domain}: {}",
                    issue.message
                )
            }
            Self::PendingWeightExceeded {
                domain,
                attempted,
                maximum,
            } => write!(
                formatter,
                "debounce intent for {domain} weighs {attempted}, above limit {maximum}"
            ),
            Self::DeadlineOverflow { domain, .. } => {
                write!(formatter, "debounce deadline overflow for {domain}")
            }
            Self::GenerationExhausted { domain } => {
                write!(formatter, "debounce generation exhausted for {domain}")
            }
        }
    }
}

impl Error for StageError {}

/// Whether retry is likely useful without changing consumer input or authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryDisposition {
    /// Coordination or pre-publication I/O may succeed later.
    LikelyTransient,
    /// Consumer input, schema, or authority likely needs intervention.
    RequiresIntervention,
}

pub(super) fn retry_disposition(error: &MutationError) -> RetryDisposition {
    match error {
        MutationError::Coordination(failure)
            if !matches!(failure.kind, CoordinationFailureKind::Unsupported) =>
        {
            RetryDisposition::LikelyTransient
        }
        MutationError::Publication(PublicationFailure {
            published: false, ..
        }) => RetryDisposition::LikelyTransient,
        _ => RetryDisposition::RequiresIntervention,
    }
}

/// Terminal result for one accepted generation range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DebounceTerminal {
    /// Applying the intent produced the current encoded value.
    Unchanged {
        /// Target domain.
        domain: DomainId,
        /// Last accepted generation covered by this result.
        generation: u64,
    },
    /// Atomic configuration publication succeeded.
    Published {
        /// Target domain.
        domain: DomainId,
        /// Last accepted generation covered by this result.
        generation: u64,
        /// Underlying coordinated mutation receipt.
        receipt: MutationReceipt,
    },
    /// Publication did not happen and pending intent remains.
    Failed {
        /// Target domain.
        domain: DomainId,
        /// Last accepted generation covered by this result.
        generation: u64,
        /// Typed mutation failure.
        error: MutationError,
        /// Whether an unchanged retry may succeed later.
        retry: RetryDisposition,
    },
    /// Replacement happened but required durability was not established.
    PublishedWithDurabilityFailure {
        /// Target domain.
        domain: DomainId,
        /// Last accepted generation covered by this result.
        generation: u64,
        /// Post-publication failure.
        failure: crate::PublicationFailure,
    },
    /// Unpublished pending intent was explicitly discarded.
    Discarded {
        /// Target domain.
        domain: DomainId,
        /// Discarded pending generation.
        generation: u64,
    },
}

/// Result of a due, forced, or discard operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlushOutcome {
    /// The lane had no pending work.
    NoPending {
        /// Target domain.
        domain: DomainId,
    },
    /// Pending work has not reached its trailing-edge deadline.
    NotDue {
        /// Target domain.
        domain: DomainId,
        /// Pending generation.
        generation: u64,
        /// Current deadline.
        due_at: Duration,
    },
    /// Due polling is suppressed until explicit retry or new input.
    RetryRequired {
        /// Target domain.
        domain: DomainId,
        /// Pending generation.
        generation: u64,
    },
    /// The operation reached a terminal result.
    Terminal(DebounceTerminal),
}

/// Bounded public metadata for pending work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingSnapshot {
    /// Pending generation.
    pub generation: u64,
    /// Current trailing-edge deadline.
    pub due_at: Duration,
    /// Coalesced intent weight.
    pub pending_weight: usize,
    /// Whether due polling is suppressed after failure.
    pub retry_required: bool,
}

/// Bounded observable state for one debounce lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebounceSnapshot {
    /// Target domain.
    pub domain: DomainId,
    /// Pending metadata without the consumer-owned intent.
    pub pending: Option<PendingSnapshot>,
    /// Last terminal result, if any.
    pub last_terminal: Option<DebounceTerminal>,
}

/// Invalid lane insertion into an aggregate flush set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlushSetError {
    /// The set already contains this domain.
    DuplicateDomain {
        /// Duplicate domain.
        domain: DomainId,
    },
    /// The lane borrows another configuration store.
    WrongStore {
        /// Rejected domain.
        domain: DomainId,
    },
}

impl fmt::Display for FlushSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDomain { domain } => {
                write!(formatter, "flush set already contains domain {domain}")
            }
            Self::WrongStore { domain } => {
                write!(
                    formatter,
                    "flush lane for {domain} belongs to another store"
                )
            }
        }
    }
}

impl Error for FlushSetError {}
