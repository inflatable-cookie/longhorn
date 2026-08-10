use longhorn_config::LoadOutcome;
use longhorn_surfaces::LayoutMutationOutcome;
use longhorn_transfer::{
    PanelHostBindingKind, PanelTransferErrorCode, PanelTransferOperation, TransferErrorCode,
    commit_panel_transfer,
};

use super::support::{
    Fixture, Runtime, domain, main_region, options, source_container, target_container, tool_panel,
};

#[test]
fn direct_and_surface_container_shapes_commit_the_same_authoritative_move() {
    for kind in [
        PanelHostBindingKind::DirectWindow,
        PanelHostBindingKind::SurfaceContainer,
    ] {
        let fixture = Fixture::new();
        let domain = domain();
        let mut store = fixture.store();
        store.register(&domain).unwrap();
        let mut runtime = Runtime::admit(&store, &domain, kind);
        runtime.publish_default_zone();
        let request = runtime.commit_request(PanelTransferOperation::Move);

        let receipt = commit_panel_transfer(
            &store,
            &domain,
            &mut runtime.coordinator,
            &runtime.clock,
            &runtime.bindings,
            options(),
            request,
        )
        .unwrap();

        assert_eq!(receipt.source_binding_kind(), kind);
        assert_eq!(receipt.target_binding_kind(), kind);
        assert_eq!(receipt.publication().layout().committed_revision().get(), 8);
        assert_eq!(
            receipt.publication().layout().outcome(),
            &LayoutMutationOutcome::PanelMoved {
                panel_instance_id: tool_panel(),
                source_container_id: source_container(),
                source_region_id: main_region(),
                former_index: 0,
                target_container_id: target_container(),
                target_region_id: main_region(),
                insertion_index: 0,
            }
        );
        assert_committed_document(receipt.publication().layout().authoritative_document());

        let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
            panic!("committed layout should load");
        };
        assert_eq!(
            loaded.value,
            *receipt.publication().layout().authoritative_document()
        );

        let before = std::fs::read(fixture.path(&domain)).unwrap();
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
        assert_eq!(std::fs::read(fixture.path(&domain)).unwrap(), before);
    }
}

fn assert_committed_document(document: &longhorn_surfaces::LayoutDocument) {
    assert_eq!(document.revision().get(), 8);
    assert!(
        document
            .container(&source_container())
            .unwrap()
            .region(&main_region())
            .unwrap()
            .panel_instance_ids()
            .is_empty()
    );
    assert_eq!(
        document
            .container(&target_container())
            .unwrap()
            .region(&main_region())
            .unwrap()
            .panel_instance_ids(),
        &[tool_panel()]
    );
}
