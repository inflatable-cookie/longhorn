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
    Artifact, GenerationMode, apply, config, exported_declaration, field_map,
    string_union_variants, variant_field_map,
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

/// Generates or checks the bridge bindings and golden fixtures.
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
    let host_form = BridgeHostForm::decl(config());
    let connection_state = BridgeConnectionState::decl(config());
    let connection_reason = BridgeConnectionReason::decl(config());
    let authentication = AuthenticationPosture::decl(config());
    let availability = DomainAvailability::decl(config());
    let read_authority = ReadAuthority::decl(config());
    let write_authority = WriteAuthority::decl(config());
    let execution_authority = ExecutionAuthority::decl(config());
    let failure_phase = BridgeFailurePhase::decl(config());
    let retry_class = BridgeRetryClass::decl(config());
    let command_delivery = BridgeCommandDelivery::decl(config());
    let command_retry = BridgeCommandRetryDecision::decl(config());
    let query_retry = BridgeQueryRetryDecision::decl(config());
    let required_authority = BridgeRequiredAuthority::decl(config());
    let authority_cursor_decision = BridgeAuthorityCursorDecision::decl(config());
    let service_ownership = BridgeServiceOwnership::decl(config());
    let service_state = BridgeServiceState::decl(config());
    let service_action = BridgeServiceAction::decl(config());
    let service_failure = BridgeServiceFailureCode::decl(config());
    let service_outcome = BridgeServiceOutcome::decl(config());
    let snapshot_decision = BridgeSnapshotDecision::decl(config());
    let stream_decision = BridgeStreamDecision::decl(config());
    let progress_decision = BridgeProgressDecision::decl(config());
    let terminal_decision = BridgeJobTerminalDecision::decl(config());

    let declarations = [
        BridgeId::decl(config()),
        BridgeSessionId::decl(config()),
        HostInstanceId::decl(config()),
        BridgeCapabilityId::decl(config()),
        AuthorityScopeId::decl(config()),
        TransportFeatureId::decl(config()),
        BridgeDiagnosticId::decl(config()),
        BridgeRequestId::decl(config()),
        BridgeIdempotencyKey::decl(config()),
        BridgeJobId::decl(config()),
        BridgeErrorCode::decl(config()),
        BridgeCredentialRef::decl(config()),
        DomainId::decl(config()),
        BridgeProtocolVersion::decl(config()),
        host_form,
        BridgeHostDescriptor::decl(config()),
        connection_state,
        connection_reason,
        BridgeConnectionStatus::decl(config()),
        authentication,
        AuthorityEpoch::decl(config()),
        AuthorityRevision::decl(config()),
        availability,
        read_authority,
        write_authority,
        execution_authority,
        DomainAuthorityDescriptor::decl(config()),
        DomainCapabilityDescriptor::decl(config()),
        BridgeDiagnostic::decl(config()),
        BridgeHelloRequest::decl(config()),
        BridgeNegotiationReceipt::decl(config()),
        BridgeRequestContext::decl(config()),
        BridgeQueryEnvelope::<String>::decl(config()),
        BridgeCommandEnvelope::<String>::decl(config()),
        BridgeCancellationRequest::decl(config()),
        BridgeFailureMessage::decl(config()),
        failure_phase,
        retry_class,
        BridgeFailure::<String>::decl(config()),
        BridgeQueryOutcome::<String, String>::decl(config()),
        BridgeQueryReply::<String, String>::decl(config()),
        BridgeCommandOutcome::<String, String>::decl(config()),
        BridgeCommandReply::<String, String>::decl(config()),
        BridgeDeduplicationCapacity::decl(config()),
        BridgeDeduplicationSupport::decl(config()),
        command_delivery,
        command_retry,
        query_retry,
        BridgeMonotonicMillis::decl(config()),
        BridgeRetryAttempt::decl(config()),
        BridgeReconnectSchedule::decl(config()),
        BridgeTransitionSequence::decl(config()),
        BridgeConnectionTransitionReceipt::decl(config()),
        required_authority,
        BridgeAuthorityRequirement::decl(config()),
        authority_cursor_decision,
        BridgeStreamSequence::decl(config()),
        BridgeStreamCursor::decl(config()),
        BridgeSnapshotEnvelope::<String>::decl(config()),
        BridgeEventEnvelope::<String>::decl(config()),
        snapshot_decision,
        stream_decision,
        BridgeProgressEvent::<String>::decl(config()),
        BridgeJobTerminalOutcome::<String, String>::decl(config()),
        BridgeJobTerminalEvent::<String, String>::decl(config()),
        BridgeCancellationOutcome::<String>::decl(config()),
        BridgeCancellationReceipt::<String>::decl(config()),
        progress_decision,
        terminal_decision,
        service_ownership,
        service_state,
        service_action,
        service_failure,
        service_outcome,
        BridgeServiceRequest::decl(config()),
        BridgeServiceGeneration::decl(config()),
        BridgeServiceTransitionReceipt::decl(config()),
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
        serde_json::to_string(&string_union_variants(&BridgeHostForm::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&BridgeConnectionState::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeConnectionReason::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&AuthenticationPosture::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&DomainAvailability::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&ReadAuthority::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&WriteAuthority::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&ExecutionAuthority::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&BridgeFailurePhase::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&BridgeRetryClass::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(&BridgeCommandDelivery::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeCommandRetryDecision::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeQueryRetryDecision::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeRequiredAuthority::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(
            &BridgeAuthorityCursorDecision::decl(config())
        )?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceOwnership::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceState::decl(config()))?)?,
        serde_json::to_string(&string_union_variants(
            &BridgeServiceAction::decl(config())
        )?)?,
        serde_json::to_string(&string_union_variants(&BridgeServiceFailureCode::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeSnapshotDecision::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeStreamDecision::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeProgressDecision::decl(
            config()
        ))?)?,
        serde_json::to_string(&string_union_variants(&BridgeJobTerminalDecision::decl(
            config()
        ))?)?,
        declarations.join("\n\n"),
        render_admitted_reasons()?
    );
    let (fields, _skipped) = field_map("generate:bridge", "BRIDGE_FIELDS", &declarations);

    let variant_fields =
        variant_field_map("generate:bridge", "BRIDGE_VARIANT_FIELDS", &declarations);

    Ok(RenderedProtocol {
        contents,
        fields,
        variant_fields,
    })
}
