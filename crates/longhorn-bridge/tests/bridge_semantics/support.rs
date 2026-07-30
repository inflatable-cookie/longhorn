use longhorn_bridge::{
    AuthorityEpoch, BridgeFailure, BridgeFailureMessage, BridgeFailurePhase, BridgeRequestContext,
    BridgeRetryClass, BridgeStreamCursor, BridgeStreamSequence,
};
use longhorn_core::{
    BridgeErrorCode, BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, DomainId,
};
use serde::{Deserialize, Serialize};

pub fn request_id(value: &str) -> BridgeRequestId {
    BridgeRequestId::new(value).unwrap()
}

pub fn session_id(value: &str) -> BridgeSessionId {
    BridgeSessionId::new(value).unwrap()
}

pub fn job_id(value: &str) -> BridgeJobId {
    BridgeJobId::new(value).unwrap()
}

pub fn idempotency_key(value: &str) -> BridgeIdempotencyKey {
    BridgeIdempotencyKey::new(value).unwrap()
}

pub fn domain_id(value: &str) -> DomainId {
    DomainId::new(value).unwrap()
}

pub fn context(request: &str) -> BridgeRequestContext {
    BridgeRequestContext::new(
        request_id(request),
        session_id("session:current"),
        domain_id("example.workspace"),
    )
}

pub fn cursor(session: &str, epoch: u64, sequence: u64) -> BridgeStreamCursor {
    BridgeStreamCursor::new(
        session_id(session),
        domain_id("example.workspace"),
        AuthorityEpoch::new(epoch).unwrap(),
        BridgeStreamSequence::new(sequence),
    )
}

pub fn failure(
    retry_class: BridgeRetryClass,
    phase: BridgeFailurePhase,
) -> BridgeFailure<FailureDetail> {
    BridgeFailure::new(
        BridgeErrorCode::new("workspace:unavailable").unwrap(),
        BridgeFailureMessage::new("workspace authority is unavailable").unwrap(),
        retry_class,
        phase,
        Some(FailureDetail {
            source: "fixture".into(),
        }),
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FailureDetail {
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryPayload {
    pub include_archived: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandPayload {
    pub delta: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SuccessPayload {
    pub value: i64,
}
