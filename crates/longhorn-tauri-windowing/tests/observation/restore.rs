use longhorn_core::{
    DisplayId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_display::{
    DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayIdAllocator, KnownDisplayRegistry,
    ObservationId, ObservedDisplay,
};
use longhorn_tauri_windowing::{DesktopObservation, plan_window_restore_from_observation};
use longhorn_windowing::{
    PlacementPolicy, PlacementReason, SavedDisplayAssociation, SavedDisplayEvidence,
    SavedWindowPlacement, WindowPlacementResolution, WindowRole,
};

struct ObservationAllocator;

impl DisplayIdAllocator for ObservationAllocator {
    type Error = std::convert::Infallible;

    fn allocate(&mut self, observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
        Ok(DisplayId::new(format!("display:{}", observation.observation_id())).unwrap())
    }
}

fn display(id: &str, bounds: ScreenRect, main: bool) -> ObservedDisplay {
    ObservedDisplay::new(
        ObservationId::new(id).unwrap(),
        DisplayFacts::new(
            None,
            main,
            DisplayBuiltinStatus::Unknown,
            bounds,
            bounds,
            ScaleFactor::from_thousandths(1_000).unwrap(),
        ),
        DisplayEvidence::new(),
    )
}

#[test]
fn one_entry_point_reconciles_displays_and_restores_saved_evidence() {
    let side = ScreenRect::new(ScreenPoint::new(1_600, 0), ScreenSize::new(1_200, 900));
    let saved = SavedWindowPlacement::new(
        WindowId::new("window:main").unwrap(),
        WindowPlacement::new(ScreenPoint::new(1_700, 40), ScreenSize::new(900, 700)),
        false,
        SavedDisplayAssociation::new(
            None,
            Some(SavedDisplayEvidence::new(
                side,
                side,
                ScaleFactor::from_thousandths(1_000).unwrap(),
            )),
        ),
    );
    let observation = DesktopObservation::new(
        vec![
            display(
                "main",
                ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(1_600, 900)),
                true,
            ),
            display("side", side, false),
        ],
        Vec::new(),
    );

    let restore = plan_window_restore_from_observation(
        &saved,
        &KnownDisplayRegistry::new(),
        &observation,
        &mut ObservationAllocator,
        WindowRole::RequiredPrimary,
        PlacementPolicy::new(ScreenSize::new(100, 100), ScreenSize::new(80, 80)),
    )
    .unwrap();

    assert_eq!(restore.reconciliation().registry().len(), 2);
    let WindowPlacementResolution::Resolved(placement) = restore.placement() else {
        panic!("placement should resolve");
    };
    assert_eq!(
        placement.target_display_id(),
        &DisplayId::new("display:side").unwrap()
    );
    assert_eq!(placement.reason(), &PlacementReason::ConfiguredHome);
}
