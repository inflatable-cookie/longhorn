use longhorn_bridge::{
    BridgeHostForm, BridgeNegotiationErrorCode, BridgeStreamTracker, DomainAvailability,
    ExecutionAuthority, ReadAuthority, WriteAuthority,
};
use longhorn_core::{BridgeSessionId, TransportFeatureId};

use super::support::{authority, authority_with_availability, capabilities, domain, host, receipt};

#[test]
fn absent_domains_remain_absent_and_extra_domains_are_rejected() {
    let request = longhorn_bridge::BridgeHelloRequest::new(
        longhorn_core::BridgeId::new("bridge:test").unwrap(),
        vec![domain("example.present"), domain("example.absent")],
    )
    .unwrap();
    let partial = receipt(
        host("host:test", BridgeHostForm::Direct),
        "session:partial",
        &["request_reply"],
        vec![capabilities("example.present", &["query"]).unwrap()],
        Vec::new(),
    )
    .unwrap();
    partial.validate_for(&request).unwrap();
    assert_eq!(partial.domain_capabilities().len(), 1);

    let extra = receipt(
        host("host:test", BridgeHostForm::Direct),
        "session:extra",
        &["request_reply"],
        vec![capabilities("example.extra", &["query"]).unwrap()],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        extra.validate_for(&request).unwrap_err().code(),
        BridgeNegotiationErrorCode::UnrequestedDomain
    );
}

#[test]
fn query_only_receipt_has_no_subscription_or_service_feature() {
    let query_only = receipt(
        host("host:query-only-desktop", BridgeHostForm::TauriLocal),
        "session:query-only",
        &["request_reply"],
        vec![capabilities("query-only.workspace", &["query"]).unwrap()],
        vec![authority(
            "query-only.workspace",
            "scope:query-only-workspace",
            ReadAuthority::Authoritative,
            WriteAuthority::None,
            ExecutionAuthority::None,
            1,
            Some(12),
        )],
    )
    .unwrap();

    let feature_names: Vec<_> = query_only
        .transport_features()
        .iter()
        .map(TransportFeatureId::as_str)
        .collect();
    assert_eq!(feature_names, ["request_reply"]);
    assert_eq!(
        query_only.domain_capabilities()[0].capabilities()[0].as_str(),
        "query"
    );
}

#[test]
fn ordered_receipt_requires_listener_first_snapshot_and_gap_resync() {
    let ordered = receipt(
        host("host:ordered-desktop", BridgeHostForm::TauriLocal),
        "session:ordered",
        &["request_reply", "ordered_streams"],
        vec![capabilities("example.snapshot", &["query", "subscribe"]).unwrap()],
        vec![authority(
            "example.snapshot",
            "scope:snapshot",
            ReadAuthority::Authoritative,
            WriteAuthority::None,
            ExecutionAuthority::None,
            2,
            Some(0),
        )],
    )
    .unwrap();
    let mut stream = BridgeStreamTracker::new(
        BridgeSessionId::new("session:ordered").unwrap(),
        domain("example.snapshot"),
    );

    assert_eq!(
        stream.classify_event(&super::support::cursor(
            "session:ordered",
            "example.snapshot",
            2,
            1,
        )),
        longhorn_bridge::BridgeStreamDecision::ResnapshotRequired
    );
    assert_eq!(
        ordered
            .transport_features()
            .iter()
            .map(TransportFeatureId::as_str)
            .collect::<Vec<_>>(),
        ["request_reply", "ordered_streams"]
    );
}

#[test]
fn service_absence_preserves_local_domain_authority() {
    let local_first = receipt(
        host("host:local-first-desktop", BridgeHostForm::TauriLocal),
        "session:local-first",
        &["request_reply", "ordered_streams", "job_execution"],
        vec![
            capabilities("example.local-config", &["query", "mutate"]).unwrap(),
            capabilities("example.local-window", &["query", "mutate"]).unwrap(),
            capabilities("example.local-settings", &["query", "mutate"]).unwrap(),
            capabilities("example.analysis", &["query", "start_job", "cancel_job"]).unwrap(),
        ],
        vec![
            authority(
                "example.local-config",
                "scope:local-config",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(6),
            ),
            authority(
                "example.local-window",
                "scope:local-window",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(7),
            ),
            authority(
                "example.local-settings",
                "scope:local-settings",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(8),
            ),
            authority_with_availability(
                "example.analysis",
                "scope:analysis",
                DomainAvailability::Offline,
                ReadAuthority::None,
                WriteAuthority::None,
                ExecutionAuthority::None,
                3,
                None,
            ),
        ],
    )
    .unwrap();

    for local in &local_first.domain_authorities()[..3] {
        assert_eq!(local.availability(), DomainAvailability::Available);
        assert_eq!(local.write_authority(), WriteAuthority::Authoritative);
    }
    assert_eq!(
        local_first.domain_authorities()[3].availability(),
        DomainAvailability::Offline
    );
}

#[test]
fn receipts_support_embedded_and_optional_service_hosts() {
    let build = |form, host_id, session_id| {
        receipt(
            host(host_id, form),
            session_id,
            &["request_reply", "job_execution"],
            vec![capabilities("example.indexing", &["query", "start_job"]).unwrap()],
            vec![authority(
                "example.indexing",
                "scope:indexing",
                ReadAuthority::Projection,
                WriteAuthority::None,
                ExecutionAuthority::Executor,
                4,
                None,
            )],
        )
        .unwrap()
    };
    let embedded = build(
        BridgeHostForm::Direct,
        "host:indexing-embedded",
        "session:indexing-embedded",
    );
    let optional_service = build(
        BridgeHostForm::LocalService,
        "host:indexing-service",
        "session:indexing-service",
    );

    assert_eq!(embedded.host().form, BridgeHostForm::Direct);
    assert_eq!(optional_service.host().form, BridgeHostForm::LocalService);
    assert_ne!(
        embedded.host().host_instance_id,
        optional_service.host().host_instance_id
    );
    assert_ne!(embedded.session_id(), optional_service.session_id());
    assert_eq!(
        embedded.domain_authorities()[0].domain_id(),
        optional_service.domain_authorities()[0].domain_id()
    );
}

#[test]
fn host_form_switch_preserves_domain_identity() {
    let build = |form, host_id, session_id| {
        receipt(
            host(host_id, form),
            session_id,
            &["request_reply", "ordered_streams"],
            vec![capabilities("example.workspace", &["query", "mutate", "subscribe"]).unwrap()],
            vec![authority(
                "example.workspace",
                "scope:workspace",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::Executor,
                9,
                Some(82),
            )],
        )
        .unwrap()
    };
    let local = build(
        BridgeHostForm::LocalFirst,
        "host:switch-local",
        "session:switch-local",
    );
    let remote = build(
        BridgeHostForm::Remote,
        "host:switch-remote",
        "session:switch-remote",
    );

    assert_ne!(local.host().form, remote.host().form);
    assert_eq!(
        local.domain_capabilities()[0].domain_id(),
        remote.domain_capabilities()[0].domain_id()
    );
    assert_eq!(
        local.domain_authorities()[0].scope_id(),
        remote.domain_authorities()[0].scope_id()
    );
}
