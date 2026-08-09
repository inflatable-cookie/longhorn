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
fn query_only_split_shell_fixture_has_no_subscription_or_service_feature() {
    let split_shell = receipt(
        host("host:split-shell-desktop", BridgeHostForm::TauriLocal),
        "session:split-shell",
        &["request_reply"],
        vec![capabilities("split-shell.workspace", &["query"]).unwrap()],
        vec![authority(
            "split-shell.workspace",
            "scope:split-shell-workspace",
            ReadAuthority::Authoritative,
            WriteAuthority::None,
            ExecutionAuthority::None,
            1,
            Some(12),
        )],
    )
    .unwrap();

    let feature_names: Vec<_> = split_shell
        .transport_features()
        .iter()
        .map(TransportFeatureId::as_str)
        .collect();
    assert_eq!(feature_names, ["request_reply"]);
    assert_eq!(
        split_shell.domain_capabilities()[0].capabilities()[0].as_str(),
        "query"
    );
}

#[test]
fn jetstream_fixture_requires_listener_first_snapshot_and_gap_resync() {
    let jetstream = receipt(
        host("host:jetstream-desktop", BridgeHostForm::TauriLocal),
        "session:jetstream",
        &["request_reply", "ordered_streams"],
        vec![capabilities("jetstream.snapshot", &["query", "subscribe"]).unwrap()],
        vec![authority(
            "jetstream.snapshot",
            "scope:jetstream-snapshot",
            ReadAuthority::Authoritative,
            WriteAuthority::None,
            ExecutionAuthority::None,
            2,
            Some(0),
        )],
    )
    .unwrap();
    let mut stream = BridgeStreamTracker::new(
        BridgeSessionId::new("session:jetstream").unwrap(),
        domain("jetstream.snapshot"),
    );

    assert_eq!(
        stream.classify_event(&super::support::cursor(
            "session:jetstream",
            "jetstream.snapshot",
            2,
            1,
        )),
        longhorn_bridge::BridgeStreamDecision::ResnapshotRequired
    );
    assert_eq!(
        jetstream
            .transport_features()
            .iter()
            .map(TransportFeatureId::as_str)
            .collect::<Vec<_>>(),
        ["request_reply", "ordered_streams"]
    );
}

#[test]
fn soundcheck_service_absence_preserves_local_domain_authority() {
    let soundcheck = receipt(
        host("host:soundcheck-desktop", BridgeHostForm::TauriLocal),
        "session:soundcheck",
        &["request_reply", "ordered_streams", "job_execution"],
        vec![
            capabilities("soundcheck.local-config", &["query", "mutate"]).unwrap(),
            capabilities("soundcheck.local-window", &["query", "mutate"]).unwrap(),
            capabilities("soundcheck.local-settings", &["query", "mutate"]).unwrap(),
            capabilities("soundcheck.analysis", &["query", "start_job", "cancel_job"]).unwrap(),
        ],
        vec![
            authority(
                "soundcheck.local-config",
                "scope:soundcheck-local-config",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(6),
            ),
            authority(
                "soundcheck.local-window",
                "scope:soundcheck-local-window",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(7),
            ),
            authority(
                "soundcheck.local-settings",
                "scope:soundcheck-local-settings",
                ReadAuthority::Authoritative,
                WriteAuthority::Authoritative,
                ExecutionAuthority::None,
                3,
                Some(8),
            ),
            authority_with_availability(
                "soundcheck.analysis",
                "scope:soundcheck-analysis",
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

    for local in &soundcheck.domain_authorities()[..3] {
        assert_eq!(local.availability(), DomainAvailability::Available);
        assert_eq!(local.write_authority(), WriteAuthority::Authoritative);
    }
    assert_eq!(
        soundcheck.domain_authorities()[3].availability(),
        DomainAvailability::Offline
    );
}

#[test]
fn nucleus_fixture_supports_embedded_and_optional_service_hosts() {
    let build = |form, host_id, session_id| {
        receipt(
            host(host_id, form),
            session_id,
            &["request_reply", "job_execution"],
            vec![capabilities("nucleus.indexing", &["query", "start_job"]).unwrap()],
            vec![authority(
                "nucleus.indexing",
                "scope:nucleus-indexing",
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
        "host:nucleus-embedded",
        "session:nucleus-embedded",
    );
    let optional_service = build(
        BridgeHostForm::LocalService,
        "host:nucleus-service",
        "session:nucleus-service",
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
fn loophole_host_form_switch_preserves_domain_identity() {
    let build = |form, host_id, session_id| {
        receipt(
            host(host_id, form),
            session_id,
            &["request_reply", "ordered_streams"],
            vec![capabilities("loophole.workspace", &["query", "mutate", "subscribe"]).unwrap()],
            vec![authority(
                "loophole.workspace",
                "scope:loophole-workspace",
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
        "host:loophole-local",
        "session:loophole-local",
    );
    let remote = build(
        BridgeHostForm::Remote,
        "host:loophole-remote",
        "session:loophole-remote",
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
