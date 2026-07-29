use longhorn_core::{LayoutContainerId, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId};
use longhorn_surfaces::{
    LayoutContainerInventory, ParticipatingWindow, SurfaceDocument, SurfaceHostPreference,
    SurfaceLimits, SurfaceRecord, SurfaceValidationCode, validate_document,
};

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::new(8, 4, 4, 64).unwrap()
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).unwrap()
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

pub fn surface(
    id: &str,
    container: &str,
    label: Option<&str>,
    preferences: impl IntoIterator<Item = SurfaceHostPreference>,
) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        container_id(container),
        label.map(ToOwned::to_owned),
        preferences,
    )
}

pub fn loophole_document() -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::new(11),
        [
            surface(
                "surface:mix",
                "container:mix",
                Some("Mix"),
                [host("window:main", 0), host("window:tools", 1)],
            ),
            surface(
                "surface:edit",
                "container:edit",
                Some("Edit"),
                [host("window:main", 1), host("window:tools", 0)],
            ),
            surface(
                "surface:plugins",
                "container:plugins",
                None,
                [host("window:tools", 2)],
            ),
        ],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:edit"))),
            ParticipatingWindow::new(
                window_id("window:tools"),
                Some(surface_id("surface:plugins")),
            ),
        ],
    )
}

pub fn layout_containers() -> LayoutContainerInventory {
    LayoutContainerInventory::new(
        [
            "container:mix",
            "container:edit",
            "container:plugins",
            "container:new",
            "container:duplicate",
        ]
        .into_iter()
        .map(container_id),
    )
}

pub fn assert_code(document: SurfaceDocument, expected: SurfaceValidationCode) {
    assert_eq!(
        validate_document(limits(), &document).unwrap_err().code(),
        expected
    );
}
