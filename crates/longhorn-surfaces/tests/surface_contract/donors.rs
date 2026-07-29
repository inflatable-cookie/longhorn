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
            .map(|surface| surface.layout_container_id().as_str())
            .collect::<Vec<_>>(),
        vec!["container:mix", "container:edit"]
    );
    assert_eq!(
        result.windows()[1].surfaces()[0]
            .layout_container_id()
            .as_str(),
        "container:plugins"
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

    assert!(encoded.contains("container:mix"));
    assert!(!encoded.contains("regions"));
    assert!(!encoded.contains("panels"));
    assert!(!encoded.contains("display"));
    assert!(!encoded.contains("geometry"));
    assert!(!encoded.contains("predicate"));
}
