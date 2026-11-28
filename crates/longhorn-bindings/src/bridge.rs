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
    HostInstanceId, TransportFeatureId,
};
use ts_rs::TS;

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, build_domain_artifacts, config,
        exported_declaration,
    },
    serde_shape::{enum_names, enum_names_json},
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-bridge/src/error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/bridge/generated/protocol.ts",
    fields: "packages/longhorn/src/bridge/generated/fields.ts",
    fields_constant: "BRIDGE_FIELDS",
    variant_fields: "packages/longhorn/src/bridge/generated/variant-fields.ts",
    variant_fields_constant: "BRIDGE_VARIANT_FIELDS",
    fixture: "fixtures/bridge/protocol-v1.json",
};

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

/// Emits the state-to-state matrix from `BridgeConnectionState::ADMITTED_TRANSITIONS`.
///
/// Keys follow `BridgeConnectionStatus::ADMITTED_REASONS`, so every state
/// appears as a key, including a state with no outgoing transition. The
/// TypeScript runtime reads this table in place of a hand-written copy.
fn render_admitted_transitions() -> Result<String, Box<dyn Error>> {
    let mut rendered = String::from(
        "export const BRIDGE_ADMITTED_CONNECTION_TRANSITIONS: Record<\n\
         \u{20} BridgeConnectionState,\n\
         \u{20} readonly BridgeConnectionState[]\n\
         > = {\n",
    );
    for (state, _) in BridgeConnectionStatus::ADMITTED_REASONS {
        let targets = BridgeConnectionState::ADMITTED_TRANSITIONS
            .iter()
            .filter(|(from, _)| *from == state)
            .map(|(_, to)| serde_json::to_string(to))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        rendered.push_str(&format!(
            "  {}: [{targets}],\n",
            serde_json::to_string(&state)?
        ));
    }
    rendered.push_str("};\n");
    Ok(rendered)
}

/// Emits the supervision tables the Rust machine admits actions from.
///
/// Like the connection matrices, these are relations between enums rather than
/// types, so `ts-rs` cannot carry them.
fn render_service_supervision() -> Result<String, Box<dyn Error>> {
    let outcomes = enum_names::<BridgeServiceOutcome>()?;
    if enum_names::<BridgeServiceAction>()?.len() != BridgeServiceAction::ALL.len() {
        return Err("BridgeServiceAction::ALL does not list every action".into());
    }
    let unit_outcomes = outcomes
        .iter()
        .filter(|name| **name != "failed")
        .collect::<Vec<_>>();

    let mut rendered = format!(
        "export const BRIDGE_SERVICE_UNIT_OUTCOMES = {} as const;\n\
         export const BRIDGE_SERVICE_FAILURE_STATE: BridgeServiceState = {};\n\
         export const BRIDGE_SERVICE_INCARNATION_STATES: readonly BridgeServiceState[] = {};\n\n",
        serde_json::to_string(&unit_outcomes)?,
        serde_json::to_string(&BridgeServiceState::OF_FAILURE)?,
        serde_json::to_string(BridgeServiceState::STARTS_INCARNATION)?,
    );

    rendered.push_str(
        "export const BRIDGE_SERVICE_ADMISSIONS: Record<\n\
         \u{20} BridgeServiceAction,\n\
         \u{20} {\n\
         \u{20}   readonly ownerships: readonly BridgeServiceOwnership[];\n\
         \u{20}   readonly states: readonly BridgeServiceState[];\n\
         \u{20}   readonly requiresOwnedLifecycle: boolean;\n\
         \u{20} }\n\
         > = {\n",
    );
    for action in BridgeServiceAction::ALL {
        rendered.push_str(&format!(
            "  {}: {},\n",
            serde_json::to_string(&action)?,
            serde_json::to_string(&action.admission_rule())?
        ));
    }
    rendered.push_str("};\n\n");

    rendered.push_str(
        "export const BRIDGE_SERVICE_ADMITTED_OBSERVATIONS: readonly (readonly [\n\
         \u{20} BridgeServiceAction,\n\
         \u{20} BridgeServiceOutcome,\n\
         \u{20} BridgeServiceState,\n\
         ])[] = [\n",
    );
    for observation in BridgeServiceAction::ADMITTED_OBSERVATIONS {
        rendered.push_str(&format!("  {},\n", serde_json::to_string(observation)?));
    }
    rendered.push_str("];\n");
    Ok(rendered)
}

/// Renders the bridge bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
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

    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::BridgeHostErrorCode>(
            host_error::BridgeHostError::authority("fixture", true),
            "BridgeHostError",
            "BridgeHostErrorCode",
        )?;

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
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         export const BRIDGE_PROTOCOL_VERSION = {BRIDGE_PROTOCOL_VERSION} as const;\n\
         export const BRIDGE_DOMAIN_ID_RULE = {domain_id_rule} as const;\n\
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
         {}\n\
         {}\n\
         {}\n",
        enum_names_json::<BridgeHostForm>()?,
        enum_names_json::<BridgeConnectionState>()?,
        enum_names_json::<BridgeConnectionReason>()?,
        enum_names_json::<AuthenticationPosture>()?,
        enum_names_json::<DomainAvailability>()?,
        enum_names_json::<ReadAuthority>()?,
        enum_names_json::<WriteAuthority>()?,
        enum_names_json::<ExecutionAuthority>()?,
        enum_names_json::<BridgeFailurePhase>()?,
        enum_names_json::<BridgeRetryClass>()?,
        enum_names_json::<BridgeCommandDelivery>()?,
        enum_names_json::<BridgeCommandRetryDecision>()?,
        enum_names_json::<BridgeQueryRetryDecision>()?,
        enum_names_json::<BridgeRequiredAuthority>()?,
        enum_names_json::<BridgeAuthorityCursorDecision>()?,
        enum_names_json::<BridgeServiceOwnership>()?,
        enum_names_json::<BridgeServiceState>()?,
        enum_names_json::<BridgeServiceAction>()?,
        enum_names_json::<BridgeServiceFailureCode>()?,
        enum_names_json::<BridgeSnapshotDecision>()?,
        enum_names_json::<BridgeStreamDecision>()?,
        enum_names_json::<BridgeProgressDecision>()?,
        enum_names_json::<BridgeJobTerminalDecision>()?,
        declarations.join("\n\n"),
        render_admitted_reasons()?,
        render_admitted_transitions()?,
        render_service_supervision()?,
        domain_id_rule = serde_json::json!({
            "maximumBytes": DomainId::MAX_BYTES,
            "separator": DomainId::SEPARATOR,
            "segmentInitial": DomainId::SEGMENT_INITIAL,
            "segmentAdditional": DomainId::SEGMENT_ADDITIONAL,
        }),
    );

    let mut artifacts =
        build_domain_artifacts(&ARTIFACTS, contents, &declarations, fixture::render()?)?;
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/bridge/host-errors.json",
        &[
            host_error::BridgeHostError::authority("Host temporarily unavailable", true),
            host_error::BridgeHostError::state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
