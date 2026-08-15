use std::error::Error;

use longhorn_core::{
    ClientLogicalPx, ClientPoint, ClientRect, ClientSize, MAX_OPAQUE_ID_BYTES,
    NativeContentFailureCode, NativeContentIslandId, NativeContentKindId, NativeContentRequestId,
    NativeContentRevision, RoundingMode, ScaleFactor, VisibilityReasonId, WindowId,
};
use longhorn_native_content::{
    ApplyPlan, ApplyReceipt, AttachGeneration, AttachmentLifecycle, ContentSizeDecision,
    ContentSizeProposal, ContentSizeProposalReceipt, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredUpdateReceipt, DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility,
    FocusIntent, HostDestroyOutcome, HostDestroyReceipt, InputRoutingMode, MechanismCapabilities,
    NATIVE_CONTENT_PROTOCOL_VERSION, NativeContentAuthorityEpoch, NativeContentChangeProjection,
    NativeContentChangedEvent, NativeContentClientEpoch, NativeContentConnectRequest,
    NativeContentConnectResult, NativeContentContentSizeDecisionRequest,
    NativeContentContentSizeDecisionResult, NativeContentCursor, NativeContentDesiredUpdateRequest,
    NativeContentDesiredUpdateResult, NativeContentFailurePhase, NativeContentMechanism,
    NativeContentOperation, NativeContentProtocolRejection, NativeContentProtocolVersion,
    NativeContentRejectionCode, NativeContentRetryClass, NativeContentSnapshot,
    NativeContentSnapshotRequest, NativeContentSnapshotResult, ObservationReceipt,
    ObservationUpdate, ObservedGeometry, ObservedReadiness, ObservedState, OperationOutcome,
    PlanStepId, PlannedOperation, StepReceipt,
};
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, apply, config, exported_declaration, field_map,
    string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/native-content/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/native-content/protocol-v1.json";
const GENERATED_FIELDS: &str = "packages/longhorn/src/native-content/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str =
    "packages/longhorn/src/native-content/generated/variant-fields.ts";

struct RenderedProtocol {
    contents: String,
    fields: String,
    variant_fields: String,
}

/// Generates or checks the native-content bindings and golden fixtures.
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
    apply(
        "native-content",
        "generate:native-content",
        mode,
        &artifacts,
    )
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let mechanism = NativeContentMechanism::decl(config());
    let detach_policy = DetachPolicy::decl(config());
    let input_routing = InputRoutingMode::decl(config());
    let desired_presence = DesiredPresence::decl(config());
    let desired_visibility = DesiredVisibility::decl(config());
    let focus_intent = FocusIntent::decl(config());
    let lifecycle = AttachmentLifecycle::decl(config());
    let readiness = ObservedReadiness::decl(config());
    let effective_visibility = EffectiveVisibility::decl(config());
    let effective_focus = EffectiveFocus::decl(config());
    let observed_geometry = ObservedGeometry::decl(config());
    let operation = NativeContentOperation::decl(config());
    let operation_outcome = OperationOutcome::decl(config());
    let decision = ContentSizeDecision::decl(config());
    let host_destroy_outcome = HostDestroyOutcome::decl(config());
    let rejection_code = NativeContentRejectionCode::decl(config());
    let failure_phase = NativeContentFailurePhase::decl(config());
    let retry_class = NativeContentRetryClass::decl(config());
    let connect_result = NativeContentConnectResult::decl(config());
    let snapshot_result = NativeContentSnapshotResult::decl(config());
    let update_result = NativeContentDesiredUpdateResult::decl(config());
    let decision_result = NativeContentContentSizeDecisionResult::decl(config());
    let change = NativeContentChangeProjection::decl(config());
    let declarations = [
        ClientLogicalPx::decl(config()),
        ClientPoint::decl(config()),
        ClientSize::decl(config()),
        ClientRect::decl(config()),
        ScaleFactor::decl(config()),
        RoundingMode::decl(config()),
        WindowId::decl(config()),
        NativeContentIslandId::decl(config()),
        NativeContentKindId::decl(config()),
        NativeContentRequestId::decl(config()),
        NativeContentFailureCode::decl(config()),
        VisibilityReasonId::decl(config()),
        NativeContentRevision::decl(config()),
        AttachGeneration::decl(config()),
        PlanStepId::decl(config()),
        NativeContentProtocolVersion::decl(config()),
        NativeContentAuthorityEpoch::decl(config()),
        NativeContentClientEpoch::decl(config()),
        mechanism.clone(),
        detach_policy.clone(),
        input_routing.clone(),
        MechanismCapabilities::decl(config()),
        desired_presence.clone(),
        desired_visibility.clone(),
        focus_intent.clone(),
        lifecycle.clone(),
        readiness.clone(),
        effective_visibility.clone(),
        effective_focus.clone(),
        observed_geometry.clone(),
        DesiredState::decl(config()),
        DesiredUpdate::decl(config()),
        ObservedState::decl(config()),
        ObservationUpdate::decl(config()),
        operation.clone(),
        PlannedOperation::decl(config()),
        ApplyPlan::decl(config()),
        operation_outcome.clone(),
        StepReceipt::decl(config()),
        ApplyReceipt::decl(config()),
        ContentSizeProposal::decl(config()),
        decision.clone(),
        ContentSizeProposalReceipt::decl(config()),
        DesiredUpdateReceipt::decl(config()),
        ObservationReceipt::decl(config()),
        host_destroy_outcome.clone(),
        HostDestroyReceipt::decl(config()),
        rejection_code.clone(),
        failure_phase.clone(),
        retry_class.clone(),
        NativeContentProtocolRejection::decl(config()),
        NativeContentCursor::decl(config()),
        NativeContentSnapshot::decl(config()),
        NativeContentConnectRequest::decl(config()),
        NativeContentSnapshotRequest::decl(config()),
        NativeContentDesiredUpdateRequest::decl(config()),
        NativeContentContentSizeDecisionRequest::decl(config()),
        connect_result.clone(),
        snapshot_result.clone(),
        update_result.clone(),
        decision_result.clone(),
        change.clone(),
        NativeContentChangedEvent::decl(config()),
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy generate:native-content`; do not edit.\n\
         // Rust serde types are the wire authority. Browser, plugin, GPU, and product payloads are absent.\n\n\
         export const NATIVE_CONTENT_PROTOCOL_VERSION = {NATIVE_CONTENT_PROTOCOL_VERSION} as const;\n\
         export const NATIVE_CONTENT_MAXIMUM_OPAQUE_ID_BYTES = {MAX_OPAQUE_ID_BYTES} as const;\n\
         export const NATIVE_CONTENT_MECHANISMS = {} as const;\n\
         export const NATIVE_CONTENT_DETACH_POLICIES = {} as const;\n\
         export const NATIVE_CONTENT_INPUT_ROUTING_MODES = {} as const;\n\
         export const NATIVE_CONTENT_DESIRED_PRESENCE = {} as const;\n\
         export const NATIVE_CONTENT_DESIRED_VISIBILITY_STATES = {} as const;\n\
         export const NATIVE_CONTENT_FOCUS_INTENTS = {} as const;\n\
         export const NATIVE_CONTENT_ATTACHMENT_LIFECYCLES = {} as const;\n\
         export const NATIVE_CONTENT_READINESS_STATES = {} as const;\n\
         export const NATIVE_CONTENT_EFFECTIVE_VISIBILITY_STATES = {} as const;\n\
         export const NATIVE_CONTENT_EFFECTIVE_FOCUS_STATES = {} as const;\n\
         export const NATIVE_CONTENT_OBSERVED_GEOMETRY_KINDS = {} as const;\n\
         export const NATIVE_CONTENT_OPERATION_KINDS = {} as const;\n\
         export const NATIVE_CONTENT_OPERATION_OUTCOME_KINDS = {} as const;\n\
         export const NATIVE_CONTENT_SIZE_DECISION_KINDS = {} as const;\n\
         export const NATIVE_CONTENT_HOST_DESTROY_OUTCOMES = {} as const;\n\
         export const NATIVE_CONTENT_REJECTION_CODES = {} as const;\n\
         export const NATIVE_CONTENT_FAILURE_PHASES = {} as const;\n\
         export const NATIVE_CONTENT_RETRY_CLASSES = {} as const;\n\
         export const NATIVE_CONTENT_CONNECT_STATUSES = {} as const;\n\
         export const NATIVE_CONTENT_SNAPSHOT_STATUSES = {} as const;\n\
         export const NATIVE_CONTENT_UPDATE_STATUSES = {} as const;\n\
         export const NATIVE_CONTENT_DECISION_STATUSES = {} as const;\n\
         export const NATIVE_CONTENT_CHANGE_KINDS = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&string_union_variants(&mechanism)?)?,
        serde_json::to_string(&string_union_variants(&detach_policy)?)?,
        serde_json::to_string(&string_union_variants(&input_routing)?)?,
        serde_json::to_string(&string_union_variants(&desired_presence)?)?,
        serde_json::to_string(&tagged_variants(&desired_visibility, "state")?)?,
        serde_json::to_string(&string_union_variants(&focus_intent)?)?,
        serde_json::to_string(&string_union_variants(&lifecycle)?)?,
        serde_json::to_string(&string_union_variants(&readiness)?)?,
        serde_json::to_string(&string_union_variants(&effective_visibility)?)?,
        serde_json::to_string(&string_union_variants(&effective_focus)?)?,
        serde_json::to_string(&tagged_variants(&observed_geometry, "kind")?)?,
        serde_json::to_string(&tagged_variants(&operation, "kind")?)?,
        serde_json::to_string(&tagged_variants(&operation_outcome, "kind")?)?,
        serde_json::to_string(&tagged_variants(&decision, "kind")?)?,
        serde_json::to_string(&string_union_variants(&host_destroy_outcome)?)?,
        serde_json::to_string(&string_union_variants(&rejection_code)?)?,
        serde_json::to_string(&string_union_variants(&failure_phase)?)?,
        serde_json::to_string(&string_union_variants(&retry_class)?)?,
        serde_json::to_string(&tagged_variants(&connect_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&snapshot_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&update_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&decision_result, "status")?)?,
        serde_json::to_string(&tagged_variants(&change, "kind")?)?,
        declarations.join("\n\n")
    );
    let (fields, _skipped) = field_map(
        "generate:native-content",
        "NATIVE_CONTENT_FIELDS",
        &declarations,
    );

    let variant_fields = variant_field_map(
        "generate:native-content",
        "NATIVE_CONTENT_VARIANT_FIELDS",
        &declarations,
    );

    Ok(RenderedProtocol {
        contents,

        fields,

        variant_fields,
    })
}
