use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::{BridgeIdempotencyKey, BridgeRequestId};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{BridgeCommandEnvelope, BridgeQueryEnvelope, BridgeRetryClass};

/// Defensive ceiling for one in-memory deduplication evidence ledger.
pub const MAXIMUM_DEDUPLICATION_ENTRIES: u32 = 65_536;

/// Validated finite capacity for replay evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeDeduplicationCapacity(u32);

impl BridgeDeduplicationCapacity {
    /// Validates a nonzero finite ledger capacity.
    pub fn new(value: u32) -> Result<Self, BridgeDeduplicationError> {
        if value == 0 || value > MAXIMUM_DEDUPLICATION_ENTRIES {
            Err(BridgeDeduplicationError::InvalidCapacity {
                maximum: MAXIMUM_DEDUPLICATION_ENTRIES,
                actual: value,
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized entry capacity.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for BridgeDeduplicationCapacity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Deduplication support advertised by the current authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeDeduplicationSupport {
    /// The authority provides no command replay guarantee.
    Unsupported,
    /// The authority retains a bounded no-eviction ledger for this session.
    Finite(BridgeDeduplicationCapacity),
}

/// Whether transport failure left command delivery uncertain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeCommandDelivery {
    /// The adapter can prove the command was not dispatched.
    NotDispatched,
    /// The adapter cannot prove whether the authority applied the command.
    Uncertain,
}

/// Safe disposition after command transport failure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeCommandRetryDecision {
    /// Do not replay this request automatically.
    DoNotRetry,
    /// Replay the same command and idempotency key under authority deduplication.
    RetrySameRequest,
    /// Return an explicit indeterminate terminal.
    Indeterminate,
}

/// Adapter-policy decision for a failed query.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeQueryRetryDecision {
    /// Do not retry automatically.
    DoNotRetry,
    /// Adapter policy may retry the query.
    Retry,
}

impl<P> BridgeCommandEnvelope<P> {
    /// Classifies transport failure without treating correlation as replay permission.
    #[must_use]
    pub fn classify_transport_failure(
        &self,
        delivery: BridgeCommandDelivery,
        retry_class: BridgeRetryClass,
        deduplication: BridgeDeduplicationSupport,
    ) -> BridgeCommandRetryDecision {
        match delivery {
            BridgeCommandDelivery::NotDispatched => BridgeCommandRetryDecision::DoNotRetry,
            BridgeCommandDelivery::Uncertain
                if retry_class != BridgeRetryClass::Never
                    && self.idempotency_key().is_some()
                    && matches!(deduplication, BridgeDeduplicationSupport::Finite(_)) =>
            {
                BridgeCommandRetryDecision::RetrySameRequest
            }
            BridgeCommandDelivery::Uncertain => BridgeCommandRetryDecision::Indeterminate,
        }
    }
}

impl<P> BridgeQueryEnvelope<P> {
    /// Applies explicit adapter retry policy to a query failure.
    #[must_use]
    pub const fn classify_retry(
        &self,
        retry_class: BridgeRetryClass,
        adapter_allows_retry: bool,
    ) -> BridgeQueryRetryDecision {
        let _ = self;
        if adapter_allows_retry && !matches!(retry_class, BridgeRetryClass::Never) {
            BridgeQueryRetryDecision::Retry
        } else {
            BridgeQueryRetryDecision::DoNotRetry
        }
    }
}

/// Retained result proving that one idempotency key already executed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeReplayRecord<T> {
    original_request_id: BridgeRequestId,
    outcome: T,
}

impl<T> BridgeReplayRecord<T> {
    /// Returns the first request recorded for this durable key.
    #[must_use]
    pub const fn original_request_id(&self) -> &BridgeRequestId {
        &self.original_request_id
    }

    /// Returns the retained typed outcome.
    #[must_use]
    pub const fn outcome(&self) -> &T {
        &self.outcome
    }
}

/// Finite no-eviction evidence ledger for one authority session.
///
/// A full ledger rejects new records. It never evicts a key and then treats
/// that key as fresh within the same ledger lifetime.
#[derive(Clone, Debug)]
pub struct BridgeDeduplicationLedger<T> {
    capacity: BridgeDeduplicationCapacity,
    records: BTreeMap<BridgeIdempotencyKey, BridgeReplayRecord<T>>,
}

impl<T> BridgeDeduplicationLedger<T> {
    /// Constructs an empty bounded evidence ledger.
    #[must_use]
    pub fn new(capacity: BridgeDeduplicationCapacity) -> Self {
        Self {
            capacity,
            records: BTreeMap::new(),
        }
    }

    /// Returns the finite support this ledger can advertise.
    #[must_use]
    pub const fn support(&self) -> BridgeDeduplicationSupport {
        BridgeDeduplicationSupport::Finite(self.capacity)
    }

    /// Returns retained record count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the ledger contains no evidence.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Looks up retained replay evidence without mutation.
    #[must_use]
    pub fn lookup(&self, key: &BridgeIdempotencyKey) -> Option<&BridgeReplayRecord<T>> {
        self.records.get(key)
    }

    /// Records a first result or rejects duplicate/full admission.
    pub fn record(
        &mut self,
        key: BridgeIdempotencyKey,
        original_request_id: BridgeRequestId,
        outcome: T,
    ) -> Result<(), BridgeDeduplicationError> {
        if self.records.contains_key(&key) {
            return Err(BridgeDeduplicationError::DuplicateKey);
        }
        if self.records.len() >= self.capacity.get() as usize {
            return Err(BridgeDeduplicationError::Full);
        }
        self.records.insert(
            key,
            BridgeReplayRecord {
                original_request_id,
                outcome,
            },
        );
        Ok(())
    }
}

/// Finite replay-evidence failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeDeduplicationError {
    /// Capacity was zero or exceeded the defensive ceiling.
    InvalidCapacity {
        /// Maximum supported entries.
        maximum: u32,
        /// Supplied entry count.
        actual: u32,
    },
    /// The durable key already has retained evidence.
    DuplicateKey,
    /// The no-eviction ledger has no space for a new key.
    Full,
}

impl fmt::Display for BridgeDeduplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity { maximum, actual } => {
                write!(
                    formatter,
                    "deduplication capacity is {actual}; expected 1..={maximum}"
                )
            }
            Self::DuplicateKey => formatter.write_str("deduplication key is already recorded"),
            Self::Full => formatter.write_str("deduplication evidence ledger is full"),
        }
    }
}

impl Error for BridgeDeduplicationError {}
