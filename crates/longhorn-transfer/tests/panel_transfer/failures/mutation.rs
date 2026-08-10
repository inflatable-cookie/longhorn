use longhorn_layout_config::publish_layout_mutation;
use longhorn_surfaces::{
    LayoutMutationCommand, LayoutMutationRejectionCode, LayoutMutationRequest,
};
use longhorn_transfer::{
    PanelHostBindingKind, PanelTransferErrorCode, PanelTransferOperation, commit_panel_transfer,
};

use crate::panel_transfer::support::{
    Fixture, Runtime, domain, domain_id, main_region, options, side_region, target_container,
    tool_panel, write_instance_policy_violation,
};

#[test]
fn missing_ineligible_and_invalid_insertion_targets_publish_nothing() {
    let cases = [
        (
            longhorn_core::RegionId::new("region:missing").unwrap(),
            None,
            PanelTransferErrorCode::TargetChanged,
            None,
        ),
        (
            side_region(),
            None,
            PanelTransferErrorCode::IneligibleTarget,
            Some(LayoutMutationRejectionCode::PanelPlacementNotAllowed),
        ),
        (
            main_region(),
            Some(1),
            PanelTransferErrorCode::InvalidInsertionPosition,
            Some(LayoutMutationRejectionCode::InvalidInsertionIndex),
        ),
    ];
    for (region_id, insertion, expected, layout_code) in cases {
        let fixture = Fixture::new();
        let domain = domain();
        let mut store = fixture.store();
        store.register(&domain).unwrap();
        let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
        runtime.publish_zone(domain_id(), 7, target_container(), region_id, insertion);
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
        assert_eq!(error.code(), expected);
        assert_eq!(error.layout_code(), layout_code);
        assert!(error.session_consumed());
        assert!(!fixture.path(&domain).exists());
    }
}

#[test]
fn stale_target_revision_aborts_before_loading_or_publication() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
    runtime.publish_zone(domain_id(), 8, target_container(), main_region(), None);
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
    assert_eq!(error.code(), PanelTransferErrorCode::StaleLayoutRevision);
    assert!(error.session_consumed());
    assert!(!fixture.path(&domain).exists());
}

#[test]
fn disappeared_source_preserves_exact_intervening_authority() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
    runtime.publish_default_zone();
    publish_layout_mutation(
        &store,
        &domain,
        options(),
        &LayoutMutationRequest::new(
            longhorn_core::LayoutRequestId::new("request:close-source").unwrap(),
            longhorn_core::LayoutRevision::new(7),
            LayoutMutationCommand::ClosePanel {
                panel_instance_id: tool_panel(),
            },
        ),
    )
    .unwrap();
    let before = std::fs::read(fixture.path(&domain)).unwrap();
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
    assert_eq!(error.code(), PanelTransferErrorCode::StaleLayoutRevision);
    assert!(error.session_consumed());
    assert_eq!(std::fs::read(fixture.path(&domain)).unwrap(), before);
}

#[test]
fn instance_policy_invalid_current_authority_is_preserved_for_recovery() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut runtime = Runtime::admit(&store, &domain, PanelHostBindingKind::DirectWindow);
    runtime.publish_default_zone();
    write_instance_policy_violation(&fixture, &domain);
    let before = std::fs::read(fixture.path(&domain)).unwrap();
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
    assert_eq!(error.code(), PanelTransferErrorCode::LayoutUnavailable);
    assert!(error.session_consumed());
    assert_eq!(std::fs::read(fixture.path(&domain)).unwrap(), before);
}
