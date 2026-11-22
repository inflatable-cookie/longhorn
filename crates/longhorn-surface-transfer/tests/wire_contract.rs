//! Renderer-safe whole-Surface wire contract checks.

use longhorn_core::{DomainId, SurfaceId, TransferHostBindingId, TransferRequestId, WindowId};
use longhorn_surface_transfer::{
    SurfaceHostBinding, SurfaceHostBindings, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferAbortSource, SurfaceTransferErrorCode,
};
use longhorn_transfer::TransferErrorCode;
use serde_json::json;

#[test]
fn admission_wire_shape_cannot_supply_window_or_host_authority() {
    let request = SurfaceSessionStartRequest::new(
        TransferRequestId::new("request:start").unwrap(),
        SurfaceId::new("surface:mix").unwrap(),
    );
    let value = serde_json::to_value(request).unwrap();
    let object = value.as_object().unwrap();

    assert_eq!(object.len(), 3);
    for required in ["protocol_version", "request_id", "surface_id"] {
        assert!(object.contains_key(required));
    }
    for forbidden in [
        "window_id",
        "host_binding_id",
        "lifetime",
        "document",
        "mutation_options",
    ] {
        assert!(!object.contains_key(forbidden));
    }
}

#[test]
fn strict_wire_deserialization_rejects_future_version_and_spoof_fields() {
    let valid = json!({
        "protocol_version": 1,
        "request_id": "request:start",
        "surface_id": "surface:mix",
    });
    assert!(serde_json::from_value::<SurfaceSessionStartRequest>(valid.clone()).is_ok());

    let mut future = valid.clone();
    future["protocol_version"] = json!(2);
    assert!(serde_json::from_value::<SurfaceSessionStartRequest>(future).is_err());

    let mut spoofed = valid;
    spoofed["window_id"] = json!("window:other");
    assert!(serde_json::from_value::<SurfaceSessionStartRequest>(spoofed).is_err());
}

#[test]
fn surface_transfer_abort_from_domain_projects_codes_and_flags() {
    let duplicate = SurfaceHostBindings::new([
        SurfaceHostBinding::new(
            TransferHostBindingId::new("binding:dup").unwrap(),
            WindowId::new("window:main").unwrap(),
            DomainId::new("app.surfaces").unwrap(),
        ),
        SurfaceHostBinding::new(
            TransferHostBindingId::new("binding:dup").unwrap(),
            WindowId::new("window:main").unwrap(),
            DomainId::new("app.surfaces").unwrap(),
        ),
    ])
    .unwrap_err();
    let abort = SurfaceTransferAbort::from_domain(
        TransferRequestId::new("request:surface").unwrap(),
        &duplicate,
    );
    assert_eq!(
        abort.source(),
        SurfaceTransferAbortSource::SurfaceTransfer(
            SurfaceTransferErrorCode::InvalidBindingSnapshot
        )
    );
    assert_eq!(abort.surface_code(), None);
    assert!(!abort.retryable());
    assert!(!abort.session_consumed());
    assert!(!abort.reconciliation_required());

    let host_abort = SurfaceTransferAbort::host_transfer(
        TransferRequestId::new("request:host").unwrap(),
        TransferErrorCode::UnknownClientEpoch,
        "unbound caller",
    );
    assert_eq!(
        host_abort.source(),
        SurfaceTransferAbortSource::Transfer(TransferErrorCode::UnknownClientEpoch)
    );
    assert!(!host_abort.retryable());
    assert!(!host_abort.session_consumed());
    assert!(!host_abort.reconciliation_required());
}
