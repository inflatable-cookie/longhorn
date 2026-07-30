//! Pure bridge identity, negotiation, capability, and authority protocol.
//!
//! The crate describes a connection to an authoritative host without owning a
//! transport, process lifecycle, domain payload, or renderer integration.

mod authority;
mod connection;
mod error;
mod failure;
mod identity;
mod job;
mod lifecycle;
mod negotiation;
mod operation;
mod ordering;
mod replay;
#[cfg(feature = "supervision")]
mod supervision;

pub use authority::{
    AuthorityEpoch, AuthorityRevision, DomainAuthorityDescriptor, DomainAvailability,
    ExecutionAuthority, ReadAuthority, WriteAuthority,
};
pub use connection::{
    AuthenticationPosture, BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus,
    BridgeHostDescriptor, BridgeHostForm,
};
pub use error::{BridgeNegotiationError, BridgeNegotiationErrorCode};
pub use failure::{
    BridgeCommandOutcome, BridgeCommandReply, BridgeFailure, BridgeFailureMessage,
    BridgeFailureMessageError, BridgeFailurePhase, BridgeQueryOutcome, BridgeQueryReply,
    BridgeRetryClass, MAXIMUM_FAILURE_MESSAGE_BYTES,
};
pub use identity::{BRIDGE_PROTOCOL_VERSION, BridgeProtocolVersion};
pub use job::{
    BridgeCancellationOutcome, BridgeCancellationReceipt, BridgeJobTerminalDecision,
    BridgeJobTerminalEvent, BridgeJobTerminalOutcome, BridgeJobTracker, BridgeProgressDecision,
    BridgeProgressEvent,
};
pub use lifecycle::{
    BridgeAuthorityCursorDecision, BridgeAuthorityRequirement, BridgeBackoffPolicy, BridgeClock,
    BridgeConnectionMachine, BridgeConnectionTransitionReceipt, BridgeDelayMillis,
    BridgeLifecycleError, BridgeLifecycleErrorCode, BridgeMonotonicMillis,
    BridgeQueryRetryController, BridgeReconnectSchedule, BridgeRequiredAuthority,
    BridgeRetryAttempt, BridgeRetryLimit, BridgeRetryPolicyError, BridgeTransitionSequence,
};
pub use negotiation::{
    BridgeDiagnostic, BridgeHelloRequest, BridgeNegotiationReceipt, DomainCapabilityDescriptor,
    MAXIMUM_AUTHORITY_DOMAINS, MAXIMUM_CAPABILITIES_PER_DOMAIN, MAXIMUM_CAPABILITY_DOMAINS,
    MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES, MAXIMUM_DIAGNOSTICS, MAXIMUM_REQUESTED_DOMAINS,
    MAXIMUM_TRANSPORT_FEATURES,
};
pub use operation::{
    BridgeCancellationRequest, BridgeCommandEnvelope, BridgeQueryEnvelope, BridgeRequestContext,
};
pub use ordering::{
    BridgeEventEnvelope, BridgeSnapshotDecision, BridgeSnapshotEnvelope, BridgeStreamCursor,
    BridgeStreamDecision, BridgeStreamSequence, BridgeStreamTracker,
};
pub use replay::{
    BridgeCommandDelivery, BridgeCommandRetryDecision, BridgeDeduplicationCapacity,
    BridgeDeduplicationError, BridgeDeduplicationLedger, BridgeDeduplicationSupport,
    BridgeQueryRetryDecision, BridgeReplayRecord, MAXIMUM_DEDUPLICATION_ENTRIES,
};
#[cfg(feature = "supervision")]
pub use supervision::{
    BridgeServiceAction, BridgeServiceFailureCode, BridgeServiceGeneration, BridgeServiceMachine,
    BridgeServiceOutcome, BridgeServiceOwnership, BridgeServiceRequest, BridgeServiceState,
    BridgeServiceSupervisor, BridgeServiceTransitionReceipt, BridgeSupervisionError,
};
