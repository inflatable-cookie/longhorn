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
    MAXIMUM_DEDUPLICATION_ENTRIES, MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES, MAXIMUM_DIAGNOSTICS,
    MAXIMUM_FAILURE_MESSAGE_BYTES, MAXIMUM_REQUESTED_DOMAINS, MAXIMUM_TRANSPORT_FEATURES,
    ReadAuthority, WriteAuthority,
};
use longhorn_core::{
    AuthorityScopeId, BridgeCapabilityId, BridgeCredentialRef, BridgeDiagnosticId, BridgeErrorCode,
    BridgeId, BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, DomainId,
    HostInstanceId, MAX_OPAQUE_ID_BYTES, TransportFeatureId,
};
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, apply, exported_declaration, field_map, string_union_variants,
    variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/bridge/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/bridge/protocol-v1.json";
const GENERATED_FIELDS: &str = "packages/longhorn/src/bridge/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str = "packages/longhorn/src/bridge/generated/variant-fields.ts";

struct RenderedProtocol {
    contents: String,
    fields: String,
    variant_fields: String,
}

pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let protocol = render_protocol()?;
    let artifacts = [
        Artifact {
            relative_path: GENERATED_PROTOCOL,
            contents: protocol.contents,
        },
        Artifact {
            relative_path: GENERATED_FIELDS,
            contents: protocol.fields,
        },
        Artifact {
            relative_path: GENERATED_VARIANT_FIELDS,
            contents: protocol.variant_fields,
        },
        Artifact {
            relative_path: GOLDEN_FIXTURE,
            contents: fixture::render()?,
        },
    ];
    apply("bridge", "generate:bridge", mode, &artifacts)
}

/// Emits the state/reason matrix from `BridgeConnectionStatus::ADMITTED_REASONS`.
///
/// The rule is a relation between two enums, not a type, so `ts-rs` cannot
/// carry it — which is why the TypeScript boundary kept a hand-written copy
/// that agreed with Rust by maintenance rather than by construction. Both
/// sides now read the same table.
///
/// Wire names come from serde rather than from the variant identifiers, so a
/// `rename_all` change moves both sides together.
fn render_admitted_reasons() -> Result<String, Box<dyn Error>> {
    let mut rendered = String::from(
        "export const BRIDGE_ADMITTED_CONNECTION_REASONS: Record<\n\
         \u{20} BridgeConnectionState,\n\
         \u{20} readonly (BridgeConnectionReason | null)[]\n\
         > = {\n",
    );
    for (state, reasons) in BridgeConnectionStatus::ADMITTED_REASONS {
        let state_name = serde_json::to_string(&state)?;
        let listed = reasons
            .iter()
            .map(|reason| match reason {
                Some(reason) => serde_json::to_string(reason),
                None => Ok("null".to_owned()),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        rendered.push_str(&format!("  {state_name}: [{listed}],\n"));
    }
    rendered.push_str("};\n");
    Ok(rendered)
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
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

    let contents = format!(
        "// @generated by `effigy generate:bridge`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const BRIDGE_PROTOCOL_VERSION = {BRIDGE_PROTOCOL_VERSION} as const;\n\
         export const BRIDGE_MAXIMUM_OPAQUE_ID_BYTES = {MAX_OPAQUE_ID_BYTES} as const;\n\
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
         export const BRIDGE_MAXIMUM_DEDUPLICATION_ENTRIES = {MAXIMUM_DEDUPLICATION_ENTRIES} as const;\n\
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
         {}\n\
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
        declarations.join("\n\n"),
        render_admitted_reasons()?
    );
    let (fields, skipped) = field_map("generate:bridge", "BRIDGE_FIELDS", &declarations);
    if !skipped.is_empty() {
        eprintln!(
            "[bridge] tagged unions not in the field map: {}",
            skipped.join(", ")
        );
    }

    let (variant_fields, unreadable) =
        variant_field_map("generate:bridge", "BRIDGE_VARIANT_FIELDS", &declarations);

    if !unreadable.is_empty() {
        eprintln!("[bridge] unreadable unions: {}", unreadable.join(", "));
    }

    Ok(RenderedProtocol {
        contents,
        fields,
        variant_fields,
    })
}
