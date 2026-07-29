//! Renderer-safe whole-Surface wire contract checks.

use longhorn_core::{SurfaceId, TransferRequestId};
use longhorn_surface_transfer::SurfaceSessionStartRequest;
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
