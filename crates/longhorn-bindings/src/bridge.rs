use std::error::Error;

use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BRIDGE_PROTOCOL_VERSION,
    BridgeAuthorityCursorDecision, BridgeAuthorityRequirement, BridgeCancellationOutcome,
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandDelivery,
    BridgeCommandEnvelope, BridgeCommandOutcome, BridgeCommandReply, BridgeCommandRetryDecision,
    BridgeConnectionReason, BridgeConnectionState, BridgeConnectionStatus,
    BridgeConnectionTransitionReceipt, BridgeDeduplicationCapacity, BridgeDeduplicationSupport,
    BridgeDiagnostic, BridgeEventEnvelope, BridgeFailure, BridgeFailureMessage, BridgeFailurePhase,
    BridgeHelloRequest, BridgeHostDescriptor, BridgeHostForm, BridgeJobTerminalDecision,
    BridgeJobTerminalEvent, BridgeJobTerminalOutcome, BridgeMonotonicMillis,
    BridgeNegotiationReceipt, BridgeProgressDecision, BridgeProgressEvent, BridgeProtocolVersion,
    BridgeQueryEnvelope, BridgeQueryOutcome, BridgeQueryReply, BridgeQueryRetryDecision,
    BridgeReconnectSchedule, BridgeRequestContext, BridgeRequiredAuthority, BridgeRetryAttempt,
    BridgeRetryClass, BridgeServiceAction, BridgeServiceFailureCode, BridgeServiceGeneration,
    BridgeServiceOutcome, BridgeServiceOwnership, BridgeServiceRequest, BridgeServiceState,
    BridgeServiceTransitionReceipt, BridgeSnapshotDecision, BridgeSnapshotEnvelope,
    BridgeStreamCursor, BridgeStreamDecision, BridgeStreamSequence, BridgeTransitionSequence,
    DomainAuthorityDescriptor, DomainAvailability, DomainCapabilityDescriptor, ExecutionAuthority,
    MAXIMUM_AUTHORITY_DOMAINS, MAXIMUM_CAPABILITIES_PER_DOMAIN, MAXIMUM_CAPABILITY_DOMAINS,
    MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES, MAXIMUM_DIAGNOSTICS, MAXIMUM_FAILURE_MESSAGE_BYTES,
    MAXIMUM_REQUESTED_DOMAINS, MAXIMUM_TRANSPORT_FEATURES, ReadAuthority, WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeCredentialRef, BridgeDiagnosticId, BridgeErrorCode,
    BridgeId, BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, DomainId,
    HostInstanceId, TransportFeatureId,
};
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, apply, exported_declaration, string_union_variants,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/bridge/src/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/bridge/protocol-v1.json";

pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let protocol = render_protocol()?;
    let artifacts = [
        Artifact {
            relative_path: GENERATED_PROTOCOL,
            contents: protocol,
        },
        Artifact {
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render()?,
        },
    ];
    apply("bridge", "generate:bridge", mode, &artifacts)
}

fn render_protocol() -> Result<String, Box<dyn Error>> {
    let host_form = BridgeHostForm::decl();
    let connection_state = BridgeConnectionState::decl();
    let connection_reason = BridgeConnectionReason::decl();
    let authentication = AuthenticationPosture::decl();
    let availability = DomainAvailability::decl();
    let read_authority = ReadAuthority::decl();
    let write_authority = WriteAuthority::decl();
    let execution_authority = ExecutionAuthority::decl();
    let failure_phase = BridgeFailurePhase::decl();
    let retry_class = BridgeRetryClass::decl();
    let command_delivery = BridgeCommandDelivery::decl();
    let command_retry = BridgeCommandRetryDecision::decl();
    let query_retry = BridgeQueryRetryDecision::decl();
    let required_authority = BridgeRequiredAuthority::decl();
    let authority_cursor_decision = BridgeAuthorityCursorDecision::decl();
    let service_ownership = BridgeServiceOwnership::decl();
    let service_state = BridgeServiceState::decl();
    let service_action = BridgeServiceAction::decl();
    let service_failure = BridgeServiceFailureCode::decl();
    let service_outcome = BridgeServiceOutcome::decl();
    let snapshot_decision = BridgeSnapshotDecision::decl();
    let stream_decision = BridgeStreamDecision::decl();
    let progress_decision = BridgeProgressDecision::decl();
    let terminal_decision = BridgeJobTerminalDecision::decl();

    let declarations = [
        BridgeId::decl(),
        BridgeSessionId::decl(),
        HostInstanceId::decl(),
        BridgeCapabilityId::decl(),
        AuthorityScopeId::decl(),
        TransportFeatureId::decl(),
        BridgeDiagnosticId::decl(),
        BridgeRequestId::decl(),
        BridgeIdempotencyKey::decl(),
        BridgeJobId::decl(),
        BridgeErrorCode::decl(),
        BridgeCredentialRef::decl(),
        DomainId::decl(),
        BridgeProtocolVersion::decl(),
        host_form,
        BridgeHostDescriptor::decl(),
        connection_state,
        connection_reason,
        BridgeConnectionStatus::decl(),
        authentication,
        AuthorityEpoch::decl(),
        AuthorityRevision::decl(),
        availability,
        read_authority,
        write_authority,
        execution_authority,
        DomainAuthorityDescriptor::decl(),
        DomainCapabilityDescriptor::decl(),
        BridgeDiagnostic::decl(),
        BridgeHelloRequest::decl(),
        BridgeNegotiationReceipt::decl(),
        BridgeRequestContext::decl(),
        BridgeQueryEnvelope::<String>::decl(),
        BridgeCommandEnvelope::<String>::decl(),
        BridgeCancellationRequest::decl(),
        BridgeFailureMessage::decl(),
        failure_phase,
        retry_class,
        BridgeFailure::<String>::decl(),
        BridgeQueryOutcome::<String, String>::decl(),
        BridgeQueryReply::<String, String>::decl(),
        BridgeCommandOutcome::<String, String>::decl(),
        BridgeCommandReply::<String, String>::decl(),
        BridgeDeduplicationCapacity::decl(),
        BridgeDeduplicationSupport::decl(),
        command_delivery,
        command_retry,
        query_retry,
        BridgeMonotonicMillis::decl(),
        BridgeRetryAttempt::decl(),
        BridgeReconnectSchedule::decl(),
        BridgeTransitionSequence::decl(),
        BridgeConnectionTransitionReceipt::decl(),
        required_authority,
        BridgeAuthorityRequirement::decl(),
        authority_cursor_decision,
        BridgeStreamSequence::decl(),
        BridgeStreamCursor::decl(),
        BridgeSnapshotEnvelope::<String>::decl(),
        BridgeEventEnvelope::<String>::decl(),
        snapshot_decision,
        stream_decision,
        BridgeProgressEvent::<String>::decl(),
        BridgeJobTerminalOutcome::<String, String>::decl(),
        BridgeJobTerminalEvent::<String, String>::decl(),
        BridgeCancellationOutcome::<String>::decl(),
        BridgeCancellationReceipt::<String>::decl(),
        progress_decision,
        terminal_decision,
        service_ownership,
        service_state,
        service_action,
        service_failure,
        service_outcome,
        BridgeServiceRequest::decl(),
        BridgeServiceGeneration::decl(),
        BridgeServiceTransitionReceipt::decl(),
    ]
    .map(exported_declaration);

    Ok(format!(
        "// @generated by `effigy generate:bridge`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const BRIDGE_PROTOCOL_VERSION = {BRIDGE_PROTOCOL_VERSION} as const;\n\
         // Wire-visible bounds. The Rust constants are the authority; a\n\
         // hand-copied literal in a validator is drift waiting to happen.\n\
         export const BRIDGE_MAXIMUM_REQUESTED_DOMAINS = {MAXIMUM_REQUESTED_DOMAINS} as const;\n\
         export const BRIDGE_MAXIMUM_CAPABILITY_DOMAINS = {MAXIMUM_CAPABILITY_DOMAINS} as const;\n\
         export const BRIDGE_MAXIMUM_AUTHORITY_DOMAINS = {MAXIMUM_AUTHORITY_DOMAINS} as const;\n\
         export const BRIDGE_MAXIMUM_CAPABILITIES_PER_DOMAIN = {MAXIMUM_CAPABILITIES_PER_DOMAIN} as const;\n\
         export const BRIDGE_MAXIMUM_TRANSPORT_FEATURES = {MAXIMUM_TRANSPORT_FEATURES} as const;\n\
         export const BRIDGE_MAXIMUM_DIAGNOSTICS = {MAXIMUM_DIAGNOSTICS} as const;\n\
         export const BRIDGE_MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES = {MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES} as const;\n\
         export const BRIDGE_MAXIMUM_FAILURE_MESSAGE_BYTES = {MAXIMUM_FAILURE_MESSAGE_BYTES} as const;\n\
         export const BRIDGE_HOST_FORMS = {} as const;\n\
         export const BRIDGE_CONNECTION_STATES = {} as const;\n\
         export const BRIDGE_CONNECTION_REASONS = {} as const;\n\
         export const BRIDGE_AUTHENTICATION_POSTURES = {} as const;\n\
         export const BRIDGE_DOMAIN_AVAILABILITIES = {} as const;\n\
         export const BRIDGE_READ_AUTHORITIES = {} as const;\n\
         export const BRIDGE_WRITE_AUTHORITIES = {} as const;\n\
         export const BRIDGE_EXECUTION_AUTHORITIES = {} as const;\n\
         export const BRIDGE_FAILURE_PHASES = {} as const;\n\
         export const BRIDGE_RETRY_CLASSES = {} as const;\n\
         export const BRIDGE_COMMAND_DELIVERIES = {} as const;\n\
         export const BRIDGE_COMMAND_RETRY_DECISIONS = {} as const;\n\
         export const BRIDGE_QUERY_RETRY_DECISIONS = {} as const;\n\
         export const BRIDGE_REQUIRED_AUTHORITIES = {} as const;\n\
         export const BRIDGE_AUTHORITY_CURSOR_DECISIONS = {} as const;\n\
         export const BRIDGE_SERVICE_OWNERSHIPS = {} as const;\n\
         export const BRIDGE_SERVICE_STATES = {} as const;\n\
         export const BRIDGE_SERVICE_ACTIONS = {} as const;\n\
         export const BRIDGE_SERVICE_FAILURE_CODES = {} as const;\n\
         export const BRIDGE_SNAPSHOT_DECISIONS = {} as const;\n\
         export const BRIDGE_STREAM_DECISIONS = {} as const;\n\
         export const BRIDGE_PROGRESS_DECISIONS = {} as const;\n\
         export const BRIDGE_TERMINAL_DECISIONS = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&string_union_variants(&BridgeHostForm::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeConnectionState::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeConnectionReason::decl())?)?,
        serde_json::to_string(&string_union_variants(&AuthenticationPosture::decl())?)?,
        serde_json::to_string(&string_union_variants(&DomainAvailability::decl())?)?,
        serde_json::to_string(&string_union_variants(&ReadAuthority::decl())?)?,
        serde_json::to_string(&string_union_variants(&WriteAuthority::decl())?)?,
        serde_json::to_string(&string_union_variants(&ExecutionAuthority::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeFailurePhase::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeRetryClass::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeCommandDelivery::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeCommandRetryDecision::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeQueryRetryDecision::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeRequiredAuthority::decl())?)?,
        serde_json::to_string(&string_union_variants(
            &BridgeAuthorityCursorDecision::decl()
        )?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceOwnership::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceState::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceAction::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceFailureCode::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeSnapshotDecision::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeStreamDecision::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeProgressDecision::decl())?)?,
        serde_json::to_string(&string_union_variants(&BridgeJobTerminalDecision::decl())?)?,
        declarations.join("\n\n")
    ))
}
