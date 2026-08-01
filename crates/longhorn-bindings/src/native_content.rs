use std::error::Error;

use longhorn_core::{
    ClientCssPx, ClientPoint, ClientRect, ClientSize, NativeContentFailureCode,
    NativeContentIslandId, NativeContentKindId, NativeContentRequestId, NativeContentRevision,
    RoundingMode, ScaleFactor, VisibilityReasonId, WindowId,
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
    Artifact, GenerationMode, apply, exported_declaration, string_union_variants, tagged_variants,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/native-content/src/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/native-content/protocol-v1.json";

pub fn run(mode: GenerationMode) -> Result<(), Box<dyn Error>> {
    let artifacts = [
        Artifact {
            relative_path: GENERATED_PROTOCOL,
            contents: render_protocol()?,
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

fn render_protocol() -> Result<String, Box<dyn Error>> {
    let mechanism = NativeContentMechanism::decl();
    let detach_policy = DetachPolicy::decl();
    let input_routing = InputRoutingMode::decl();
    let desired_presence = DesiredPresence::decl();
    let desired_visibility = DesiredVisibility::decl();
    let focus_intent = FocusIntent::decl();
    let lifecycle = AttachmentLifecycle::decl();
    let readiness = ObservedReadiness::decl();
    let effective_visibility = EffectiveVisibility::decl();
    let effective_focus = EffectiveFocus::decl();
    let observed_geometry = ObservedGeometry::decl();
    let operation = NativeContentOperation::decl();
    let operation_outcome = OperationOutcome::decl();
    let decision = ContentSizeDecision::decl();
    let host_destroy_outcome = HostDestroyOutcome::decl();
    let rejection_code = NativeContentRejectionCode::decl();
    let failure_phase = NativeContentFailurePhase::decl();
    let retry_class = NativeContentRetryClass::decl();
    let connect_result = NativeContentConnectResult::decl();
    let snapshot_result = NativeContentSnapshotResult::decl();
    let update_result = NativeContentDesiredUpdateResult::decl();
    let decision_result = NativeContentContentSizeDecisionResult::decl();
    let change = NativeContentChangeProjection::decl();
    let declarations = [
        ClientCssPx::decl(),
        ClientPoint::decl(),
        ClientSize::decl(),
        ClientRect::decl(),
        ScaleFactor::decl(),
        RoundingMode::decl(),
        WindowId::decl(),
        NativeContentIslandId::decl(),
        NativeContentKindId::decl(),
        NativeContentRequestId::decl(),
        NativeContentFailureCode::decl(),
        VisibilityReasonId::decl(),
        NativeContentRevision::decl(),
        AttachGeneration::decl(),
        PlanStepId::decl(),
        NativeContentProtocolVersion::decl(),
        NativeContentAuthorityEpoch::decl(),
        NativeContentClientEpoch::decl(),
        mechanism.clone(),
        detach_policy.clone(),
        input_routing.clone(),
        MechanismCapabilities::decl(),
        desired_presence.clone(),
        desired_visibility.clone(),
        focus_intent.clone(),
        lifecycle.clone(),
        readiness.clone(),
        effective_visibility.clone(),
        effective_focus.clone(),
        observed_geometry.clone(),
        DesiredState::decl(),
        DesiredUpdate::decl(),
        ObservedState::decl(),
        ObservationUpdate::decl(),
        operation.clone(),
        PlannedOperation::decl(),
        ApplyPlan::decl(),
        operation_outcome.clone(),
        StepReceipt::decl(),
        ApplyReceipt::decl(),
        ContentSizeProposal::decl(),
        decision.clone(),
        ContentSizeProposalReceipt::decl(),
        DesiredUpdateReceipt::decl(),
        ObservationReceipt::decl(),
        host_destroy_outcome.clone(),
        HostDestroyReceipt::decl(),
        rejection_code.clone(),
        failure_phase.clone(),
        retry_class.clone(),
        NativeContentProtocolRejection::decl(),
        NativeContentCursor::decl(),
        NativeContentSnapshot::decl(),
        NativeContentConnectRequest::decl(),
        NativeContentSnapshotRequest::decl(),
        NativeContentDesiredUpdateRequest::decl(),
        NativeContentContentSizeDecisionRequest::decl(),
        connect_result.clone(),
        snapshot_result.clone(),
        update_result.clone(),
        decision_result.clone(),
        change.clone(),
        NativeContentChangedEvent::decl(),
    ]
    .map(exported_declaration);

    Ok(format!(
        "// @generated by `effigy generate:native-content`; do not edit.\n\
         // Rust serde types are the wire authority. Browser, plugin, GPU, and product payloads are absent.\n\n\
         export const NATIVE_CONTENT_PROTOCOL_VERSION = {NATIVE_CONTENT_PROTOCOL_VERSION} as const;\n\
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
    ))
}
