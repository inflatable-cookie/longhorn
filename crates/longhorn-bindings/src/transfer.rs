use std::error::Error;

use longhorn_core::{
    ClientLogicalPx, ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, ScreenPoint,
    TransferClientId, TransferHostBindingId, TransferRequestId, WindowId,
};
use longhorn_transfer::{
    ClientDropZone, ClientEpoch, DragSessionId, InsertionPosition, LeaseGeneration,
    PanelHostBindingKind, PanelSessionStartRequest, PanelTransferCommand, PanelTransferCompletion,
    PanelTransferErrorCode, PanelTransferResponse, SessionCancellationStatus,
    TRANSFER_PROTOCOL_VERSION, TargetResolutionPath, TransferAbort, TransferAbortSource,
    TransferCancelReceipt, TransferCancelRequest, TransferCancelResponse, TransferCapability,
    TransferClientSnapshot, TransferCommitSelector, TransferCommittedTarget, TransferErrorCode,
    TransferLeaseReceipt, TransferLeaseRequest, TransferLeaseResponse, TransferPayload,
    TransferProtocolVersion, TransferRevision, TransferSessionResponse, TransferSessionStarted,
    TransferTargetBinding,
};
use ts_rs::TS;

use crate::{
    generation::{
        Artifact, DomainArtifacts, GENERATE_TASK, build_domain_artifacts, config,
        exported_declaration, tagged_variants_json,
    },
    serde_shape::{enum_names, enum_names_json},
};

mod fixture;

// Compile the adapter's wire declarations without linking the Tauri runtime.
#[allow(dead_code)]
#[path = "../../longhorn-tauri-transfer/src/host_error.rs"]
mod host_error;

const ARTIFACTS: DomainArtifacts = DomainArtifacts {
    protocol: "packages/longhorn/src/transfer/generated/protocol.ts",
    fields: "packages/longhorn/src/transfer/generated/fields.ts",
    fields_constant: "TRANSFER_FIELDS",
    variant_fields: "packages/longhorn/src/transfer/generated/variant-fields.ts",
    variant_fields_constant: "TRANSFER_VARIANT_FIELDS",
    fixture: "fixtures/transfer/protocol-v1.json",
};

/// The keys Serde writes for a screen point, which TypeScript declares inline.
fn screen_point_fields_json() -> Result<String, Box<dyn Error>> {
    let value = serde_json::to_value(ScreenPoint::new(0, 0))?;
    let fields = value
        .as_object()
        .ok_or("a screen point must serialize as an object")?;
    Ok(serde_json::to_string(&fields.keys().collect::<Vec<_>>())?)
}

/// Renders the transfer bindings and golden fixtures.
///
/// # Errors
///
/// Returns an error if declaration inspection or fixture serialization fails.
pub fn render() -> Result<Vec<Artifact>, Box<dyn Error>> {
    let target_binding = TransferTargetBinding::decl(config());
    let commit_selector = TransferCommitSelector::decl(config());
    let abort_source = TransferAbortSource::decl(config());
    let session_response = TransferSessionResponse::decl(config());
    let lease_response = TransferLeaseResponse::decl(config());
    let cancel_response = TransferCancelResponse::decl(config());
    let panel_response = PanelTransferResponse::decl(config());
    let transfer_error_codes = enum_names::<TransferErrorCode>()?;
    let panel_error_codes = enum_names::<PanelTransferErrorCode>()?;

    let [host_error_code, host_error] =
        crate::serde_shape::error_declarations::<_, host_error::TransferHostErrorCode>(
            host_error::TransferHostError::state_unavailable(),
            "TransferHostError",
            "TransferHostErrorCode",
        )?;

    let declarations = [
        DomainId::decl(config()),
        DropZoneId::decl(config()),
        TransferClientId::decl(config()),
        TransferHostBindingId::decl(config()),
        TransferRequestId::decl(config()),
        WindowId::decl(config()),
        ClientLogicalPx::decl(config()),
        ClientPoint::decl(config()),
        ClientSize::decl(config()),
        ClientRect::decl(config()),
        DragSessionId::decl(config()),
        ClientEpoch::decl(config()),
        LeaseGeneration::decl(config()),
        TransferRevision::decl(config()),
        InsertionPosition::decl(config()),
        TransferProtocolVersion::decl(config()),
        TransferCapability::decl(config()),
        target_binding.clone(),
        TransferPayload::decl(config()),
        SessionCancellationStatus::decl(config()),
        TargetResolutionPath::decl(config()),
        PanelHostBindingKind::decl(config()),
        TransferErrorCode::decl(config()),
        PanelTransferErrorCode::decl(config()),
        PanelSessionStartRequest::decl(config()),
        TransferCancelRequest::decl(config()),
        ClientDropZone::decl(config()),
        TransferLeaseRequest::decl(config()),
        commit_selector.clone(),
        PanelTransferCommand::decl(config()),
        TransferClientSnapshot::decl(config()),
        TransferSessionStarted::decl(config()),
        TransferLeaseReceipt::decl(config()),
        TransferCancelReceipt::decl(config()),
        TransferCommittedTarget::decl(config()),
        PanelTransferCompletion::decl(config()),
        abort_source.clone(),
        TransferAbort::decl(config()),
        session_response.clone(),
        lease_response.clone(),
        cancel_response.clone(),
        panel_response.clone(),
        host_error_code,
        host_error,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy {GENERATE_TASK}`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         import type {{ LayoutMutationRejectionCode, SurfaceId, SurfaceDocument, SurfaceRevision, PanelInstanceId, RegionId }} from \"@inflatable-cookie/longhorn/layout\";\n\n\
         export const TRANSFER_PROTOCOL_VERSION = {TRANSFER_PROTOCOL_VERSION} as const;\n\
         export const TRANSFER_TARGET_BINDING_KINDS = {} as const;\n\
         export const TRANSFER_COMMIT_SELECTOR_KINDS = {} as const;\n\
         export const TRANSFER_ABORT_DOMAINS = {} as const;\n\
         export const TRANSFER_SESSION_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_LEASE_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_CANCEL_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_SESSION_CANCELLATION_STATUSES = {} as const;\n\
         export const PANEL_TRANSFER_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_TARGET_RESOLUTION_PATHS = {} as const;\n\
         export const TRANSFER_PANEL_HOST_BINDING_KINDS = {} as const;\n\
         export const TRANSFER_SCREEN_POINT_FIELDS = {} as const;\n\
         export const TRANSFER_ERROR_CODES = {} as const;\n\
         export const PANEL_TRANSFER_ERROR_CODES = {} as const;\n\n\
         {}\n",
        tagged_variants_json(&target_binding)?,
        tagged_variants_json(&commit_selector)?,
        tagged_variants_json(&abort_source)?,
        tagged_variants_json(&session_response)?,
        tagged_variants_json(&lease_response)?,
        tagged_variants_json(&cancel_response)?,
        enum_names_json::<SessionCancellationStatus>()?,
        tagged_variants_json(&panel_response)?,
        enum_names_json::<TargetResolutionPath>()?,
        enum_names_json::<PanelHostBindingKind>()?,
        screen_point_fields_json()?,
        serde_json::to_string(transfer_error_codes)?,
        serde_json::to_string(panel_error_codes)?,
        declarations.join("\n\n")
    );

    let mut artifacts = build_domain_artifacts(
        &ARTIFACTS,
        contents,
        &declarations,
        fixture::render(transfer_error_codes, panel_error_codes)?,
    )?;
    artifacts.push(crate::serde_shape::error_fixture(
        "fixtures/transfer/host-errors.json",
        &[
            host_error::TransferHostError {
                code: host_error::TransferHostErrorCode::RuntimeUnavailable,
                message: "Transfer runtime temporarily unavailable".into(),
                retryable: true,
            },
            host_error::TransferHostError::state_unavailable(),
        ],
    )?);
    Ok(artifacts)
}
