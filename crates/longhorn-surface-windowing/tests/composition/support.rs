use std::convert::Infallible;

use longhorn_core::{
    DisplayId, LayoutContainerId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, SurfaceId,
    SurfaceRevision, WindowId, WindowPlacement,
};
use longhorn_display::{
    DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayIdAllocator, DisplayInventory,
    KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay, StrongDisplayKey,
    reconcile_displays,
};
use longhorn_surfaces::{
    ParticipatingWindow, SurfaceDocument, SurfaceHostPreference, SurfaceLimits, SurfaceRecord,
};
use longhorn_windowing::{
    PlacementPolicy, WindowPlacementConfig, WindowPlacementResolution, WindowRole,
    resolve_window_placement,
};

pub struct DisplayFixture {
    id: DisplayId,
    facts: DisplayFacts,
}

struct NeverAllocator;

impl DisplayIdAllocator for NeverAllocator {
    type Error = Infallible;

    fn allocate(&mut self, observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
        panic!("unexpected allocation for {}", observation.observation_id());
    }
}

pub fn limits() -> SurfaceLimits {
    SurfaceLimits::new(8, 4, 4, 64).unwrap()
}

pub fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

pub fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).unwrap()
}

pub fn document() -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::new(12),
        [
            SurfaceRecord::new(
                surface_id("surface:mix"),
                container_id("container:mix"),
                Some("Mix".to_owned()),
                [
                    SurfaceHostPreference::new(window_id("window:preferred"), 0),
                    SurfaceHostPreference::new(window_id("window:main"), 1),
                ],
            ),
            SurfaceRecord::new(
                surface_id("surface:edit"),
                container_id("container:edit"),
                Some("Edit".to_owned()),
                [SurfaceHostPreference::new(window_id("window:main"), 0)],
            ),
        ],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:edit"))),
            ParticipatingWindow::new(
                window_id("window:preferred"),
                Some(surface_id("surface:mix")),
            ),
        ],
    )
}

pub fn display(id: &str, x: i32, width: u32, main: bool) -> DisplayFixture {
    let bounds = ScreenRect::new(ScreenPoint::new(x, 0), ScreenSize::new(width, 900));
    DisplayFixture {
        id: DisplayId::new(id).unwrap(),
        facts: DisplayFacts::new(
            None,
            main,
            if main {
                DisplayBuiltinStatus::BuiltIn
            } else {
                DisplayBuiltinStatus::External
            },
            bounds,
            bounds,
            ScaleFactor::from_thousandths(1000).unwrap(),
        ),
    }
}

pub fn inventory(fixtures: &[DisplayFixture]) -> DisplayInventory {
    let known = fixtures.iter().map(|fixture| {
        KnownDisplay::new(
            fixture.id.clone(),
            fixture.facts.clone(),
            DisplayEvidence::new().with_strong_key(strong_key(fixture.id.as_str())),
        )
    });
    let registry = KnownDisplayRegistry::from_displays(known).unwrap();
    let observed = fixtures.iter().enumerate().map(|(index, fixture)| {
        ObservedDisplay::new(
            ObservationId::new(format!("observation-{index}")).unwrap(),
            fixture.facts.clone(),
            DisplayEvidence::new().with_strong_key(strong_key(fixture.id.as_str())),
        )
    });
    reconcile_displays(&registry, observed, &mut NeverAllocator)
        .unwrap()
        .inventory()
        .clone()
}

pub fn resolve(
    window: &str,
    role: WindowRole,
    home: &str,
    inventory: &DisplayInventory,
) -> WindowPlacementResolution {
    let config = WindowPlacementConfig::new(
        window_id(window),
        role,
        WindowPlacement::new(ScreenPoint::new(50, 50), ScreenSize::new(800, 600)),
    )
    .with_home_display(Some(DisplayId::new(home).unwrap()));
    resolve_window_placement(
        &config,
        inventory,
        PlacementPolicy::new(ScreenSize::new(200, 150), ScreenSize::new(1, 1)),
    )
    .unwrap()
}

fn strong_key(value: &str) -> StrongDisplayKey {
    StrongDisplayKey::new("surface-windowing-test", value).unwrap()
}
