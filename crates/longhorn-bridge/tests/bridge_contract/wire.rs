use longhorn_bridge::{
    BridgeConnectionStatus, BridgeHostForm, BridgeNegotiationReceipt, DomainAuthorityDescriptor,
    DomainCapabilityDescriptor, ExecutionAuthority, ReadAuthority, WriteAuthority,
};
use serde_json::json;

use super::support::{authority, capabilities, host, receipt};

#[test]
fn checked_receipts_round_trip_and_invalid_wire_values_fail_closed() {
    let original = receipt(
        host("host:round-trip", BridgeHostForm::Remote),
        "session:round-trip",
        &["request_reply"],
        vec![capabilities("example.workspace", &["query"]).unwrap()],
        Vec::new(),
    )
    .unwrap();
    let encoded = serde_json::to_string(&original).unwrap();
    assert_eq!(
        serde_json::from_str::<BridgeNegotiationReceipt>(&encoded).unwrap(),
        original
    );

    let duplicate_capability = json!({
        "domainId": "example.workspace",
        "capabilities": ["query", "query"]
    });
    assert!(serde_json::from_value::<DomainCapabilityDescriptor>(duplicate_capability).is_err());

    let invalid_status = json!({
        "state": "ready",
        "reason": "transportReady"
    });
    assert!(serde_json::from_value::<BridgeConnectionStatus>(invalid_status).is_err());
}

#[test]
fn authority_wire_uses_contract_camel_case_and_rejects_rust_field_names() {
    let authority = authority(
        "example.workspace",
        "scope:workspace",
        ReadAuthority::Authoritative,
        WriteAuthority::Authoritative,
        ExecutionAuthority::Executor,
        3,
        Some(8),
    );
    let encoded = serde_json::to_value(authority).unwrap();
    assert_eq!(encoded["domainId"], "example.workspace");
    assert_eq!(encoded["authorityEpoch"], 3);
    assert!(encoded.get("domain_id").is_none());
    assert!(
        serde_json::from_value::<DomainAuthorityDescriptor>(json!({
            "domain_id": "example.workspace",
            "scope_id": "scope:workspace",
            "availability": "available",
            "read_authority": "authoritative",
            "write_authority": "authoritative",
            "execution_authority": "executor",
            "authority_epoch": 3,
            "authoritative_revision": 8
        }))
        .is_err()
    );
}
