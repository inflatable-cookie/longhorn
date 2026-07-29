use longhorn_surfaces::{SurfaceResolutionErrorCode, SurfaceResolutionInput, resolve_surfaces};

use super::super::support::*;

#[test]
fn malformed_external_sets_fail_typed() {
    let document = loophole_document();
    let duplicate_surface = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [surface_id("surface:mix"), surface_id("surface:mix")],
            [window_id("window:main")],
        ),
    )
    .unwrap_err();
    assert_eq!(
        duplicate_surface.code(),
        SurfaceResolutionErrorCode::DuplicateAdmittedSurface
    );

    let unknown_surface = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new([surface_id("surface:unknown")], [window_id("window:main")]),
    )
    .unwrap_err();
    assert_eq!(
        unknown_surface.code(),
        SurfaceResolutionErrorCode::UnknownAdmittedSurface
    );

    let duplicate_window = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [surface_id("surface:mix")],
            [window_id("window:main"), window_id("window:main")],
        ),
    )
    .unwrap_err();
    assert_eq!(
        duplicate_window.code(),
        SurfaceResolutionErrorCode::DuplicateAvailableWindow
    );

    let unknown_window = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new([surface_id("surface:mix")], [window_id("window:unknown")]),
    )
    .unwrap_err();
    assert_eq!(
        unknown_window.code(),
        SurfaceResolutionErrorCode::UnknownAvailableWindow
    );

    let one_surface_document = longhorn_surfaces::SurfaceDocument::new(
        longhorn_core::SurfaceRevision::INITIAL,
        [surface(
            "surface:only",
            "container:only",
            None,
            [host("window:only", 0)],
        )],
        [longhorn_surfaces::ParticipatingWindow::new(
            window_id("window:only"),
            None,
        )],
    );
    let one_each = longhorn_surfaces::SurfaceLimits::new(1, 1, 1, 16).unwrap();
    assert_eq!(
        resolve_surfaces(
            one_each,
            &one_surface_document,
            &SurfaceResolutionInput::new(
                [surface_id("surface:only"), surface_id("surface:only")],
                [window_id("window:only")]
            ),
        )
        .unwrap_err()
        .code(),
        SurfaceResolutionErrorCode::TooManyAdmittedSurfaces
    );
    assert_eq!(
        resolve_surfaces(
            one_each,
            &one_surface_document,
            &SurfaceResolutionInput::new(
                [surface_id("surface:only")],
                [window_id("window:only"), window_id("window:only")]
            ),
        )
        .unwrap_err()
        .code(),
        SurfaceResolutionErrorCode::TooManyAvailableWindows
    );

    assert!(
        serde_json::from_value::<SurfaceResolutionInput>(serde_json::json!({
            "admitted_surface_ids": [],
            "available_window_ids": [],
            "presence_predicate": {"kind": "project_open"}
        }))
        .is_err()
    );
}
