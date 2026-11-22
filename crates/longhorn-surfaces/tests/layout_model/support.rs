use longhorn_core::{
    LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId,
    SizingSlotId, SurfaceId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutRatio, LayoutSchemaDefinition,
    PanelDefinition, PanelInstance, PanelInstancePolicy, ParticipatingWindow, PlacementSelector,
    RegionDefinition, RegionState, SizingSlotDefinition, SizingSlotState, SurfaceDocument,
    SurfaceHostPreference, SurfaceLimits, SurfaceRecord,
};

pub fn schema_id(value: &str) -> LayoutSchemaId {
    LayoutSchemaId::new(value).unwrap()
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn region_id(value: &str) -> RegionId {
    RegionId::new(value).unwrap()
}

pub fn family_id(value: &str) -> RegionFamilyId {
    RegionFamilyId::new(value).unwrap()
}

pub fn slot_id(value: &str) -> SizingSlotId {
    SizingSlotId::new(value).unwrap()
}

pub fn definition_id(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).unwrap()
}

pub fn instance_id(value: &str) -> PanelInstanceId {
    PanelInstanceId::new(value).unwrap()
}

pub fn request_id(value: &str) -> LayoutRequestId {
    LayoutRequestId::new(value).unwrap()
}

pub fn ratio(value: u32) -> LayoutRatio {
    LayoutRatio::from_millionths(value).unwrap()
}

pub fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::builder()
        .maximum_schemas(8)
        .maximum_regions_per_schema(16)
        .maximum_sizing_slots_per_schema(16)
        .maximum_panel_definitions(32)
        .maximum_surfaces(8)
        .maximum_windows(4)
        .maximum_host_preferences_per_surface(4)
        .maximum_label_bytes(64)
        .maximum_panel_instances(128)
        .maximum_panel_instances_per_region(64)
        .build()
        .unwrap()
}

pub fn workspace_schema() -> LayoutSchemaDefinition {
    LayoutSchemaDefinition::new(
        schema_id("schema:workspace"),
        [
            RegionDefinition::new(
                region_id("left"),
                family_id("activity"),
                10,
                EmptyRegionPolicy::KeepVisible,
                true,
            ),
            RegionDefinition::new(
                region_id("center"),
                family_id("workspace"),
                20,
                EmptyRegionPolicy::HideWhenEmpty,
                false,
            ),
            RegionDefinition::new(
                region_id("right"),
                family_id("workspace"),
                30,
                EmptyRegionPolicy::HideWhenEmpty,
                true,
            ),
        ],
        [
            SizingSlotDefinition::new(
                slot_id("left-width"),
                10,
                ratio(100_000),
                ratio(200_000),
                ratio(500_000),
            ),
            SizingSlotDefinition::new(
                slot_id("right-width"),
                20,
                ratio(100_000),
                ratio(250_000),
                ratio(500_000),
            ),
        ],
    )
}

pub fn chat_definition() -> PanelDefinition {
    PanelDefinition::new(
        definition_id("panel:chat"),
        [PlacementSelector::Region(region_id("center"))],
        [PlacementSelector::Family(family_id("workspace"))],
        PanelInstancePolicy::Singleton,
        true,
        true,
    )
}

pub fn tool_definition() -> PanelDefinition {
    PanelDefinition::new(
        definition_id("panel:tool"),
        [PlacementSelector::Region(region_id("right"))],
        [PlacementSelector::Family(family_id("workspace"))],
        PanelInstancePolicy::Multiple,
        true,
        true,
    )
}

pub fn activity_definition() -> PanelDefinition {
    PanelDefinition::new(
        definition_id("panel:activity"),
        [PlacementSelector::Region(region_id("left"))],
        [PlacementSelector::Family(family_id("activity"))],
        PanelInstancePolicy::OnePerSurface,
        false,
        false,
    )
}

pub fn bounded_definition() -> PanelDefinition {
    PanelDefinition::new(
        definition_id("panel:bounded"),
        [PlacementSelector::Region(region_id("center"))],
        [PlacementSelector::Family(family_id("workspace"))],
        PanelInstancePolicy::Bounded {
            maximum_per_document: 2,
            maximum_per_surface: 1,
        },
        true,
        true,
    )
}

pub fn registry() -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        limits(),
        [workspace_schema()],
        [
            chat_definition(),
            tool_definition(),
            activity_definition(),
            bounded_definition(),
        ],
    )
    .unwrap()
}

pub fn empty_surface(value: &str) -> SurfaceRecord {
    empty_surface_at(value, 1)
}

pub fn empty_surface_at(value: &str, order: u32) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(value),
        schema_id("schema:workspace"),
        None,
        [
            RegionState::new(region_id("left"), [], None, Some(false)),
            RegionState::new(region_id("center"), [], None, None),
            RegionState::new(region_id("right"), [], None, Some(false)),
        ],
        [
            SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
            SizingSlotState::new(slot_id("right-width"), ratio(250_000)),
        ],
        [SurfaceHostPreference::new(window_id("window:main"), order)],
    )
}

pub fn two_surface_document() -> SurfaceDocument {
    let source = document();
    SurfaceDocument::new(
        source.revision(),
        [
            source.surfaces()[0].clone(),
            empty_surface("surface:secondary"),
        ],
        source.panel_instances().iter().cloned(),
        source.windows().iter().cloned(),
    )
}

pub fn document() -> SurfaceDocument {
    let chat = instance_id("instance:chat");
    let activity = instance_id("instance:activity");
    let tool = instance_id("instance:tool");

    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            [
                RegionState::new(
                    region_id("left"),
                    [activity.clone()],
                    Some(activity.clone()),
                    Some(false),
                ),
                RegionState::new(
                    region_id("center"),
                    [chat.clone(), tool.clone()],
                    Some(chat.clone()),
                    None,
                ),
                RegionState::new(region_id("right"), [], None, Some(false)),
            ],
            [
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
                SizingSlotState::new(slot_id("right-width"), ratio(250_000)),
            ],
            [SurfaceHostPreference::new(window_id("window:main"), 0)],
        )],
        [
            PanelInstance::new(activity, definition_id("panel:activity")),
            PanelInstance::new(chat, definition_id("panel:chat")),
            PanelInstance::new(tool, definition_id("panel:tool")),
        ],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:primary")),
        )],
    )
}
