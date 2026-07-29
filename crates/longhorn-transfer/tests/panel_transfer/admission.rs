use longhorn_core::{PanelInstanceId, TransferHostBindingId, WindowId};
use longhorn_transfer::{
    ClientEpoch, PanelHostBinding, PanelHostBindingKind, PanelHostBindings, PanelSessionAdmission,
    PanelTransferErrorCode, TransferDuration, admit_panel_session,
};

use super::support::{
    Allocator, FakeClock, Fixture, SOURCE_BINDING, SOURCE_CLIENT, SOURCE_WINDOW, binding_id,
    bindings, client, coordinator, domain, domain_id, fixed_panel, source_container, tool_panel,
    window,
};

#[test]
fn admission_resolves_fresh_movable_panel_before_allocating() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let clock = FakeClock::new(10);
    let mut coordinator = coordinator(&clock);
    let bindings = bindings(PanelHostBindingKind::DirectWindow, domain_id());
    let mut allocator = Allocator::new();

    let receipt = admit_panel_session(
        &store,
        &domain,
        &mut coordinator,
        &clock,
        &mut allocator,
        &bindings,
        request(tool_panel()),
    )
    .unwrap();

    assert_eq!(allocator.calls(), 1);
    assert_eq!(coordinator.session_count(), 1);
    assert_eq!(receipt.payload().session_id().entropy(), [7; 16]);
}

#[test]
fn unknown_fixed_and_stale_bound_sources_allocate_nothing() {
    for (panel, expected) in [
        (
            PanelInstanceId::new("panel:missing").unwrap(),
            PanelTransferErrorCode::UnknownPanel,
        ),
        (fixed_panel(), PanelTransferErrorCode::PanelNotMovable),
    ] {
        let fixture = Fixture::new();
        let domain = domain();
        let mut store = fixture.store();
        store.register(&domain).unwrap();
        let clock = FakeClock::new(10);
        let mut coordinator = coordinator(&clock);
        let bindings = bindings(PanelHostBindingKind::DirectWindow, domain_id());
        let mut allocator = Allocator::new();

        let error = admit_panel_session(
            &store,
            &domain,
            &mut coordinator,
            &clock,
            &mut allocator,
            &bindings,
            request(panel),
        )
        .unwrap_err();
        assert_eq!(error.code(), expected);
        assert_eq!(allocator.calls(), 0);
        assert_eq!(coordinator.session_count(), 0);
    }

    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let clock = FakeClock::new(10);
    let mut coordinator = coordinator(&clock);
    let stale = PanelHostBindings::new([PanelHostBinding::direct_window(
        binding_id(SOURCE_BINDING),
        WindowId::new("window:other").unwrap(),
        domain_id(),
        source_container(),
    )])
    .unwrap();
    let mut allocator = Allocator::new();
    let error = admit_panel_session(
        &store,
        &domain,
        &mut coordinator,
        &clock,
        &mut allocator,
        &stale,
        request(tool_panel()),
    )
    .unwrap_err();
    assert_eq!(error.code(), PanelTransferErrorCode::StaleHostBinding);
    assert_eq!(allocator.calls(), 0);
}

#[test]
fn duplicate_binding_ids_fail_before_admission() {
    let id = TransferHostBindingId::new("binding:duplicate").unwrap();
    let binding = PanelHostBinding::direct_window(
        id,
        WindowId::new("window:one").unwrap(),
        domain_id(),
        source_container(),
    );
    let error = PanelHostBindings::new([binding.clone(), binding]).unwrap_err();
    assert_eq!(error.code(), PanelTransferErrorCode::InvalidBindingSnapshot);
}

fn request(panel_instance_id: PanelInstanceId) -> PanelSessionAdmission {
    PanelSessionAdmission::new(
        window(SOURCE_WINDOW),
        client(SOURCE_CLIENT),
        ClientEpoch::new(1),
        panel_instance_id,
        binding_id(SOURCE_BINDING),
        TransferDuration::new(40),
    )
}
