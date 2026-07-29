use longhorn_core::{LayoutRevision, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionId};
use longhorn_layout::{LayoutContainer, LayoutDocument, PanelInstance, RegionState};
use longhorn_surface_windowing::compose_surface_window_plan;
use longhorn_windowing::WindowRole;

use crate::support::{container_id, display, document, inventory, limits, resolve, surface_id};

#[test]
fn loophole_fixture_resolves_window_surface_container_region_and_panel() {
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
    let layout = layout_document();
    let container = layout
        .container(resolved_surface.layout_container_id())
        .unwrap();
    let region = &container.regions()[0];
    let panel = layout
        .panel_instance(&region.panel_instance_ids()[0])
        .unwrap();

    assert_eq!(
        preferred.surfaces().window_id().as_str(),
        "window:preferred"
    );
    assert_eq!(resolved_surface.surface_id().as_str(), "surface:mix");
    assert_eq!(container.id().as_str(), "container:mix");
    assert_eq!(region.region_id().as_str(), "region:main");
    assert_eq!(panel.id().as_str(), "panel-instance:mix");
}

fn layout_document() -> LayoutDocument {
    let panel_id = PanelInstanceId::new("panel-instance:mix").unwrap();
    LayoutDocument::new(
        LayoutRevision::new(8),
        [LayoutContainer::new(
            container_id("container:mix"),
            LayoutSchemaId::new("schema:loophole").unwrap(),
            [RegionState::new(
                RegionId::new("region:main").unwrap(),
                [panel_id.clone()],
                Some(panel_id.clone()),
                None,
            )],
            [],
        )],
        [PanelInstance::new(
            panel_id,
            PanelDefinitionId::new("panel:mix").unwrap(),
        )],
    )
}
