use longhorn_core::LayoutRevision;
use longhorn_layout::{
    EmptyRegionPolicy, LayoutContainer, LayoutDefinitionRegistry, LayoutDocument,
    LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy, PlacementSelector,
    RegionDefinition, RegionState, SizingSlotDefinition, SizingSlotState, normalize_document,
    validate_document,
};

use super::support::*;

#[test]
fn loophole_eight_region_shape_uses_only_shared_semantic_types() {
    let regions = [
        ("navigation", "edge", true),
        ("activity", "edge", true),
        ("primary", "workspace", false),
        ("secondary", "workspace", true),
        ("inspector", "edge", true),
        ("timeline", "utility", true),
        ("console", "utility", true),
        ("status", "chrome", false),
    ];
    let schema = LayoutSchemaDefinition::new(
        schema_id("schema:loophole"),
        regions
            .iter()
            .enumerate()
            .map(|(index, (id, family, collapsible))| {
                RegionDefinition::new(
                    region_id(id),
                    family_id(family),
                    u32::try_from(index).unwrap(),
                    EmptyRegionPolicy::HideWhenEmpty,
                    *collapsible,
                )
            }),
        [
            sizing("navigation-width", 0, 100_000, 250_000),
            sizing("inspector-width", 1, 100_000, 250_000),
            sizing("utility-height", 2, 100_000, 250_000),
        ],
    );
    let registry = LayoutDefinitionRegistry::new(
        limits(),
        [schema],
        [PanelDefinition::new(
            definition_id("panel:editor"),
            [PlacementSelector::Region(region_id("primary"))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Singleton,
            true,
            true,
        )],
    )
    .unwrap();
    let document = empty_document(
        "schema:loophole",
        regions
            .iter()
            .map(|(id, _, collapsible)| (*id, *collapsible)),
        ["navigation-width", "inspector-width", "utility-height"],
    );

    validate_document(&registry, &document).unwrap();
    assert_eq!(
        registry
            .schema(&schema_id("schema:loophole"))
            .unwrap()
            .regions()
            .len(),
        8
    );
    assert_eq!(normalize_document(&registry, &document).unwrap(), document);
}

#[test]
fn nucleus_five_region_shape_does_not_require_surfaces_or_windows() {
    let regions = [
        ("left", "activity", false),
        ("center_top", "workspace", false),
        ("center_bottom", "workspace", true),
        ("right_top", "workspace", true),
        ("right_bottom", "workspace", true),
    ];
    let schema = LayoutSchemaDefinition::new(
        schema_id("schema:nucleus"),
        regions
            .iter()
            .enumerate()
            .map(|(index, (id, family, collapsible))| {
                RegionDefinition::new(
                    region_id(id),
                    family_id(family),
                    u32::try_from(index).unwrap(),
                    EmptyRegionPolicy::HideWhenEmpty,
                    *collapsible,
                )
            }),
        [
            sizing("left-center", 0, 200_000, 200_000),
            sizing("center-right", 1, 200_000, 740_000),
            sizing("center-stack", 2, 200_000, 740_000),
            sizing("right-stack", 3, 200_000, 740_000),
        ],
    );
    let registry = LayoutDefinitionRegistry::new(
        limits(),
        [schema],
        [PanelDefinition::new(
            definition_id("panel:tasks"),
            [PlacementSelector::Region(region_id("center_top"))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::OnePerContainer,
            true,
            true,
        )],
    )
    .unwrap();
    let document = empty_document(
        "schema:nucleus",
        regions
            .iter()
            .map(|(id, _, collapsible)| (*id, *collapsible)),
        ["left-center", "center-right", "center-stack", "right-stack"],
    );

    validate_document(&registry, &document).unwrap();
    assert_eq!(
        registry
            .schema(&schema_id("schema:nucleus"))
            .unwrap()
            .sizing_slots()
            .len(),
        4
    );
}

fn sizing(id: &str, order: u32, minimum: u32, default: u32) -> SizingSlotDefinition {
    SizingSlotDefinition::new(
        slot_id(id),
        order,
        ratio(minimum),
        ratio(default),
        ratio(900_000),
    )
}

fn empty_document<'a>(
    schema: &str,
    regions: impl IntoIterator<Item = (&'a str, bool)>,
    sizing_slots: impl IntoIterator<Item = &'a str>,
) -> LayoutDocument {
    LayoutDocument::new(
        LayoutRevision::INITIAL,
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id(schema),
            regions.into_iter().map(|(id, collapsible)| {
                RegionState::new(region_id(id), [], None, collapsible.then_some(false))
            }),
            sizing_slots
                .into_iter()
                .map(|id| SizingSlotState::new(slot_id(id), ratio(250_000))),
        )],
        [],
    )
}
