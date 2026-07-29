use std::convert::Infallible;

use longhorn_core::{
    DisplayId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_display::{
    DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayIdAllocator, DisplayInventory,
    KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay, StrongDisplayKey,
    reconcile_displays,
};
use longhorn_windowing::{
    ResolvedWindowPlacement, WindowPlacementConfig, WindowPlacementResolution, WindowRole,
};

#[derive(Clone)]
pub(super) struct DisplayFixture {
    id: DisplayId,
    facts: DisplayFacts,
}

struct NeverAllocator;

impl DisplayIdAllocator for NeverAllocator {
    type Error = Infallible;

    fn allocate(&mut self, observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
        panic!(
            "known fixture observation {} unexpectedly required allocation",
            observation.observation_id()
        );
    }
}

pub(super) fn display(
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    main: bool,
) -> DisplayFixture {
    display_with_work_area(
        id,
        rect(x, y, width, height),
        rect(x, y, width, height),
        main,
    )
}

pub(super) fn display_with_work_area(
    id: &str,
    full_bounds: ScreenRect,
    work_area: ScreenRect,
    main: bool,
) -> DisplayFixture {
    DisplayFixture {
        id: display_id(id),
        facts: DisplayFacts::new(
            None,
            main,
            if main {
                DisplayBuiltinStatus::BuiltIn
            } else {
                DisplayBuiltinStatus::External
            },
            full_bounds,
            work_area,
            ScaleFactor::from_thousandths(1000).unwrap(),
        ),
    }
}

pub(super) fn inventory(fixtures: &[DisplayFixture]) -> DisplayInventory {
    let known = fixtures.iter().map(|fixture| {
        let evidence = DisplayEvidence::new().with_strong_key(strong_key(fixture.id.as_str()));
        KnownDisplay::new(fixture.id.clone(), fixture.facts.clone(), evidence)
    });
    let registry = KnownDisplayRegistry::from_displays(known).unwrap();
    let observations = fixtures.iter().enumerate().map(|(index, fixture)| {
        ObservedDisplay::new(
            ObservationId::new(format!("observation-{index}")).unwrap(),
            fixture.facts.clone(),
            DisplayEvidence::new().with_strong_key(strong_key(fixture.id.as_str())),
        )
    });

    reconcile_displays(&registry, observations, &mut NeverAllocator)
        .unwrap()
        .inventory()
        .clone()
}

pub(super) fn display_id(value: &str) -> DisplayId {
    DisplayId::new(value).unwrap()
}

pub(super) fn window_id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub(super) fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn config(role: WindowRole) -> WindowPlacementConfig {
    WindowPlacementConfig::new(
        window_id("window:main"),
        role,
        placement(100, 100, 800, 600),
    )
}

pub(super) fn resolved(outcome: WindowPlacementResolution) -> ResolvedWindowPlacement {
    match outcome {
        WindowPlacementResolution::Resolved(resolved) => resolved,
        other => panic!("expected resolved placement, got {other:?}"),
    }
}

fn strong_key(value: &str) -> StrongDisplayKey {
    StrongDisplayKey::new("longhorn-windowing-test", value).unwrap()
}
