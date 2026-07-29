use longhorn_core::{DomainId, DropZoneId};
use longhorn_transfer::{
    LiveTransferWindow, PanelHostBindingKind, PanelTransferCommitRequest, PanelTransferErrorCode,
    PanelTransferOperation, TargetSelector, TransferErrorCode, commit_panel_transfer,
};

use crate::panel_transfer::support::{
    Fixture, Runtime, SOURCE_WINDOW, TARGET_ZONE, domain, main_region, options, source_bounds,
    target_container, window,
};

#[test]
fn cross_document_and_copy_attempts_consume_without_publication() {
    for (operation, document_id, expected) in [
        (
            PanelTransferOperation::Move,
            DomainId::new("layout.other").unwrap(),
            PanelTransferErrorCode::CrossDocument,
        ),
        (
            PanelTransferOperation::Copy,
            DomainId::new("layout.workspace").unwrap(),
            PanelTransferErrorCode::CopyUnsupported,
        ),
    ] {
        let fixture = Fixture::new();
        let domain = domain();
        let mut store = fixture.store();
        store.register(&domain).unwrap();
        let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
        runtime.publish_zone(document_id, 7, target_container(), main_region(), None);
        let request = runtime.commit_request(operation);

        let error = commit_panel_transfer(
            &store,
            &domain,
            &mut runtime.coordinator,
            &runtime.clock,
            &runtime.bindings,
            options(),
            request,
        )
        .unwrap_err();

        assert_eq!(error.code(), expected);
        assert!(error.session_consumed());
        assert!(!fixture.path(&domain).exists());

        let replay_request = runtime.commit_request(PanelTransferOperation::Move);
        let replay = commit_panel_transfer(
            &store,
            &domain,
            &mut runtime.coordinator,
            &runtime.clock,
            &runtime.bindings,
            options(),
            replay_request,
        )
        .unwrap_err();
        assert_eq!(replay.code(), PanelTransferErrorCode::TransferRejected);
        assert_eq!(
            replay.transfer_code(),
            Some(TransferErrorCode::SessionReplayed)
        );
        assert!(!fixture.path(&domain).exists());
    }
}

#[test]
fn missing_target_window_is_a_consumed_transfer_abort() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
    runtime.publish_default_zone();
    let request = PanelTransferCommitRequest::new(
        runtime.session_id,
        TargetSelector::ExplicitZone(DropZoneId::new(TARGET_ZONE).unwrap()),
        [LiveTransferWindow::new(
            window(SOURCE_WINDOW),
            source_bounds(),
        )],
        PanelTransferOperation::Move,
    );

    let error = commit_panel_transfer(
        &store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        options(),
        request,
    )
    .unwrap_err();
    assert_eq!(error.code(), PanelTransferErrorCode::TransferRejected);
    assert_eq!(
        error.transfer_code(),
        Some(TransferErrorCode::TargetWindowMissing)
    );
    assert!(error.session_consumed());
    assert!(!fixture.path(&domain).exists());
}

#[test]
fn changed_current_host_binding_aborts_before_layout_mutation() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::SurfaceContainer);
    runtime.publish_default_zone();
    runtime.bindings = crate::panel_transfer::support::bindings(
        PanelHostBindingKind::SurfaceContainer,
        DomainId::new("layout.other").unwrap(),
    );
    let request = runtime.commit_request(PanelTransferOperation::Move);

    let error = commit_panel_transfer(
        &store,
        &domain,
        &mut runtime.coordinator,
        &runtime.clock,
        &runtime.bindings,
        options(),
        request,
    )
    .unwrap_err();
    assert_eq!(error.code(), PanelTransferErrorCode::StaleHostBinding);
    assert!(error.session_consumed());
    assert!(!fixture.path(&domain).exists());
}
