use std::{error::Error, fmt};

use longhorn_core::{
    NotificationId, NotificationLedgerRevision, NotificationProducerToken,
    NotificationReplacementKey, NotificationSourceId,
};

use crate::{NotificationAuthorityCursor, NotificationPageSizeError};

/// Rejected notification ledger command or projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationLedgerError {
    /// The command names another authority id or live epoch.
    WrongAuthority {
        /// Current authority.
        expected: NotificationAuthorityCursor,
        /// Supplied authority.
        actual: NotificationAuthorityCursor,
    },
    /// The command was based on stale or impossible ledger state.
    StaleRevision {
        /// Current revision.
        expected: NotificationLedgerRevision,
        /// Supplied revision.
        actual: NotificationLedgerRevision,
    },
    /// Stable record identity is already retained.
    DuplicateNotification {
        /// Colliding identity.
        notification_id: NotificationId,
    },
    /// The record is not retained.
    NotificationNotFound {
        /// Missing identity.
        notification_id: NotificationId,
    },
    /// A source and replacement key must identify at most one record.
    DuplicateReplacementKey {
        /// Source identity.
        source_id: NotificationSourceId,
        /// Colliding replacement key.
        replacement_key: NotificationReplacementKey,
    },
    /// Explicit replacement requires a replacement key in the draft.
    MissingReplacementKey,
    /// No retained record matches the explicit source and replacement key.
    ReplacementTargetNotFound {
        /// Source identity.
        source_id: NotificationSourceId,
        /// Requested replacement key.
        replacement_key: NotificationReplacementKey,
    },
    /// A producer token already belongs to another retained record.
    DuplicateProducerToken {
        /// Colliding producer token.
        producer_token: NotificationProducerToken,
    },
    /// Idempotent publication requires a producer token.
    MissingProducerToken,
    /// The retained record was already marked seen.
    AlreadySeen {
        /// Record identity.
        notification_id: NotificationId,
    },
    /// An explicit clear request repeated one identity.
    DuplicateClearTarget {
        /// Repeated identity.
        notification_id: NotificationId,
    },
    /// An explicit clear request exceeded the finite ledger identity ceiling.
    TooManyClearTargets {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied target count.
        actual: usize,
    },
    /// An explicit clear target was not retained.
    ClearTargetNotFound {
        /// Missing identity.
        notification_id: NotificationId,
    },
    /// Too many actions were attached to one draft.
    TooManyActions {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied count.
        actual: usize,
    },
    /// Protected or newly admitted data prevents satisfaction of finite limits.
    RetentionUnsatisfied {
        /// Requested count ceiling.
        maximum_count: usize,
        /// Count still retained.
        retained_count: usize,
        /// Requested encoded-weight ceiling.
        maximum_encoded_weight: u64,
        /// Weight still retained.
        retained_encoded_weight: u64,
    },
    /// Canonical encoded metadata weight overflowed.
    EncodedWeightOverflow,
    /// Ledger revision cannot advance without wrapping.
    RevisionOverflow,
    /// Insertion sequence cannot advance without wrapping.
    SequenceOverflow,
    /// Cumulative prune count cannot advance without wrapping.
    PrunedCountOverflow,
    /// The requested projection page was not bounded.
    InvalidPageSize(NotificationPageSizeError),
}

impl fmt::Display for NotificationLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongAuthority { .. } => formatter.write_str("notification authority mismatch"),
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "notification ledger revision mismatch: expected {}, got {}",
                expected.get(),
                actual.get()
            ),
            Self::DuplicateNotification { notification_id } => {
                write!(
                    formatter,
                    "notification {notification_id} is already retained"
                )
            }
            Self::NotificationNotFound { notification_id } => {
                write!(formatter, "notification {notification_id} is not retained")
            }
            Self::DuplicateReplacementKey {
                source_id,
                replacement_key,
            } => write!(
                formatter,
                "notification replacement key {source_id}/{replacement_key} is already retained"
            ),
            Self::MissingReplacementKey => {
                formatter.write_str("explicit replacement requires a replacement key")
            }
            Self::ReplacementTargetNotFound {
                source_id,
                replacement_key,
            } => write!(
                formatter,
                "notification replacement target {source_id}/{replacement_key} is not retained"
            ),
            Self::DuplicateProducerToken { producer_token } => write!(
                formatter,
                "notification producer token {producer_token} is already retained"
            ),
            Self::MissingProducerToken => {
                formatter.write_str("idempotent publication requires a producer token")
            }
            Self::AlreadySeen { notification_id } => {
                write!(formatter, "notification {notification_id} is already seen")
            }
            Self::DuplicateClearTarget { notification_id } => {
                write!(
                    formatter,
                    "clear target repeats notification {notification_id}"
                )
            }
            Self::TooManyClearTargets { maximum, actual } => write!(
                formatter,
                "clear target count {actual} exceeds maximum {maximum}"
            ),
            Self::ClearTargetNotFound { notification_id } => {
                write!(formatter, "clear target {notification_id} is not retained")
            }
            Self::TooManyActions { maximum, actual } => write!(
                formatter,
                "notification action count {actual} exceeds maximum {maximum}"
            ),
            Self::RetentionUnsatisfied {
                maximum_count,
                retained_count,
                maximum_encoded_weight,
                retained_encoded_weight,
            } => write!(
                formatter,
                "retention cannot satisfy count {retained_count}/{maximum_count} and weight {retained_encoded_weight}/{maximum_encoded_weight}"
            ),
            Self::EncodedWeightOverflow => {
                formatter.write_str("notification encoded weight overflow")
            }
            Self::RevisionOverflow => formatter.write_str("notification ledger revision overflow"),
            Self::SequenceOverflow => formatter.write_str("notification sequence overflow"),
            Self::PrunedCountOverflow => formatter.write_str("notification prune count overflow"),
            Self::InvalidPageSize(error) => error.fmt(formatter),
        }
    }
}

impl Error for NotificationLedgerError {}

impl From<NotificationPageSizeError> for NotificationLedgerError {
    fn from(value: NotificationPageSizeError) -> Self {
        Self::InvalidPageSize(value)
    }
}
