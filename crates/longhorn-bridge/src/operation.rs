use longhorn_core::{
    BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, DomainId,
};
use serde::{Deserialize, Serialize};

use crate::{AuthorityEpoch, AuthorityRevision};

/// Shared correlation and authority-routing metadata for one bridge request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeRequestContext {
    request_id: BridgeRequestId,
    session_id: BridgeSessionId,
    domain_id: DomainId,
}

impl BridgeRequestContext {
    /// Constructs request metadata without assigning replay authority.
    #[must_use]
    pub const fn new(
        request_id: BridgeRequestId,
        session_id: BridgeSessionId,
        domain_id: DomainId,
    ) -> Self {
        Self {
            request_id,
            session_id,
            domain_id,
        }
    }

    /// Returns the correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns the negotiated session identity.
    #[must_use]
    pub const fn session_id(&self) -> &BridgeSessionId {
        &self.session_id
    }

    /// Returns the domain that owns the concrete operation and payload.
    #[must_use]
    pub const fn domain_id(&self) -> &DomainId {
        &self.domain_id
    }
}

/// Domain-generic query envelope.
///
/// The concrete query operation and payload schema remain encoded by `P` and
/// the owning domain route.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeQueryEnvelope<P> {
    context: BridgeRequestContext,
    payload: P,
}

impl<P> BridgeQueryEnvelope<P> {
    /// Wraps a consumer-owned query payload.
    #[must_use]
    pub const fn new(context: BridgeRequestContext, payload: P) -> Self {
        Self { context, payload }
    }

    /// Returns shared request metadata.
    #[must_use]
    pub const fn context(&self) -> &BridgeRequestContext {
        &self.context
    }

    /// Returns the consumer-owned query payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the envelope and returns its consumer-owned payload.
    #[must_use]
    pub fn into_payload(self) -> P {
        self.payload
    }
}

/// Domain-generic authoritative command envelope.
///
/// Request correlation and optional durable idempotency are separate fields.
/// Possessing a request id never grants replay permission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeCommandEnvelope<P> {
    context: BridgeRequestContext,
    authority_epoch: AuthorityEpoch,
    expected_revision: Option<AuthorityRevision>,
    idempotency_key: Option<BridgeIdempotencyKey>,
    payload: P,
}

impl<P> BridgeCommandEnvelope<P> {
    /// Wraps a consumer-owned command payload with checked authority metadata.
    #[must_use]
    pub const fn new(
        context: BridgeRequestContext,
        authority_epoch: AuthorityEpoch,
        expected_revision: Option<AuthorityRevision>,
        idempotency_key: Option<BridgeIdempotencyKey>,
        payload: P,
    ) -> Self {
        Self {
            context,
            authority_epoch,
            expected_revision,
            idempotency_key,
            payload,
        }
    }

    /// Returns shared request metadata.
    #[must_use]
    pub const fn context(&self) -> &BridgeRequestContext {
        &self.context
    }

    /// Returns the authority tenure expected by the caller.
    #[must_use]
    pub const fn authority_epoch(&self) -> AuthorityEpoch {
        self.authority_epoch
    }

    /// Returns optional authoritative revision precondition.
    #[must_use]
    pub const fn expected_revision(&self) -> Option<AuthorityRevision> {
        self.expected_revision
    }

    /// Returns durable replay identity when the caller supplied one.
    #[must_use]
    pub const fn idempotency_key(&self) -> Option<&BridgeIdempotencyKey> {
        self.idempotency_key.as_ref()
    }

    /// Returns the consumer-owned command payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the envelope and returns its consumer-owned payload.
    #[must_use]
    pub fn into_payload(self) -> P {
        self.payload
    }
}

/// Request to cancel one optional request-correlated job.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeCancellationRequest {
    context: BridgeRequestContext,
    target_request_id: BridgeRequestId,
    job_id: BridgeJobId,
}

impl BridgeCancellationRequest {
    /// Constructs a cancellation request without claiming immediate termination.
    #[must_use]
    pub const fn new(
        context: BridgeRequestContext,
        target_request_id: BridgeRequestId,
        job_id: BridgeJobId,
    ) -> Self {
        Self {
            context,
            target_request_id,
            job_id,
        }
    }

    /// Returns metadata for the cancellation request itself.
    #[must_use]
    pub const fn context(&self) -> &BridgeRequestContext {
        &self.context
    }

    /// Returns the initiating request being cancelled.
    #[must_use]
    pub const fn target_request_id(&self) -> &BridgeRequestId {
        &self.target_request_id
    }

    /// Returns the targeted optional job.
    #[must_use]
    pub const fn job_id(&self) -> &BridgeJobId {
        &self.job_id
    }
}
