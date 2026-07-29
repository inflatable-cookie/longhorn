use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    EmptyWindowPolicy, ParticipatingWindow, SurfaceDocument, SurfaceMutationCommand,
    SurfaceMutationEngine, SurfaceMutationRejectionCode, SurfaceMutationRequest,
};

use crate::support::{
    container_id, host, layout_containers, limits, loophole_document, request_id, surface,
    surface_id, window_id,
};

fn reject(
    source: &SurfaceDocument,
    expected: u64,
    command: SurfaceMutationCommand,
) -> SurfaceMutationRejectionCode {
    let rejection =
        SurfaceMutationEngine::new(limits(), &layout_containers(), EmptyWindowPolicy::Reject)
            .apply(
                source,
                &SurfaceMutationRequest::new(
                    request_id("request:rejected"),
                    SurfaceRevision::new(expected),
                    command,
                ),
            )
            .unwrap_err();
    assert_eq!(rejection.authoritative_document(), source);
    rejection.code()
}

#[test]
fn stale_overflow_and_invalid_current_preserve_exact_source() {
    let source = loophole_document();
    assert_eq!(
        reject(
            &source,
            10,
            SurfaceMutationCommand::RenameSurface {
                surface_id: surface_id("surface:mix"),
                label: None,
            }
        ),
        SurfaceMutationRejectionCode::StaleRevision
    );

    let overflow = SurfaceDocument::new(
        SurfaceRevision::new(u64::MAX),
        source.surfaces().iter().cloned(),
        source.windows().iter().cloned(),
    );
    assert_eq!(
        reject(
            &overflow,
            u64::MAX,
            SurfaceMutationCommand::RenameSurface {
                surface_id: surface_id("surface:mix"),
                label: None,
            }
        ),
        SurfaceMutationRejectionCode::RevisionOverflow
    );

    let invalid = SurfaceDocument::new(
        SurfaceRevision::new(1),
        [
            surface("surface:a", "container:mix", None, [host("window:main", 0)]),
            surface(
                "surface:a",
                "container:edit",
                None,
                [host("window:main", 1)],
            ),
        ],
        [ParticipatingWindow::new(window_id("window:main"), None)],
    );
    assert_eq!(
        reject(
            &invalid,
            1,
            SurfaceMutationCommand::RenameSurface {
                surface_id: surface_id("surface:a"),
                label: None,
            }
        ),
        SurfaceMutationRejectionCode::InvalidCurrentDocument
    );
}

#[test]
fn identity_container_window_order_and_empty_policy_rejections_are_typed() {
    let source = loophole_document();
    let cases = [
        (
            SurfaceMutationCommand::CreateSurface {
                surface_id: surface_id("surface:mix"),
                layout_container_id: container_id("container:new"),
                label: None,
                host_preferences: vec![host("window:main", 2)],
            },
            SurfaceMutationRejectionCode::DuplicateSurface,
        ),
        (
            SurfaceMutationCommand::CreateSurface {
                surface_id: surface_id("surface:new"),
                layout_container_id: container_id("container:missing"),
                label: None,
                host_preferences: vec![host("window:main", 2)],
            },
            SurfaceMutationRejectionCode::UnknownLayoutContainer,
        ),
        (
            SurfaceMutationCommand::CreateSurface {
                surface_id: surface_id("surface:new"),
                layout_container_id: container_id("container:mix"),
                label: None,
                host_preferences: vec![host("window:main", 2)],
            },
            SurfaceMutationRejectionCode::LayoutContainerAlreadyBound,
        ),
        (
            SurfaceMutationCommand::MoveSurface {
                surface_id: surface_id("surface:plugins"),
                target_window_id: window_id("window:main"),
                insertion_index: 0,
            },
            SurfaceMutationRejectionCode::UndeclaredTargetWindow,
        ),
        (
            SurfaceMutationCommand::ReorderWindow {
                window_id: window_id("window:main"),
                surface_ids: vec![surface_id("surface:mix")],
            },
            SurfaceMutationRejectionCode::IncompleteReorder,
        ),
    ];
    for (command, expected) in cases {
        assert_eq!(reject(&source, 11, command), expected);
    }

    let singleton = SurfaceDocument::new(
        SurfaceRevision::new(3),
        [surface(
            "surface:only",
            "container:mix",
            None,
            [host("window:main", 0)],
        )],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:only")),
        )],
    );
    assert_eq!(
        reject(
            &singleton,
            3,
            SurfaceMutationCommand::CloseSurface {
                surface_id: surface_id("surface:only")
            }
        ),
        SurfaceMutationRejectionCode::EmptyWindowNotAllowed
    );
}
