use longhorn_bridge::{
    AuthenticationPosture, AuthorityEpoch, AuthorityRevision, BridgeConnectionState,
    BridgeHelloRequest, BridgeHostForm, BridgeNegotiationErrorCode, DomainAuthorityDescriptor,
    DomainAvailability, ExecutionAuthority, ReadAuthority, WriteAuthority,
};
use longhorn_core::{AuthorityScopeId, BridgeId};

use super::support::{authority, capabilities, domain, host, receipt};

#[test]
fn capability_connection_and_authentication_do_not_grant_authority() {
    let request = BridgeHelloRequest::new(
        BridgeId::new("bridge:renderer").unwrap(),
        vec![domain("example.workspace")],
    )
    .unwrap();
    let negotiated = receipt(
        host("host:local", BridgeHostForm::Direct),
        "session:one",
        &["request_reply"],
        vec![capabilities("example.workspace", &["query"]).unwrap()],
        Vec::new(),
    )
    .unwrap();

    negotiated.validate_for(&request).unwrap();
    assert_eq!(
        negotiated.connection().state(),
        BridgeConnectionState::Ready
    );
    assert_eq!(
        negotiated.authentication(),
        AuthenticationPosture::NotRequired
    );
    assert_eq!(negotiated.domain_capabilities().len(), 1);
    assert!(negotiated.domain_authorities().is_empty());
}

#[test]
fn execution_only_authority_does_not_imply_write_authority() {
    let execution = authority(
        "nucleus.indexing",
        "scope:nucleus-indexing",
        ReadAuthority::None,
        WriteAuthority::None,
        ExecutionAuthority::Executor,
        7,
        None,
    );
    let negotiated = receipt(
        host("host:nucleus-embedded", BridgeHostForm::Direct),
        "session:nucleus",
        &["request_reply", "job_execution"],
        vec![capabilities("nucleus.indexing", &["start_job"]).unwrap()],
        vec![execution],
    )
    .unwrap();

    let authority = &negotiated.domain_authorities()[0];
    assert_eq!(
        authority.execution_authority(),
        ExecutionAuthority::Executor
    );
    assert_eq!(authority.write_authority(), WriteAuthority::None);
}

#[test]
fn one_authority_scope_cannot_declare_multiple_writers() {
    let first = authority(
        "example.documents",
        "scope:project",
        ReadAuthority::Authoritative,
        WriteAuthority::Authoritative,
        ExecutionAuthority::None,
        1,
        Some(3),
    );
    let second = authority(
        "example.settings",
        "scope:project",
        ReadAuthority::Authoritative,
        WriteAuthority::Authoritative,
        ExecutionAuthority::None,
        1,
        Some(8),
    );
    let error = receipt(
        host("host:local", BridgeHostForm::Direct),
        "session:duplicate-writer",
        &["request_reply"],
        vec![
            capabilities("example.documents", &["query", "mutate"]).unwrap(),
            capabilities("example.settings", &["query", "mutate"]).unwrap(),
        ],
        vec![first, second],
    )
    .unwrap_err();

    assert_eq!(error.code(), BridgeNegotiationErrorCode::MultipleWriters);
}

#[test]
fn offline_and_revision_authority_combinations_are_rejected() {
    let offline_writer = DomainAuthorityDescriptor::new(
        domain("example.workspace"),
        AuthorityScopeId::new("scope:workspace").unwrap(),
        DomainAvailability::Offline,
        ReadAuthority::None,
        WriteAuthority::Authoritative,
        ExecutionAuthority::None,
        AuthorityEpoch::new(1).unwrap(),
        None,
    )
    .unwrap_err();
    assert_eq!(
        offline_writer.code(),
        BridgeNegotiationErrorCode::InvalidAuthorityDescriptor
    );

    let projection_revision = DomainAuthorityDescriptor::new(
        domain("example.workspace"),
        AuthorityScopeId::new("scope:workspace").unwrap(),
        DomainAvailability::Available,
        ReadAuthority::Projection,
        WriteAuthority::None,
        ExecutionAuthority::None,
        AuthorityEpoch::new(1).unwrap(),
        Some(AuthorityRevision::new(4)),
    )
    .unwrap_err();
    assert_eq!(
        projection_revision.code(),
        BridgeNegotiationErrorCode::InvalidAuthorityDescriptor
    );
}
