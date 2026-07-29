use longhorn_surfaces::{SurfaceResolutionInput, SurfaceUnresolvedReason, resolve_surfaces};

use super::super::support::*;

#[test]
fn preferred_and_fallback_resolution_table_is_exact() {
    let document = loophole_document();

    let preferred = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [
                surface_id("surface:mix"),
                surface_id("surface:edit"),
                surface_id("surface:plugins"),
            ],
            [window_id("window:main"), window_id("window:tools")],
        ),
    )
    .unwrap();
    assert_eq!(
        preferred.windows()[0]
            .surfaces()
            .iter()
            .map(|surface| (
                surface.surface_id().as_str(),
                surface.host_preference_index()
            ))
            .collect::<Vec<_>>(),
        vec![("surface:mix", 0), ("surface:edit", 0)]
    );
    assert_eq!(
        preferred.windows()[0]
            .active_surface_id()
            .map(|id| id.as_str()),
        Some("surface:edit")
    );
    assert_eq!(
        preferred.windows()[1]
            .surfaces()
            .iter()
            .map(|surface| surface.surface_id().as_str())
            .collect::<Vec<_>>(),
        vec!["surface:plugins"]
    );

    let fallback = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [
                surface_id("surface:plugins"),
                surface_id("surface:edit"),
                surface_id("surface:mix"),
            ],
            [window_id("window:tools")],
        ),
    )
    .unwrap();
    assert_eq!(
        fallback.windows()[0]
            .surfaces()
            .iter()
            .map(|surface| (
                surface.surface_id().as_str(),
                surface.host_preference_index()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("surface:edit", 1),
            ("surface:mix", 1),
            ("surface:plugins", 0)
        ]
    );
    assert_eq!(
        fallback.windows()[0]
            .active_surface_id()
            .map(|id| id.as_str()),
        Some("surface:plugins")
    );
}

#[test]
fn presence_and_missing_windows_are_typed_without_mutation() {
    let document = loophole_document();
    let before = document.clone();

    let absent = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [surface_id("surface:mix"), surface_id("surface:plugins")],
            [window_id("window:main"), window_id("window:tools")],
        ),
    )
    .unwrap();
    assert_eq!(
        absent.unresolved_surfaces()[0].surface_id().as_str(),
        "surface:edit"
    );
    assert_eq!(
        absent.unresolved_surfaces()[0].reason(),
        SurfaceUnresolvedReason::NotAdmitted
    );
    assert_eq!(
        absent.windows()[0]
            .active_surface_id()
            .map(|id| id.as_str()),
        Some("surface:mix")
    );

    let unavailable = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [
                surface_id("surface:mix"),
                surface_id("surface:edit"),
                surface_id("surface:plugins"),
            ],
            [],
        ),
    )
    .unwrap();
    assert!(unavailable.windows().is_empty());
    assert!(
        unavailable
            .unresolved_surfaces()
            .iter()
            .all(|surface| { surface.reason() == SurfaceUnresolvedReason::NoAvailableWindow })
    );
    assert_eq!(document, before);
}

#[test]
fn structural_and_external_input_permutations_produce_one_snapshot() {
    let document = loophole_document();
    let permuted = longhorn_surfaces::SurfaceDocument::new(
        document.revision(),
        document.surfaces().iter().rev().cloned(),
        document.windows().iter().rev().cloned(),
    );
    let first = resolve_surfaces(
        limits(),
        &document,
        &SurfaceResolutionInput::new(
            [
                surface_id("surface:mix"),
                surface_id("surface:edit"),
                surface_id("surface:plugins"),
            ],
            [window_id("window:main"), window_id("window:tools")],
        ),
    )
    .unwrap();
    let second = resolve_surfaces(
        limits(),
        &permuted,
        &SurfaceResolutionInput::new(
            [
                surface_id("surface:plugins"),
                surface_id("surface:edit"),
                surface_id("surface:mix"),
            ],
            [window_id("window:tools"), window_id("window:main")],
        ),
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(
        serde_json::from_slice::<longhorn_surfaces::SurfaceResolution>(
            &serde_json::to_vec(&first).unwrap()
        )
        .unwrap(),
        first
    );
}
