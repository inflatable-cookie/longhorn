use longhorn_core::LayoutSchemaId;
use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    EmptyWindowPolicy, ParticipatingWindow, SurfaceDocument, SurfaceMutationCommand,
    SurfaceMutationEngine, SurfaceMutationRejectionCode, SurfaceMutationRequest,
};

use crate::support::{
    host, limits, loophole_document, registry, request_id, schema_id, surface, surface_id,
    window_id,
};

fn reject(
    source: &SurfaceDocument,
    expected: u64,
    command: SurfaceMutationCommand,
) -> SurfaceMutationRejectionCode {
    let rejection = SurfaceMutationEngine::new(limits(), &registry(), EmptyWindowPolicy::Reject)
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
        [],
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
            surface("surface:a", None, [host("window:main", 0)]),
            surface("surface:a", None, [host("window:main", 1)]),
        ],
        [],
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
                schema_id: schema_id(),
                label: None,
                host_preferences: vec![host("window:main", 2)],
            },
            SurfaceMutationRejectionCode::DuplicateSurface,
        ),
        // Card 179 retired UnknownLayoutContainer and LayoutContainerAlreadyBound.
        // What replaces them is a single check that the named schema exists.
        (
            SurfaceMutationCommand::CreateSurface {
                surface_id: surface_id("surface:new"),
                schema_id: LayoutSchemaId::new("schema:absent").unwrap(),
                label: None,
                host_preferences: vec![host("window:main", 2)],
            },
            SurfaceMutationRejectionCode::UnknownLayoutSchema,
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
        [surface("surface:only", None, [host("window:main", 0)])],
        [],
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
