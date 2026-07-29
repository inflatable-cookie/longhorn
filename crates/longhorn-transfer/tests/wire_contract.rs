//! Renderer-safe transfer wire contract checks.

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, DomainId, DropZoneId, LayoutContainerId, RegionId,
    TransferClientId, TransferHostBindingId, TransferRequestId,
};
use longhorn_transfer::{
    ClientDropZone, ClientEpoch, InsertionPosition, LeaseGeneration, PanelSessionStartRequest,
    TransferCapability, TransferLeaseRequest, TransferRevision, TransferTargetBinding,
};
use serde_json::json;

#[test]
fn lease_wire_shape_cannot_supply_window_clock_or_screen_authority() {
    let request = TransferLeaseRequest::new(
        request_id("request:lease"),
        TransferClientId::new("client:main").unwrap(),
        ClientEpoch::new(4),
        LeaseGeneration::new(7),
        [ClientDropZone::new(
            DropZoneId::new("zone:center").unwrap(),
            ClientRect::new(
                ClientPoint::new(10.0, 20.0).unwrap(),
                ClientSize::new(300.0, 200.0).unwrap(),
            ),
            Some(InsertionPosition::new(1)),
            TransferCapability::MovePanel,
            TransferTargetBinding::PanelRegion {
                host_binding_id: TransferHostBindingId::new("binding:target").unwrap(),
                document_id: DomainId::new("app.layout").unwrap(),
                revision: TransferRevision::new(6),
                container_id: LayoutContainerId::new("container:target").unwrap(),
                region_id: RegionId::new("center").unwrap(),
            },
        )],
    );

    let value = serde_json::to_value(request).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 6);
    for required in [
        "protocol_version",
        "request_id",
        "client_id",
        "client_epoch",
        "generation",
        "zones",
    ] {
        assert!(object.contains_key(required));
    }
    for forbidden in [
        "window_id",
        "window_outer_bounds",
        "lifetime",
        "expires_at",
        "screen_bounds",
    ] {
        assert!(!object.contains_key(forbidden));
    }
}

#[test]
fn strict_wire_deserialization_rejects_future_version_and_spoof_fields() {
    let valid = json!({
        "protocol_version": 1,
        "request_id": "request:start",
        "panel_instance_id": "instance:panel",
    });
    assert!(serde_json::from_value::<PanelSessionStartRequest>(valid.clone()).is_ok());

    let mut future = valid.clone();
    future["protocol_version"] = json!(2);
    assert!(serde_json::from_value::<PanelSessionStartRequest>(future).is_err());

    let mut spoofed = valid;
    spoofed["window_id"] = json!("window:other");
    assert!(serde_json::from_value::<PanelSessionStartRequest>(spoofed).is_err());
}

fn request_id(value: &str) -> TransferRequestId {
    TransferRequestId::new(value).unwrap()
}
