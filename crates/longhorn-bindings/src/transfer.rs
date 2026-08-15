use std::error::Error;

use longhorn_core::{
    ClientLogicalPx, ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, TransferClientId,
    TransferHostBindingId, TransferRequestId, WindowId,
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

use crate::generation::{
    Artifact, GenerationMode, apply, config, exported_declaration, field_map,
    string_union_variants, tagged_variants, variant_field_map,
};

mod fixture;

const GENERATED_PROTOCOL: &str = "packages/longhorn/src/transfer/generated/protocol.ts";
const GENERATED_FIELDS: &str = "packages/longhorn/src/transfer/generated/fields.ts";
const GENERATED_VARIANT_FIELDS: &str = "packages/longhorn/src/transfer/generated/variant-fields.ts";
const GOLDEN_FIXTURE: &str = "fixtures/transfer/protocol-v1.json";

struct RenderedProtocol {
    contents: String,
    fields: String,
    transfer_error_codes: Vec<String>,
    panel_error_codes: Vec<String>,
    variant_fields: String,
}

/// Generates or checks the transfer bindings and golden fixtures.
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
            contents: fixture::render(&protocol.transfer_error_codes, &protocol.panel_error_codes)?,
        },
    ];
    apply("transfer", "generate:transfer", mode, &artifacts)
}

fn render_protocol() -> Result<RenderedProtocol, Box<dyn Error>> {
    let target_binding = TransferTargetBinding::decl(config());
    let commit_selector = TransferCommitSelector::decl(config());
    let abort_source = TransferAbortSource::decl(config());
    let session_response = TransferSessionResponse::decl(config());
    let lease_response = TransferLeaseResponse::decl(config());
    let cancel_response = TransferCancelResponse::decl(config());
    let panel_response = PanelTransferResponse::decl(config());
    let transfer_errors = TransferErrorCode::decl(config());
    let panel_errors = PanelTransferErrorCode::decl(config());

    let target_kinds = tagged_variants(&target_binding, "kind")?;
    let selector_kinds = tagged_variants(&commit_selector, "kind")?;
    let abort_domains = tagged_variants(&abort_source, "domain")?;
    let session_statuses = tagged_variants(&session_response, "status")?;
    let lease_statuses = tagged_variants(&lease_response, "status")?;
    let cancel_statuses = tagged_variants(&cancel_response, "status")?;
    let panel_statuses = tagged_variants(&panel_response, "status")?;
    let transfer_error_codes = string_union_variants(&transfer_errors)?;
    let panel_error_codes = string_union_variants(&panel_errors)?;

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
        target_binding,
        TransferPayload::decl(config()),
        SessionCancellationStatus::decl(config()),
        TargetResolutionPath::decl(config()),
        PanelHostBindingKind::decl(config()),
        transfer_errors,
        panel_errors,
        PanelSessionStartRequest::decl(config()),
        TransferCancelRequest::decl(config()),
        ClientDropZone::decl(config()),
        TransferLeaseRequest::decl(config()),
        commit_selector,
        PanelTransferCommand::decl(config()),
        TransferClientSnapshot::decl(config()),
        TransferSessionStarted::decl(config()),
        TransferLeaseReceipt::decl(config()),
        TransferCancelReceipt::decl(config()),
        TransferCommittedTarget::decl(config()),
        PanelTransferCompletion::decl(config()),
        abort_source,
        TransferAbort::decl(config()),
        session_response,
        lease_response,
        cancel_response,
        panel_response,
    ]
    .map(exported_declaration);

    let contents = format!(
        "// @generated by `effigy generate:transfer`; do not edit.\n\
         // Rust serde types are the wire authority.\n\n\
         import type {{ SurfaceId, SurfaceDocument, SurfaceRevision, PanelInstanceId, RegionId }} from \"@inflatable-cookie/longhorn/layout\";\n\n\
         export const TRANSFER_PROTOCOL_VERSION = {TRANSFER_PROTOCOL_VERSION} as const;\n\
         export const TRANSFER_TARGET_BINDING_KINDS = {} as const;\n\
         export const TRANSFER_COMMIT_SELECTOR_KINDS = {} as const;\n\
         export const TRANSFER_ABORT_DOMAINS = {} as const;\n\
         export const TRANSFER_SESSION_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_LEASE_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_CANCEL_RESPONSE_STATUSES = {} as const;\n\
         export const PANEL_TRANSFER_RESPONSE_STATUSES = {} as const;\n\
         export const TRANSFER_ERROR_CODES = {} as const;\n\
         export const PANEL_TRANSFER_ERROR_CODES = {} as const;\n\n\
         {}\n",
        serde_json::to_string(&target_kinds)?,
        serde_json::to_string(&selector_kinds)?,
        serde_json::to_string(&abort_domains)?,
        serde_json::to_string(&session_statuses)?,
        serde_json::to_string(&lease_statuses)?,
        serde_json::to_string(&cancel_statuses)?,
        serde_json::to_string(&panel_statuses)?,
        serde_json::to_string(&transfer_error_codes)?,
        serde_json::to_string(&panel_error_codes)?,
        declarations.join("\n\n")
    );
    let (fields, _skipped) = field_map("generate:transfer", "TRANSFER_FIELDS", &declarations);

    let variant_fields = variant_field_map(
        "generate:transfer",
        "TRANSFER_VARIANT_FIELDS",
        &declarations,
    );

    Ok(RenderedProtocol {
        variant_fields,
        contents,
        fields,
        transfer_error_codes,
        panel_error_codes,
    })
}
