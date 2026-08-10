use longhorn_surfaces::{SurfaceResolutionInput, resolve_surfaces};

use super::support::*;

#[test]
fn loophole_shape_resolves_multiple_surfaces_across_multiple_windows() {
    let result = resolve_surfaces(
        limits(),
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
fn external_layout_bindings_remain_opaque_and_payload_free() {
    let result = resolve_surfaces(
        limits(),
        &loophole_document(),
        &SurfaceResolutionInput::new([surface_id("surface:mix")], [window_id("window:main")]),
    )
    .unwrap();
    let value = serde_json::to_value(result).unwrap();
    let encoded = value.to_string();

    // Card 179: the resolution payload names the Surface, and a Surface is the
    // layout, so "regions" is no longer evidence of a layout leak here -- the
    // resolution result still carries none.
    assert!(encoded.contains("surface:mix"));
    assert!(!encoded.contains("panels"));
    assert!(!encoded.contains("display"));
    assert!(!encoded.contains("geometry"));
    assert!(!encoded.contains("predicate"));
}
