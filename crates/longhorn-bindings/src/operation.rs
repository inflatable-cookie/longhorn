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

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, LabelMap, build_domain_artifacts, config,
        exported_declaration, label_module, tagged_variants_json,
    },
    serde_shape::enum_names_json,
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-operation/src/error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/operation/generated/protocol.ts",
    fields: "packages/longhorn/src/operation/generated/fields.ts",
    fields_constant: "OPERATION_FIELDS",
    variant_fields: "packages/longhorn/src/operation/generated/variant-fields.ts",
    variant_fields_constant: "OPERATION_VARIANT_FIELDS",
    fixture: "fixtures/operation/protocol-v1.json",
};
const GENERATED_LABELS: &str = "packages/longhorn/src/operation/generated/labels.ts";

fn render_labels() -> String {
    let entries: Vec<(&str, &str)> = OperationStateProjection::ALL
        .iter()
        .map(|state| (state.wire_name(), state.label()))
        .collect();
    label_module(
        GENERATE_TASK,
        &[LabelMap {
            constant: "OPERATION_STATE_LABELS",
            import: "OperationStateProjection",
            key_type: "OperationStateProjection",
            entries: &entries,
        }],
    )
}

/// Renders the operation bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let overall = OperationOverallProgressProjection::decl(config());
    let teardown_resolution = OperationTeardownResolutionProjection::decl(config());
    let mutation_command = OperationMutationCommand::decl(config());
    let mutation_receipt = OperationMutationReceiptProjection::decl(config());
    let teardown_outcome = OperationTeardownOutcomeProjection::decl(config());
    let mutation_result = OperationMutationResult::decl(config());
    let executor_dispatch = OperationExecutorDispatchProjection::decl(config());
    let cancellation_result = OperationCancellationResult::decl(config());
    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::OperationHostErrorCode>(
            host_error::OperationHostError::authority_state_unavailable(),
            "OperationHostError",
            "OperationHostErrorCode",
        )?;

    let declarations = [
        OperationAuthorityId::decl(config()),
        OperationId::decl(config()),
        OperationKindId::decl(config()),
        OperationScopeId::decl(config()),
        OperationPhaseId::decl(config()),
        OperationRequestId::decl(config()),
        OperationRevision::decl(config()),
        OperationCatalogueRevision::decl(config()),
        OperationProtocolVersion::decl(config()),
        OperationAuthorityProjection::decl(config()),
        OperationStateProjection::decl(config()),
        OperationCancellationSupportProjection::decl(config()),
        overall.clone(),
        OperationPhaseProgressProjection::decl(config()),
        OperationProgressProjection::decl(config()),
        OperationEntryProjection::decl(config()),
        OperationCatalogueLimitsProjection::decl(config()),
        OperationSnapshot::decl(config()),
        OperationSnapshotQuery::decl(config()),
        OperationSnapshotResponse::decl(config()),
        teardown_resolution.clone(),
        mutation_command.clone(),
        OperationCancellationCommand::decl(config()),
        OperationRemovalReasonProjection::decl(config()),
        OperationRemovalProjection::decl(config()),
        mutation_receipt.clone(),
        teardown_outcome.clone(),
        OperationRejectionCode::decl(config()),
        OperationRejection::decl(config()),
        mutation_result.clone(),
        OperationCancellationOutcomeProjection::decl(config()),
        OperationCancellationReceiptProjection::decl(config()),
        executor_dispatch.clone(),
        cancellation_result.clone(),
        OperationChangedKind::decl(config()),
        OperationChangedEvent::decl(config()),
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
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
        enum_names_json::<OperationStateProjection>()?,
        enum_names_json::<OperationCancellationSupportProjection>()?,
        tagged_variants_json(&overall)?,
        tagged_variants_json(&teardown_resolution)?,
        tagged_variants_json(&mutation_command)?,
        enum_names_json::<OperationRemovalReasonProjection>()?,
        tagged_variants_json(&mutation_receipt)?,
        tagged_variants_json(&teardown_outcome)?,
        enum_names_json::<OperationRejectionCode>()?,
        tagged_variants_json(&mutation_result)?,
        enum_names_json::<OperationCancellationOutcomeProjection>()?,
        tagged_variants_json(&executor_dispatch)?,
        tagged_variants_json(&cancellation_result)?,
        enum_names_json::<OperationChangedKind>()?,
        declarations.join("\n\n")
    );

    let mut artifacts =
        build_domain_artifacts(&ARTIFACTS, contents, &declarations, fixture::render()?)?;
    artifacts.push(Artifact {
        relative_path: GENERATED_LABELS,
        contents: render_labels(),
    });
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/operation/host-errors.json",
        &[
            host_error::OperationHostError::authority("Host temporarily unavailable", true),
            host_error::OperationHostError::authority_state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
