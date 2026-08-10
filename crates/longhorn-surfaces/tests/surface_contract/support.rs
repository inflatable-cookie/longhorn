use longhorn_core::{LayoutSchemaId, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutLimits, LayoutSchemaDefinition,
    ParticipatingWindow, RegionDefinition, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceRecord, SurfaceValidationCode, validate_document,
};

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::new(8, 4, 4, 64).unwrap()
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

pub fn surface(
    id: &str,
    label: Option<&str>,
    preferences: impl IntoIterator<Item = SurfaceHostPreference>,
) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        schema_id(),
        label.map(ToOwned::to_owned),
        [],
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
    LayoutDefinitionRegistry::new(
        LayoutLimits::new(8, 8, 8, 64, 8, 64, 16).expect("layout limits are valid"),
        [LayoutSchemaDefinition::new(
            schema_id(),
            [RegionDefinition::new(
                longhorn_core::RegionId::new("region:main").expect("region id is valid"),
                longhorn_core::RegionFamilyId::new("family:main").expect("family id is valid"),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [],
    )
    .expect("registry is valid")
}

pub fn assert_code(document: SurfaceDocument, expected: SurfaceValidationCode) {
    assert_eq!(
        validate_document(limits(), &document).unwrap_err().code(),
        expected
    );
}
