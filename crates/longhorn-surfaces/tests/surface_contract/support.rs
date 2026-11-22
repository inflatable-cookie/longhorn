use longhorn_core::{
    LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId, SurfaceId,
    SurfaceRequestId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition,
    PanelInstancePolicy, ParticipatingWindow, PlacementSelector, RegionDefinition, RegionState,
    SurfaceDocument, SurfaceHostPreference, SurfaceLimits, SurfaceRecord, SurfaceValidationCode,
    validate_document,
};

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::builder()
        .maximum_schemas(8)
        .maximum_regions_per_schema(8)
        .maximum_sizing_slots_per_schema(8)
        .maximum_panel_definitions(64)
        .maximum_surfaces(8)
        .maximum_windows(4)
        .maximum_host_preferences_per_surface(4)
        .maximum_label_bytes(64)
        .maximum_panel_instances(64)
        .maximum_panel_instances_per_region(16)
        .build()
        .unwrap()
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn request_id(value: &str) -> SurfaceRequestId {
    SurfaceRequestId::new(value).unwrap()
}

pub fn host(window: &str, order: u32) -> SurfaceHostPreference {
    SurfaceHostPreference::new(window_id(window), order)
}

pub fn schema_id() -> LayoutSchemaId {
    LayoutSchemaId::new("schema:loophole").expect("schema id is valid")
}

pub fn region_id() -> RegionId {
    RegionId::new("region:main").expect("region id is valid")
}

pub fn panel_definition_id(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).unwrap()
}

pub fn panel_instance_id(value: &str) -> PanelInstanceId {
    PanelInstanceId::new(value).unwrap()
}

pub fn surface(
    id: &str,
    label: Option<&str>,
    preferences: impl IntoIterator<Item = SurfaceHostPreference>,
) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        schema_id(),
        label.map(ToOwned::to_owned),
        [RegionState::new(region_id(), [], None, None)],
        [],
        preferences,
    )
}

pub fn loophole_document() -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::new(11),
        [
            surface(
                "surface:mix",
                Some("Mix"),
                [host("window:main", 0), host("window:tools", 1)],
            ),
            surface(
                "surface:edit",
                Some("Edit"),
                [host("window:main", 1), host("window:tools", 0)],
            ),
            surface("surface:plugins", None, [host("window:tools", 2)]),
        ],
        [],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:edit"))),
            ParticipatingWindow::new(
                window_id("window:tools"),
                Some(surface_id("surface:plugins")),
            ),
        ],
    )
}

pub fn registry() -> LayoutDefinitionRegistry {
    registry_with_limits(limits())
}

pub fn registry_with_limits(limits: SurfaceLimits) -> LayoutDefinitionRegistry {
    LayoutDefinitionRegistry::new(
        limits,
        [LayoutSchemaDefinition::new(
            schema_id(),
            [RegionDefinition::new(
                region_id(),
                RegionFamilyId::new("family:main").expect("family id is valid"),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [PanelDefinition::new(
            panel_definition_id("panel:tool"),
            [PlacementSelector::Region(region_id())],
            [PlacementSelector::Region(region_id())],
            PanelInstancePolicy::Multiple,
            true,
            true,
        )],
    )
    .expect("registry is valid")
}

pub fn assert_code(document: SurfaceDocument, expected: SurfaceValidationCode) {
    assert_eq!(
        validate_document(&registry(), &document)
            .unwrap_err()
            .code(),
        expected
    );
}
