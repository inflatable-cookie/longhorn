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

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, build_domain_artifacts, config,
        exported_declaration, tagged_variants_json,
    },
    serde_shape::enum_names_json,
};

mod fixture;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/native-content/generated/protocol.ts",
    fields: "packages/longhorn/src/native-content/generated/fields.ts",
    fields_constant: "NATIVE_CONTENT_FIELDS",
    variant_fields: "packages/longhorn/src/native-content/generated/variant-fields.ts",
    variant_fields_constant: "NATIVE_CONTENT_VARIANT_FIELDS",
    fixture: "fixtures/native-content/protocol-v1.json",
};

/// Renders the native-content bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let desired_visibility = DesiredVisibility::decl(config());
    let observed_geometry = ObservedGeometry::decl(config());
    let operation = NativeContentOperation::decl(config());
    let operation_outcome = OperationOutcome::decl(config());
    let decision = ContentSizeDecision::decl(config());
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
        NativeContentMechanism::decl(config()),
        DetachPolicy::decl(config()),
        InputRoutingMode::decl(config()),
        MechanismCapabilities::decl(config()),
        DesiredPresence::decl(config()),
        desired_visibility.clone(),
        FocusIntent::decl(config()),
        AttachmentLifecycle::decl(config()),
        ObservedReadiness::decl(config()),
        EffectiveVisibility::decl(config()),
        EffectiveFocus::decl(config()),
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
        HostDestroyOutcome::decl(config()),
        HostDestroyReceipt::decl(config()),
        NativeContentRejectionCode::decl(config()),
        NativeContentFailurePhase::decl(config()),
        NativeContentRetryClass::decl(config()),
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
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
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
        enum_names_json::<NativeContentMechanism>()?,
        enum_names_json::<DetachPolicy>()?,
        enum_names_json::<InputRoutingMode>()?,
        enum_names_json::<DesiredPresence>()?,
        tagged_variants_json(&desired_visibility)?,
        enum_names_json::<FocusIntent>()?,
        enum_names_json::<AttachmentLifecycle>()?,
        enum_names_json::<ObservedReadiness>()?,
        enum_names_json::<EffectiveVisibility>()?,
        enum_names_json::<EffectiveFocus>()?,
        tagged_variants_json(&observed_geometry)?,
        tagged_variants_json(&operation)?,
        tagged_variants_json(&operation_outcome)?,
        tagged_variants_json(&decision)?,
        enum_names_json::<HostDestroyOutcome>()?,
        enum_names_json::<NativeContentRejectionCode>()?,
        enum_names_json::<NativeContentFailurePhase>()?,
        enum_names_json::<NativeContentRetryClass>()?,
        tagged_variants_json(&connect_result)?,
        tagged_variants_json(&snapshot_result)?,
        tagged_variants_json(&update_result)?,
        tagged_variants_json(&decision_result)?,
        tagged_variants_json(&change)?,
        declarations.join("\n\n")
    );

    build_domain_artifacts(&ARTIFACTS, contents, &declarations, fixture::render()?)
}
