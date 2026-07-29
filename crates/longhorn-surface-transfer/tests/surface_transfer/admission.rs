use longhorn_core::{SurfaceId, TransferHostBindingId};
use longhorn_surface_transfer::{
    SurfaceHostBinding, SurfaceHostBindings, SurfaceSessionAdmission, SurfaceTransferErrorCode,
    admit_surface_session,
};
use longhorn_transfer::{ClientEpoch, TransferDuration};

use super::support::{Fixture, RuntimeFixture, domain, window_id};

#[test]
fn admission_uses_fresh_primary_host_and_allocates_one_session() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let mut runtime = RuntimeFixture::new();

    let session = runtime.admit(&fixture.store, &domain).unwrap();
    assert_eq!(session.to_string(), "01010101010101010101010101010101");
}

#[test]
fn stale_binding_and_non_primary_source_fail_before_session_creation() {
    let mut fixture = Fixture::new();
    let domain = domain();
    fixture.store.register(&domain).unwrap();
    let mut runtime = RuntimeFixture::new();
    let stale = SurfaceHostBindings::new([SurfaceHostBinding::new(
        TransferHostBindingId::new("binding:source").unwrap(),
        window_id("window:target"),
        domain.descriptor().id().clone(),
    )])
    .unwrap();
    let request = SurfaceSessionAdmission::new(
        window_id("window:main"),
        longhorn_core::TransferClientId::new("client:main").unwrap(),
        ClientEpoch::new(1),
        SurfaceId::new("surface:a").unwrap(),
        TransferHostBindingId::new("binding:source").unwrap(),
        TransferDuration::new(40),
    );
    let error = admit_surface_session(
        &fixture.store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &mut runtime.allocator,
        &stale,
        request,
    )
    .unwrap_err();
    assert_eq!(error.code(), SurfaceTransferErrorCode::StaleHostBinding);
    assert!(!error.session_consumed());
}
