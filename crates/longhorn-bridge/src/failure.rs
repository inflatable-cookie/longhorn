use std::{error::Error, fmt};

use longhorn_core::{BridgeErrorCode, BridgeRequestId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::AuthorityRevision;

/// Maximum UTF-8 bytes in one bridge failure message.
pub const MAXIMUM_FAILURE_MESSAGE_BYTES: usize = 4_096;

/// Bounded human-readable bridge failure message.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct BridgeFailureMessage(String);

impl BridgeFailureMessage {
    /// Validates and constructs a nonempty failure message.
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeFailureMessageError> {
        let value = value.into();
        if value.is_empty() {
            return Err(BridgeFailureMessageError::Empty);
        }
        if value.len() > MAXIMUM_FAILURE_MESSAGE_BYTES {
            return Err(BridgeFailureMessageError::TooLong {
                maximum: MAXIMUM_FAILURE_MESSAGE_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the serialized message.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BridgeFailureMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for BridgeFailureMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BridgeFailureMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Failure-message validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeFailureMessageError {
    /// The message was empty.
    Empty,
    /// The message exceeded the defensive wire limit.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
}

impl fmt::Display for BridgeFailureMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("bridge failure message cannot be empty"),
            Self::TooLong { maximum, actual } => {
                write!(
                    formatter,
                    "bridge failure message is {actual} bytes; maximum is {maximum}"
                )
            }
        }
    }
}

impl Error for BridgeFailureMessageError {}

/// Phase in which a bridge operation failed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeFailurePhase {
    /// Request structure or preconditions were rejected before admission.
    Admission,
    /// Capability or domain authority checks failed.
    Authorization,
    /// The adapter could not dispatch the operation to its authority.
    Dispatch,
    /// The domain authority failed while executing accepted work.
    Execution,
    /// Authoritative publication failed or became uncertain.
    Publication,
    /// A reply could not be returned after execution.
    Response,
    /// The transport failed outside a known operation phase.
    Transport,
}

/// Adapter-facing retry timing classification.
///
/// This classification never grants replay. Command replay additionally
/// requires a durable idempotency key and advertised deduplication support.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeRetryClass {
    /// The operation must not be retried automatically.
    Never,
    /// Adapter policy may retry after bounded backoff.
    AfterBackoff,
    /// Adapter policy may retry after a fresh connection.
    AfterReconnect,
    /// Adapter policy may retry after refreshing domain authority.
    AfterAuthorityRefresh,
}

/// Stable coded bridge or domain failure with optional typed detail.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeFailure<D> {
    code: BridgeErrorCode,
    message: BridgeFailureMessage,
    retry_class: BridgeRetryClass,
    phase: BridgeFailurePhase,
    details: Option<D>,
}

impl<D> BridgeFailure<D> {
    /// Constructs a coded failure without interpreting domain-owned detail.
    #[must_use]
    pub const fn new(
        code: BridgeErrorCode,
        message: BridgeFailureMessage,
        retry_class: BridgeRetryClass,
        phase: BridgeFailurePhase,
        details: Option<D>,
    ) -> Self {
        Self {
            code,
            message,
            retry_class,
            phase,
            details,
        }
    }

    /// Returns the stable failure code.
    #[must_use]
    pub const fn code(&self) -> &BridgeErrorCode {
        &self.code
    }

    /// Returns the bounded operator-facing message.
    #[must_use]
    pub const fn message(&self) -> &BridgeFailureMessage {
        &self.message
    }

    /// Returns retry timing without granting command replay.
    #[must_use]
    pub const fn retry_class(&self) -> BridgeRetryClass {
        self.retry_class
    }

    /// Returns the failure phase.
    #[must_use]
    pub const fn phase(&self) -> BridgeFailurePhase {
        self.phase
    }

    /// Returns optional domain-owned structured detail.
    #[must_use]
    pub const fn details(&self) -> Option<&D> {
        self.details.as_ref()
    }
}

/// Typed query terminal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeQueryOutcome<S, D> {
    /// The authority returned a checked consumer-owned value.
    Success(S),
    /// The authority returned a stable coded rejection.
    Rejected(BridgeFailure<D>),
}

/// Query reply echoing the initiating request identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeQueryReply<S, D> {
    request_id: BridgeRequestId,
    outcome: BridgeQueryOutcome<S, D>,
}

impl<S, D> BridgeQueryReply<S, D> {
    /// Constructs a request-correlated query reply.
    #[must_use]
    pub const fn new(request_id: BridgeRequestId, outcome: BridgeQueryOutcome<S, D>) -> Self {
        Self {
            request_id,
            outcome,
        }
    }

    /// Returns the initiating request identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns the typed terminal outcome.
    #[must_use]
    pub const fn outcome(&self) -> &BridgeQueryOutcome<S, D> {
        &self.outcome
    }
}

/// Typed authoritative command terminal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeCommandOutcome<S, D> {
    /// The authority applied the command and returned checked evidence.
    Applied(S),
    /// The authority definitively rejected the command.
    Rejected(BridgeFailure<D>),
    /// The authority cannot prove whether the command applied.
    Indeterminate(BridgeFailure<D>),
}

/// Command reply echoing request identity and optional authoritative revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeCommandReply<S, D> {
    request_id: BridgeRequestId,
    authoritative_revision: Option<AuthorityRevision>,
    outcome: BridgeCommandOutcome<S, D>,
}

impl<S, D> BridgeCommandReply<S, D> {
    /// Constructs a request-correlated authoritative command reply.
    #[must_use]
    pub const fn new(
        request_id: BridgeRequestId,
        authoritative_revision: Option<AuthorityRevision>,
        outcome: BridgeCommandOutcome<S, D>,
    ) -> Self {
        Self {
            request_id,
            authoritative_revision,
            outcome,
        }
    }

    /// Returns the initiating request identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns optional post-command authoritative revision evidence.
    #[must_use]
    pub const fn authoritative_revision(&self) -> Option<AuthorityRevision> {
        self.authoritative_revision
    }

    /// Returns the typed terminal outcome.
    #[must_use]
    pub const fn outcome(&self) -> &BridgeCommandOutcome<S, D> {
        &self.outcome
    }
}
