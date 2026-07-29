use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    EmptyWindowPolicy, ParticipatingWindow, SurfaceDocument, SurfaceMutationCommand,
    SurfaceMutationEngine, SurfaceMutationOutcome, SurfaceMutationRequest,
};

use crate::support::{
    host, layout_containers, limits, loophole_document, request_id, surface, surface_id, window_id,
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
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:mix"))),
            source.window(&window_id("window:tools")).unwrap().clone(),
        ],
    );
    let receipt =
        SurfaceMutationEngine::new(limits(), &layout_containers(), EmptyWindowPolicy::Allow)
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
            surface("surface:a", "container:mix", None, [host("window:main", 0)]),
            surface(
                "surface:b",
                "container:edit",
                None,
                [host("window:main", 1)],
            ),
            surface(
                "surface:c",
                "container:plugins",
                None,
                [host("window:main", 2)],
            ),
        ],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:b")),
        )],
    );
    let containers = layout_containers();
    let engine = SurfaceMutationEngine::new(limits(), &containers, EmptyWindowPolicy::Allow);
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
    assert!(layout_containers().contains(&crate::support::container_id("container:edit")));
    assert!(matches!(
        middle.outcome(),
        SurfaceMutationOutcome::SurfaceClosed { cleanup, .. }
            if cleanup.layout_container_id() == &crate::support::container_id("container:edit")
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
