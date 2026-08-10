use std::error::Error;

use longhorn_core::{SurfaceId, TransferRequestId};
use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferCommand, SurfaceTransferResponse,
};
use longhorn_transfer::{DragSessionId, TRANSFER_PROTOCOL_VERSION, TransferCommitSelector};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct GoldenFixture {
    protocol_version: u32,
    session_requests: Vec<SurfaceSessionStartRequest>,
    commit_requests: Vec<SurfaceTransferCommand>,
    session_responses: Vec<SurfaceSessionResponse>,
    commit_responses: Vec<SurfaceTransferResponse>,
    aborts: Vec<SurfaceTransferAbort>,
    incompatibility: IncompatibilityFixture,
}

#[derive(Serialize)]
struct IncompatibilityFixture {
    future_protocol_version: u32,
    unknown_target: Value,
    unknown_abort_domain: Value,
    unknown_error_code: Value,
    unknown_response_status: Value,
}

pub fn render(error_codes: &[String]) -> Result<String, Box<dyn Error>> {
    let session_id = DragSessionId::from_entropy([0xCD; 16]);
    let session_requests = vec![SurfaceSessionStartRequest::new(
        request_id("request:start-surface"),
        SurfaceId::new("surface:mix")?,
    )];
    let commit_requests = vec![
        SurfaceTransferCommand::new(
            request_id("request:commit-existing"),
            session_id,
            TransferCommitSelector::ExplicitZone {
                drop_zone_id: longhorn_core::DropZoneId::new("zone:surface")?,
            },
        ),
        SurfaceTransferCommand::new(
            request_id("request:commit-provisioned"),
            session_id,
            TransferCommitSelector::ScreenPoint {
                point: longhorn_core::ScreenPoint::new(800, 420),
            },
        ),
    ];
    let aborts = aborts(error_codes)?;
    let session_responses = vec![
        typed(json!({
            "status": "started",
            "session": {
                "protocol_version": 1,
                "request_id": "request:start-surface",
                "payload": {
                    "protocol_version": 1,
                    "session_id": session_id,
                }
            }
        }))?,
        typed(json!({"status": "aborted", "abort": aborts[0]}))?,
    ];
    let commit_responses = vec![
        typed(json!({
            "status": "committed",
            "completion": completion(
                session_id,
                "request:commit-existing",
                json!({
                    "kind": "existing",
                    "target": {
                        "path": "explicit_zone",
                        "window_id": "window:tools",
                        "drop_zone_id": "zone:surface",
                        "insertion_position": 1,
                        "binding": {
                            "kind": "surface_window",
                            "host_binding_id": "binding:tools",
                            "document_id": "app.surfaces",
                            "revision": 11,
                        }
                    }
                })
            )
        }))?,
        typed(json!({
            "status": "committed",
            "completion": completion(
                session_id,
                "request:commit-provisioned",
                json!({
                    "kind": "provisioned",
                    "drop_point": {"x": 1800, "y": 300},
                    "provisioning": {
                        "window_id": "window:secondary",
                        "host_binding_id": "binding:secondary",
                        "display_id": "display:right",
                    }
                })
            )
        }))?,
        typed(json!({"status": "aborted", "abort": aborts[1]}))?,
    ];
    let fixture = GoldenFixture {
        protocol_version: TRANSFER_PROTOCOL_VERSION,
        session_requests,
        commit_requests,
        session_responses,
        commit_responses,
        aborts,
        incompatibility: IncompatibilityFixture {
            future_protocol_version: TRANSFER_PROTOCOL_VERSION + 1,
            unknown_target: json!({"kind": "future_surface_target"}),
            unknown_abort_domain: json!({"domain": "future_domain", "code": "future_code"}),
            unknown_error_code: json!("future_surface_transfer_error"),
            unknown_response_status: json!({"status": "future_response"}),
        },
    };

    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}

fn completion(session_id: DragSessionId, request_id: &str, target: Value) -> Value {
    json!({
        "protocol_version": 1,
        "request_id": request_id,
        "session_id": session_id,
        "source_host_binding_id": "binding:main",
        "target_host_binding_id": if request_id.ends_with("existing") {
            "binding:tools"
        } else {
            "binding:secondary"
        },
        "previous_revision": 11,
        "committed_revision": 12,
        "authoritative_document": {
            "revision": 12,
            "surfaces": [],
            "panel_instances": [],
            "windows": [],
        },
        "target": target,
    })
}

fn aborts(error_codes: &[String]) -> Result<Vec<SurfaceTransferAbort>, serde_json::Error> {
    error_codes
        .iter()
        .enumerate()
        .map(|(index, code)| {
            typed(json!({
                "protocol_version": 1,
                "request_id": format!("request:surface-error-{index:02}"),
                "source": {"domain": "surface_transfer", "code": code},
                "surface_code": if code == "surface_mutation_rejected" {
                    Some("stale_revision")
                } else {
                    None
                },
                "message": format!("golden fixture for {code}"),
                "retryable": false,
                "session_consumed": index % 2 == 0,
                "reconciliation_required": code == "host_reconciliation_required",
            }))
        })
        .chain(std::iter::once_with(|| {
            typed(json!({
                "protocol_version": 1,
                "request_id": "request:transfer-error",
                "source": {"domain": "transfer", "code": "lease_expired"},
                "surface_code": null,
                "message": "golden transfer failure",
                "retryable": false,
                "session_consumed": true,
                "reconciliation_required": false,
            }))
        }))
        .collect()
}

fn typed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(value)
}

fn request_id(value: &str) -> TransferRequestId {
    TransferRequestId::new(value).expect("fixture request id is valid")
}
