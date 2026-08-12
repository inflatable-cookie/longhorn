use std::error::Error;

use longhorn_core::{
    OperationAuthorityId, OperationCatalogueRevision, OperationId, OperationKindId,
    OperationPhaseId, OperationRequestId, OperationRevision, OperationScopeId,
};
use longhorn_operation::{
    OPERATION_PROTOCOL_VERSION, OperationAuthorityProjection, OperationCancellationCommand,
    OperationCancellationOutcomeProjection, OperationCancellationReceiptProjection,
    OperationCancellationResult, OperationCancellationSupportProjection,
    OperationCatalogueLimitsProjection, OperationChangedEvent, OperationChangedKind,
    OperationEntryProjection, OperationExecutorDispatchProjection, OperationMutationCommand,
    OperationMutationReceiptProjection, OperationMutationResult,
    OperationOverallProgressProjection, OperationPhaseProgressProjection,
    OperationProgressProjection, OperationProtocolVersion, OperationRejection,
    OperationRejectionCode, OperationRemovalProjection, OperationRemovalReasonProjection,
    OperationSnapshot, OperationSnapshotQuery, OperationSnapshotResponse, OperationStateProjection,
    OperationTeardownOutcomeProjection, OperationTeardownResolutionProjection,
};
use ts_rs::TS;

use crate::generation::{
    Artifact, GenerationMode, LabelMap, apply, exported_declaration, field_map, label_module,
    string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/operation/generated/protocol.ts";
const GOLDEN_FIXTURE: &str = "fixtures/operation/protocol-v1.json";
const GENERATED_FIELDS: &str = "packages/longhorn/src/operation/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str =
    "packages/longhorn/src/operation/generated/variant-fields.ts";

struct RenderedProtocol {
    contents: String,
    fields: String,
    variant_fields: String,
}
const GENERATED_LABELS: &str = "packages/longhorn/src/operation/generated/labels.ts";

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
        Artifact {
            relative_path: GENERATED_LABELS,
            contents: render_labels(),
        },
    ];
    apply("operation", "generate:operation", mode, &artifacts)
}

fn render_labels() -> String {
    let entries: Vec<(&str, &str)> = OperationStateProjection::ALL
        .iter()
        .map(|state| (state.wire_name(), state.label()))
        .collect();
    label_module(
        "generate:operation",
        &[LabelMap {
            constant: "OPERATION_STATE_LABELS",
            import: "OperationStateProjection",
            key_type: "OperationStateProjection",
            entries: &entries,
        }],
    )
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let state = OperationStateProjection::decl();
    let cancellation_support = OperationCancellationSupportProjection::decl();
    let overall = OperationOverallProgressProjection::decl();
    let teardown_resolution = OperationTeardownResolutionProjection::decl();
    let mutation_command = OperationMutationCommand::decl();
    let removal_reason = OperationRemovalReasonProjection::decl();
    let mutation_receipt = OperationMutationReceiptProjection::decl();
    let teardown_outcome = OperationTeardownOutcomeProjection::decl();
    let rejection_code = OperationRejectionCode::decl();
    let mutation_result = OperationMutationResult::decl();
    let cancellation_outcome = OperationCancellationOutcomeProjection::decl();
    let executor_dispatch = OperationExecutorDispatchProjection::decl();
    let cancellation_result = OperationCancellationResult::decl();
    let changed_kind = OperationChangedKind::decl();
    let declarations = [
        OperationAuthorityId::decl(),
        OperationId::decl(),
        OperationKindId::decl(),
        OperationScopeId::decl(),
        OperationPhaseId::decl(),
        OperationRequestId::decl(),
        OperationRevision::decl(),
        OperationCatalogueRevision::decl(),
        OperationProtocolVersion::decl(),
        OperationAuthorityProjection::decl(),
        state.clone(),
        cancellation_support.clone(),
        overall.clone(),
        OperationPhaseProgressProjection::decl(),
        OperationProgressProjection::decl(),
        OperationEntryProjection::decl(),
        OperationCatalogueLimitsProjection::decl(),
        OperationSnapshot::decl(),
        OperationSnapshotQuery::decl(),
        OperationSnapshotResponse::decl(),
        teardown_resolution.clone(),
        mutation_command.clone(),
        OperationCancellationCommand::decl(),
        removal_reason.clone(),
        OperationRemovalProjection::decl(),
        mutation_receipt.clone(),
        teardown_outcome.clone(),
        rejection_code.clone(),
        OperationRejection::decl(),
        mutation_result.clone(),
        cancellation_outcome.clone(),
        OperationCancellationReceiptProjection::decl(),
        executor_dispatch.clone(),
        cancellation_result.clone(),
        changed_kind.clone(),
        OperationChangedEvent::decl(),
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy generate:operation`; do not edit.\n\
         // Rust serde types are the wire authority. Product payloads are intentionally absent.\n\n\
         export const OPERATION_PROTOCOL_VERSION = {OPERATION_PROTOCOL_VERSION} as const;\n\
         export const OPERATION_STATES = {} as const;\n\
         export const OPERATION_CANCELLATION_SUPPORT = {} as const;\n\
         export const OPERATION_PROGRESS_KINDS = {} as const;\n\
         export const OPERATION_TEARDOWN_RESOLUTION_KINDS = {} as const;\n\
         export const OPERATION_MUTATION_KINDS = {} as const;\n\
         export const OPERATION_REMOVAL_REASONS = {} as const;\n\
         export const OPERATION_MUTATION_RECEIPT_KINDS = {} as const;\n\
         export const OPERATION_TEARDOWN_OUTCOME_KINDS = {} as const;\n\
         export const OPERATION_REJECTION_CODES = {} as const;\n\
         export const OPERATION_MUTATION_STATUSES = {} as const;\n\
         export const OPERATION_CANCELLATION_OUTCOMES = {} as const;\n\
         export const OPERATION_EXECUTOR_DISPATCH_KINDS = {} as const;\n\
         export const OPERATION_CANCELLATION_STATUSES = {} as const;\n\
         export const OPERATION_CHANGED_KINDS = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&string_union_variants(&state)?)?,
        serde_json::to_string(&string_union_variants(&cancellation_support)?)?,
        serde_json::to_string(&tagged_variants(&overall, "kind")?)?,
        serde_json::to_string(&tagged_variants(&teardown_resolution, "kind")?)?,
        serde_json::to_string(&tagged_variants(&mutation_command, "kind")?)?,
        serde_json::to_string(&string_union_variants(&removal_reason)?)?,
        serde_json::to_string(&tagged_variants(&mutation_receipt, "kind")?)?,
        serde_json::to_string(&tagged_variants(&teardown_outcome, "kind")?)?,
        serde_json::to_string(&string_union_variants(&rejection_code)?)?,
        serde_json::to_string(&tagged_variants(&mutation_result, "status")?)?,
        serde_json::to_string(&string_union_variants(&cancellation_outcome)?)?,
        serde_json::to_string(&tagged_variants(&executor_dispatch, "kind")?)?,
        serde_json::to_string(&tagged_variants(&cancellation_result, "status")?)?,
        serde_json::to_string(&string_union_variants(&changed_kind)?)?,
        declarations.join("\n\n")
    );
    let (fields, _skipped) = field_map("generate:operation", "OPERATION_FIELDS", &declarations);

    let variant_fields = variant_field_map(
        "generate:operation",
        "OPERATION_VARIANT_FIELDS",
        &declarations,
    );

    Ok(RenderedProtocol {
        contents,

        fields,

        variant_fields,
    })
}
