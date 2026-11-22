use longhorn_core::{LayoutRequestId, SurfaceRevision};
use longhorn_surfaces::{
    EmptyWindowPolicy, LayoutMutationCommand, LayoutMutationEngine, LayoutMutationRequest,
    PanelInstance, ParticipatingWindow, RegionState, SurfaceDocument, SurfaceMutationCommand,
    SurfaceMutationEngine, SurfaceMutationOutcome, SurfaceMutationRequest, SurfaceRecord,
    validate_document,
};

use crate::support::{
    host, loophole_document, panel_definition_id, panel_instance_id, region_id, registry,
    request_id, schema_id, surface, surface_id, window_id,
};

fn request(revision: u64, suffix: &str, command: SurfaceMutationCommand) -> SurfaceMutationRequest {
    SurfaceMutationRequest::new(
        request_id(&format!("request:{suffix}")),
        SurfaceRevision::new(revision),
        command,
    )
}

#[test]
fn move_changes_primary_host_and_selects_exact_source_fallback() {
    let mut source = loophole_document();
    source = SurfaceDocument::new(
        source.revision(),
        source.surfaces().iter().cloned(),
        [],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:mix"))),
            source.window(&window_id("window:tools")).unwrap().clone(),
        ],
    );
    let receipt = SurfaceMutationEngine::new(&registry(), EmptyWindowPolicy::Allow)
        .apply(
            &source,
            &request(
                11,
                "move",
                SurfaceMutationCommand::MoveSurface {
                    surface_id: surface_id("surface:mix"),
                    target_window_id: window_id("window:tools"),
                    insertion_index: 1,
                },
            ),
        )
        .unwrap();

    let committed = receipt.authoritative_document();
    assert_eq!(
        committed
            .surface(&surface_id("surface:mix"))
            .unwrap()
            .host_preferences()[0]
            .window_id(),
        &window_id("window:tools")
    );
    assert_eq!(
        committed
            .window(&window_id("window:main"))
            .unwrap()
            .active_surface_id(),
        Some(&surface_id("surface:edit"))
    );
    assert_eq!(
        committed
            .window(&window_id("window:tools"))
            .unwrap()
            .active_surface_id(),
        Some(&surface_id("surface:plugins"))
    );
}

#[test]
fn close_uses_former_index_then_previous_final_and_only_returns_cleanup_intent() {
    let source = SurfaceDocument::new(
        SurfaceRevision::new(20),
        [
            surface("surface:a", None, [host("window:main", 0)]),
            surface("surface:b", None, [host("window:main", 1)]),
            surface("surface:c", None, [host("window:main", 2)]),
        ],
        [],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:b")),
        )],
    );
    let containers = registry();
    let engine = SurfaceMutationEngine::new(&containers, EmptyWindowPolicy::Allow);
    let middle = engine
        .apply(
            &source,
            &request(
                20,
                "close-middle",
                SurfaceMutationCommand::CloseSurface {
                    surface_id: surface_id("surface:b"),
                },
            ),
        )
        .unwrap();
    assert_eq!(
        middle.authoritative_document().windows()[0].active_surface_id(),
        Some(&surface_id("surface:c"))
    );
    assert!(
        middle
            .authoritative_document()
            .surface(&surface_id("surface:b"))
            .is_none()
    );
    // Card 179: closing a Surface no longer hands back a container to clean up,
    // because the Surface was the container.
    assert!(matches!(
        middle.outcome(),
        SurfaceMutationOutcome::SurfaceClosed { surface_id: closed }
            if closed == &surface_id("surface:b")
    ));

    let final_close = engine
        .apply(
            &source,
            &request(
                20,
                "close-final",
                SurfaceMutationCommand::CloseSurface {
                    surface_id: surface_id("surface:c"),
                },
            ),
        )
        .unwrap();
    assert_eq!(
        final_close.authoritative_document().windows()[0].active_surface_id(),
        Some(&surface_id("surface:b"))
    );
}

#[test]
fn close_surface_holding_panels_removes_its_panel_instances_and_keeps_layout_valid() {
    let reg = registry();
    let panel_a = panel_instance_id("panel:a");
    let panel_b = panel_instance_id("panel:b");
    let source = SurfaceDocument::new(
        SurfaceRevision::new(5),
        [
            SurfaceRecord::new(
                surface_id("surface:a"),
                schema_id(),
                Some("A".to_owned()),
                [RegionState::new(
                    region_id(),
                    [panel_a.clone()],
                    Some(panel_a.clone()),
                    None,
                )],
                [],
                [host("window:main", 0)],
            ),
            SurfaceRecord::new(
                surface_id("surface:b"),
                schema_id(),
                Some("B".to_owned()),
                [RegionState::new(
                    region_id(),
                    [panel_b.clone()],
                    Some(panel_b.clone()),
                    None,
                )],
                [],
                [host("window:main", 1)],
            ),
        ],
        [
            PanelInstance::new(panel_a.clone(), panel_definition_id("panel:tool")),
            PanelInstance::new(panel_b.clone(), panel_definition_id("panel:tool")),
        ],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:a")),
        )],
    );

    let closed = SurfaceMutationEngine::new(&reg, EmptyWindowPolicy::Allow)
        .apply(
            &source,
            &request(
                5,
                "close-with-panels",
                SurfaceMutationCommand::CloseSurface {
                    surface_id: surface_id("surface:a"),
                },
            ),
        )
        .unwrap();
    let after_close = closed.authoritative_document();
    validate_document(&reg, after_close).unwrap();
    assert!(after_close.panel_instance(&panel_a).is_none());
    assert!(after_close.panel_instance(&panel_b).is_some());

    // Subsequent layout mutation on the committed document must succeed.
    let layout_receipt = LayoutMutationEngine::new(&reg)
        .apply(
            after_close,
            &LayoutMutationRequest::new(
                LayoutRequestId::new("request:create-after-close").unwrap(),
                after_close.revision(),
                LayoutMutationCommand::CreatePanel {
                    panel_instance_id: panel_instance_id("panel:c"),
                    panel_definition_id: panel_definition_id("panel:tool"),
                    surface_id: surface_id("surface:b"),
                    region_id: region_id(),
                    insertion_index: 1,
                },
            ),
        )
        .unwrap();
    validate_document(&reg, layout_receipt.authoritative_document()).unwrap();
    assert_eq!(
        layout_receipt
            .authoritative_document()
            .panel_instances()
            .len(),
        2
    );
}

#[test]
fn move_active_surface_uses_primary_index_not_all_member_index() {
    // window:main declares three members:
    //   order 0: surface:fallback (primary in window:tools, fallback in window:main)
    //   order 1: surface:first    (primary in window:main) -> primary index 0
    //   order 2: surface:second   (primary in window:main) -> primary index 1
    //   order 3: surface:third    (primary in window:main) -> primary index 2
    // Moving surface:first (active in window:main) must select surface:second (primary index 0
    // after removal), NOT surface:third (all-member index 1).
    let reg = registry();
    let source = SurfaceDocument::new(
        SurfaceRevision::new(1),
        [
            surface(
                "surface:fallback",
                None,
                [host("window:tools", 0), host("window:main", 0)],
            ),
            surface(
                "surface:first",
                None,
                [host("window:main", 1), host("window:tools", 1)],
            ),
            surface("surface:second", None, [host("window:main", 2)]),
            surface("surface:third", None, [host("window:main", 3)]),
        ],
        [],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:first"))),
            ParticipatingWindow::new(
                window_id("window:tools"),
                Some(surface_id("surface:fallback")),
            ),
        ],
    );

    let moved = SurfaceMutationEngine::new(&reg, EmptyWindowPolicy::Allow)
        .apply(
            &source,
            &request(
                1,
                "move-primary-index",
                SurfaceMutationCommand::MoveSurface {
                    surface_id: surface_id("surface:first"),
                    target_window_id: window_id("window:tools"),
                    insertion_index: 1,
                },
            ),
        )
        .unwrap();
    assert_eq!(
        moved
            .authoritative_document()
            .window(&window_id("window:main"))
            .unwrap()
            .active_surface_id(),
        Some(&surface_id("surface:second"))
    );
}
