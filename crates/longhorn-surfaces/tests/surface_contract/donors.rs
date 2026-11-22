use longhorn_surfaces::{SurfaceResolutionInput, resolve_surfaces};

use super::support::*;

#[test]
fn loophole_shape_resolves_multiple_surfaces_across_multiple_windows() {
    let result = resolve_surfaces(
        &registry(),
        &loophole_document(),
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

    assert_eq!(result.windows().len(), 2);
    assert_eq!(
        result.windows()[0]
            .surfaces()
            .iter()
            .map(|surface| surface.surface_id().as_str())
            .collect::<Vec<_>>(),
        vec!["surface:mix", "surface:edit"]
    );
    assert_eq!(
        result.windows()[1].surfaces()[0].surface_id().as_str(),
        "surface:plugins"
    );
    assert!(result.unresolved_surfaces().is_empty());
}

#[test]
fn resolution_projects_only_surface_identity_schema_label_and_host_preference() {
    let result = resolve_surfaces(
        &registry(),
        &loophole_document(),
        &SurfaceResolutionInput::new([surface_id("surface:mix")], [window_id("window:main")]),
    )
    .unwrap();

    assert_eq!(result.windows().len(), 1);
    let window = &result.windows()[0];
    assert_eq!(window.window_id(), &window_id("window:main"));
    assert_eq!(window.active_surface_id(), Some(&surface_id("surface:mix")));
    assert_eq!(window.surfaces().len(), 1);
    let resolved = &window.surfaces()[0];
    assert_eq!(resolved.surface_id(), &surface_id("surface:mix"));
    assert_eq!(resolved.schema_id(), &schema_id());
    assert_eq!(resolved.label(), Some("Mix"));
    assert_eq!(resolved.host_preference_index(), 0);
}
