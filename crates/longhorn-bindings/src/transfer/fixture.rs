use std::error::Error;

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, PanelInstanceId, RegionId,
    SurfaceId, TransferClientId, TransferHostBindingId, TransferRequestId,
};
use longhorn_transfer::{
    ClientDropZone, ClientEpoch, DragSessionId, InsertionPosition, LeaseGeneration,
    PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse,
    TRANSFER_PROTOCOL_VERSION, TransferAbort, TransferCancelRequest, TransferCancelResponse,
    TransferClientSnapshot, TransferCommitSelector, TransferLeaseRequest, TransferLeaseResponse,
    TransferSessionResponse, TransferTargetBinding,
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct GoldenFixture {
    protocol_version: u32,
    client_snapshot: TransferClientSnapshot,
    session_requests: Vec<PanelSessionStartRequest>,
    lease_requests: Vec<TransferLeaseRequest>,
    commit_requests: Vec<PanelTransferCommand>,
    cancel_requests: Vec<TransferCancelRequest>,
    session_responses: Vec<TransferSessionResponse>,
    lease_responses: Vec<TransferLeaseResponse>,
    commit_responses: Vec<PanelTransferResponse>,
    cancel_responses: Vec<TransferCancelResponse>,
    aborts: Vec<TransferAbort>,
    incompatibility: IncompatibilityFixture,
}

#[derive(Serialize)]
struct IncompatibilityFixture {
    future_protocol_version: u32,
    unknown_target_binding: Value,
    unknown_commit_selector: Value,
    unknown_response_status: Value,
    unknown_abort_domain: Value,
    unknown_transfer_error_code: Value,
    unknown_panel_error_code: Value,
}

pub fn render(
    transfer_error_codes: &[String],
    panel_error_codes: &[String],
) -> Result<String, Box<dyn Error>> {
    let session_id = DragSessionId::from_entropy([0xAB; 16]);
    let client_id = TransferClientId::new("client:main")?;
    let epoch = ClientEpoch::new(8);
    let generation = LeaseGeneration::new(3);
    let request_id = transfer_request_id("request:lease");
    let zones = [panel_zone(), surface_zone()];
    let client_snapshot = TransferClientSnapshot::new(client_id.clone(), epoch);
    let session_requests = vec![PanelSessionStartRequest::new(
        transfer_request_id("request:start-panel"),
        PanelInstanceId::new("instance:inspector")?,
    )];
    let lease_requests = vec![TransferLeaseRequest::new(
        request_id,
        client_id.clone(),
        epoch,
        generation,
        zones,
    )];
    let commit_requests = vec![
        PanelTransferCommand::new(
            transfer_request_id("request:commit-zone"),
            session_id,
            TransferCommitSelector::ExplicitZone {
                drop_zone_id: DropZoneId::new("zone:center")?,
            },
        ),
        PanelTransferCommand::new(
            transfer_request_id("request:commit-point"),
            session_id,
            TransferCommitSelector::ScreenPoint {
                point: longhorn_core::ScreenPoint::new(220, 160),
            },
        ),
    ];
    let cancel_requests = vec![TransferCancelRequest::new(
        transfer_request_id("request:cancel"),
        session_id,
    )];
    let aborts = aborts(transfer_error_codes, panel_error_codes)?;
    let session_abort = find_transfer_abort(&aborts, "session_capacity");
    let lease_abort = find_transfer_abort(&aborts, "invalid_lease");
    let commit_abort = find_transfer_abort(&aborts, "no_target");
    let cancel_abort = find_transfer_abort(&aborts, "unknown_session");
    let session_responses = vec![
        typed(json!({
            "status": "started",
            "session": {
                "protocol_version": 1,
                "request_id": "request:start-panel",
                "payload": {
                    "protocol_version": 1,
                    "session_id": session_id,
                }
            }
        }))?,
        typed(json!({"status": "aborted", "abort": session_abort}))?,
    ];
    let lease_responses = vec![
        typed(json!({
            "status": "published",
            "lease": {
                "protocol_version": 1,
                "request_id": "request:lease",
                "client_id": client_id,
                "client_epoch": epoch,
                "generation": generation,
                "zone_count": 2,
            }
        }))?,
        typed(json!({"status": "aborted", "abort": lease_abort}))?,
    ];
    let commit_responses = vec![
        typed(json!({
            "status": "committed",
            "completion": {
                "protocol_version": 1,
                "request_id": "request:commit-zone",
                "session_id": session_id,
                "source_host_binding_id": "binding:source",
                "target_host_binding_id": "binding:target",
                "source_binding_kind": "direct_window",
                "target_binding_kind": "surface_container",
                "previous_revision": 7,
                "committed_revision": 8,
                "authoritative_document": {
                    "revision": 8,
                    "surfaces": [],
                    "panel_instances": [],
                    "windows": [],
                },
                "target": {
                    "path": "explicit_zone",
                    "window_id": "window:target",
                    "drop_zone_id": "zone:center",
                    "insertion_position": 1,
                    "binding": {
                        "kind": "panel_region",
                        "host_binding_id": "binding:target",
                        "document_id": "app.layout",
                        "revision": 7,
                        "surface_id": "surface:target",
                        "region_id": "center",
                    }
                }
            }
        }))?,
        typed(json!({"status": "aborted", "abort": commit_abort}))?,
    ];
    let cancel_responses = vec![
        typed(json!({
            "status": "cancelled",
            "cancellation": {
                "protocol_version": 1,
                "request_id": "request:cancel",
                "session_id": session_id,
                "status": "cancelled",
            }
        }))?,
        typed(json!({"status": "aborted", "abort": cancel_abort}))?,
    ];
    let fixture = GoldenFixture {
        protocol_version: TRANSFER_PROTOCOL_VERSION,
        client_snapshot,
        session_requests,
        lease_requests,
        commit_requests,
        cancel_requests,
        session_responses,
        lease_responses,
        commit_responses,
        cancel_responses,
        aborts,
        incompatibility: IncompatibilityFixture {
            future_protocol_version: TRANSFER_PROTOCOL_VERSION + 1,
            unknown_target_binding: json!({"kind": "future_target"}),
            unknown_commit_selector: json!({"kind": "future_selector"}),
            unknown_response_status: json!({"status": "future_response"}),
            unknown_abort_domain: json!({"domain": "future_domain", "code": "future_code"}),
            unknown_transfer_error_code: json!("future_transfer_error"),
            unknown_panel_error_code: json!("future_panel_error"),
        },
    };

    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}

fn panel_zone() -> ClientDropZone {
    ClientDropZone::new(
        DropZoneId::new("zone:center").expect("fixture zone id is valid"),
        ClientRect::new(
            ClientPoint::new(20.0, 30.0).expect("fixture point is valid"),
            ClientSize::new(400.0, 260.0).expect("fixture size is valid"),
        ),
        Some(InsertionPosition::new(1)),
        TransferTargetBinding::PanelRegion {
            host_binding_id: TransferHostBindingId::new("binding:target")
                .expect("fixture binding is valid"),
            document_id: DomainId::new("app.layout").expect("fixture domain is valid"),
            revision: longhorn_transfer::TransferRevision::new(7),
            surface_id: SurfaceId::new("surface:target").expect("fixture container is valid"),
            region_id: RegionId::new("center").expect("fixture region is valid"),
        },
    )
}

fn surface_zone() -> ClientDropZone {
    ClientDropZone::new(
        DropZoneId::new("zone:surface").expect("fixture zone id is valid"),
        ClientRect::new(
            ClientPoint::new(440.0, 30.0).expect("fixture point is valid"),
            ClientSize::new(300.0, 260.0).expect("fixture size is valid"),
        ),
        None,
        TransferTargetBinding::SurfaceWindow {
            host_binding_id: TransferHostBindingId::new("binding:surface")
                .expect("fixture binding is valid"),
            document_id: DomainId::new("app.surfaces").expect("fixture domain is valid"),
            revision: longhorn_transfer::TransferRevision::new(9),
        },
    )
}

fn aborts(
    transfer_error_codes: &[String],
    panel_error_codes: &[String],
) -> Result<Vec<TransferAbort>, serde_json::Error> {
    transfer_error_codes
        .iter()
        .enumerate()
        .map(|(index, code)| {
            typed(json!({
                "protocol_version": 1,
                "request_id": format!("request:transfer-error-{index:02}"),
                "source": {"domain": "transfer", "code": code},
                "layout_code": null,
                "message": format!("golden fixture for {code}"),
                "retryable": false,
                "session_consumed": transfer_code_consumes_session(code),
            }))
        })
        .chain(panel_error_codes.iter().enumerate().map(|(index, code)| {
            typed(json!({
                "protocol_version": 1,
                "request_id": format!("request:panel-error-{index:02}"),
                "source": {"domain": "panel", "code": code},
                "layout_code": panel_layout_code(code),
                "message": format!("golden fixture for {code}"),
                "retryable": false,
                "session_consumed": panel_code_consumes_session(code),
            }))
        }))
        .collect()
}

fn panel_layout_code(code: &str) -> Option<&'static str> {
    match code {
        "ineligible_target" => Some("panel_placement_not_allowed"),
        "instance_policy_exceeded" => Some("instance_policy_exceeded"),
        "invalid_insertion_position" => Some("invalid_insertion_index"),
        "layout_mutation_rejected" => Some("move_target_unchanged"),
        _ => None,
    }
}

fn find_transfer_abort<'a>(aborts: &'a [TransferAbort], code: &str) -> &'a TransferAbort {
    aborts
        .iter()
        .find(|abort| {
            serde_json::to_value(abort.source())
                .ok()
                .is_some_and(|value| value["domain"] == "transfer" && value["code"] == code)
        })
        .unwrap_or_else(|| panic!("missing transfer abort fixture for {code}"))
}

fn transfer_code_consumes_session(code: &str) -> bool {
    matches!(
        code,
        "lease_expired"
            | "invalid_live_windows"
            | "target_window_missing"
            | "stale_window_geometry"
            | "no_target"
            | "ambiguous_window"
            | "ambiguous_zone"
            | "ineligible_capability"
    )
}

// Codes that can occur during either admission or commit (such as
// `unknown_host_binding` or `stale_host_binding`) are represented in their
// unconsumed admission phase here; commit-only codes are always consumed.
fn panel_code_consumes_session(code: &str) -> bool {
    matches!(
        code,
        "unsupported_source"
            | "unsupported_target"
            | "cross_document"
            | "copy_unsupported"
            | "stale_surface_revision"
            | "target_changed"
            | "ineligible_target"
            | "instance_policy_exceeded"
            | "invalid_insertion_position"
            | "layout_mutation_rejected"
            | "publication_failed"
    )
}

fn typed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(value)
}

fn transfer_request_id(value: &str) -> TransferRequestId {
    TransferRequestId::new(value).expect("fixture request id is valid")
}
