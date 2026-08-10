use longhorn_core::{
    LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionId, SurfaceRevision,
};
use longhorn_surface_windowing::compose_surface_window_plan;
use longhorn_surfaces::{PanelInstance, RegionState, SurfaceDocument, SurfaceRecord};
use longhorn_windowing::WindowRole;

use crate::support::{display, document, inventory, limits, resolve, surface_id};

#[test]
fn loophole_fixture_resolves_window_surface_region_and_panel() {
    let displays = inventory(&[
        display("display:main", 0, 1600, true),
        display("display:right", 1600, 1200, false),
    ]);
    let plan = compose_surface_window_plan(
        limits(),
        &document(),
        [surface_id("surface:mix"), surface_id("surface:edit")],
        &[
            resolve(
                "window:main",
                WindowRole::RequiredPrimary,
                "display:main",
                &displays,
            ),
            resolve(
                "window:preferred",
                WindowRole::Optional,
                "display:right",
                &displays,
            ),
        ],
        |_| true,
    )
    .unwrap();
    let preferred = &plan.windows()[1];
    let resolved_surface = &preferred.surfaces().surfaces()[0];
    let layout = registry();
    let surface = layout.surface(resolved_surface.surface_id()).unwrap();
    let region = &surface.regions()[0];
    let panel = layout
        .panel_instance(&region.panel_instance_ids()[0])
        .unwrap();

    assert_eq!(
        preferred.surfaces().window_id().as_str(),
        "window:preferred"
    );
    // Card 179: the chain is window -> Surface -> region -> panel. The
    // container link that used to sit between Surface and region is gone,
    // because the Surface is the container.
    assert_eq!(resolved_surface.surface_id().as_str(), "surface:mix");
    assert_eq!(surface.id().as_str(), "surface:mix");
    assert_eq!(region.region_id().as_str(), "region:main");
    assert_eq!(panel.id().as_str(), "panel-instance:mix");
}

fn registry() -> SurfaceDocument {
    let panel_id = PanelInstanceId::new("panel-instance:mix").unwrap();
    SurfaceDocument::new(
        SurfaceRevision::new(8),
        [SurfaceRecord::new(
            surface_id("surface:mix"),
            LayoutSchemaId::new("schema:loophole").unwrap(),
            None,
            [RegionState::new(
                RegionId::new("region:main").unwrap(),
                [panel_id.clone()],
                Some(panel_id.clone()),
                None,
            )],
            [],
            [],
        )],
        [PanelInstance::new(
            panel_id,
            PanelDefinitionId::new("panel:mix").unwrap(),
        )],
        [],
    )
}
