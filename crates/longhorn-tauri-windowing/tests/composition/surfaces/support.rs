use std::convert::Infallible;

use longhorn_core::{
    DisplayId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, SurfaceId, SurfaceRevision,
    WindowId, WindowPlacement,
};
use longhorn_display::{
    DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayIdAllocator, KnownDisplay,
    KnownDisplayRegistry, ObservationId, ObservedDisplay, StrongDisplayKey, reconcile_displays,
};
use longhorn_surface_windowing::{SurfaceWindowPlan, compose_surface_window_plan};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, ParticipatingWindow,
    RegionDefinition, RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceRecord,
};
use longhorn_tauri_windowing::{TauriWindowFactory, WindowFactoryError};
use longhorn_windowing::{
    PlacementPolicy, WindowPlacementConfig, WindowPlacementResolution, WindowRole,
    resolve_window_placement,
};
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use crate::composition::support::id;

pub(super) struct SurfaceWindowFactory;

impl TauriWindowFactory<MockRuntime> for SurfaceWindowFactory {
    fn can_create(&self) -> bool {
        true
    }

    fn create(
        &mut self,
        app: &AppHandle<MockRuntime>,
        window_id: &WindowId,
    ) -> Result<WebviewWindow<MockRuntime>, WindowFactoryError> {
        WebviewWindowBuilder::new(
            app,
            window_id.as_str().replace("window:", ""),
            Default::default(),
        )
        .visible(false)
        .build()
        .map_err(|error| WindowFactoryError::Failed {
            detail: error.to_string(),
        })
    }

    fn validate_neutral(
        &mut self,
        _window: &WebviewWindow<MockRuntime>,
    ) -> Result<(), WindowFactoryError> {
        Ok(())
    }
}

pub(super) fn surface_plan(document: &SurfaceDocument) -> SurfaceWindowPlan {
    let inventory = display_inventory();
    let registry = LayoutDefinitionRegistry::new(
        SurfaceLimits::builder()
            .maximum_surfaces(8)
            .maximum_windows(4)
            .maximum_host_preferences_per_surface(4)
            .maximum_label_bytes(64)
            .build()
            .unwrap(),
        [LayoutSchemaDefinition::new(
            longhorn_core::LayoutSchemaId::new("schema:composition").unwrap(),
            [RegionDefinition::new(
                longhorn_core::RegionId::new("region:main").unwrap(),
                longhorn_core::RegionFamilyId::new("family:main").unwrap(),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [],
    )
    .unwrap();
    compose_surface_window_plan(
        &registry,
        document,
        [surface_id("surface:main"), surface_id("surface:workspace")],
        &[
            resolve_placement("window:main", 20, &inventory),
            resolve_placement("window:workspace", 900, &inventory),
        ],
        |_| true,
    )
    .unwrap()
}

pub(super) fn surface_document() -> SurfaceDocument {
    let region_id = longhorn_core::RegionId::new("region:main").unwrap();
    SurfaceDocument::new(
        SurfaceRevision::new(12),
        [
            SurfaceRecord::new(
                surface_id("surface:main"),
                longhorn_core::LayoutSchemaId::new("schema:composition").unwrap(),
                None,
                [RegionState::new(region_id.clone(), [], None, None)],
                [],
                [SurfaceHostPreference::new(id("window:main"), 0)],
            ),
            SurfaceRecord::new(
                surface_id("surface:workspace"),
                longhorn_core::LayoutSchemaId::new("schema:composition").unwrap(),
                None,
                [RegionState::new(region_id, [], None, None)],
                [],
                [SurfaceHostPreference::new(id("window:workspace"), 0)],
            ),
        ],
        [],
        [
            ParticipatingWindow::new(id("window:main"), Some(surface_id("surface:main"))),
            ParticipatingWindow::new(
                id("window:workspace"),
                Some(surface_id("surface:workspace")),
            ),
        ],
    )
}

fn resolve_placement(
    window: &str,
    x: i32,
    inventory: &longhorn_display::DisplayInventory,
) -> WindowPlacementResolution {
    let placement = WindowPlacement::new(ScreenPoint::new(x, 20), ScreenSize::new(700, 500));
    let config = WindowPlacementConfig::new(id(window), WindowRole::RequiredPrimary, placement)
        .with_home_display(Some(DisplayId::new("display:main").unwrap()))
        .with_normal_placement(DisplayId::new("display:main").unwrap(), placement);
    resolve_window_placement(
        &config,
        inventory,
        PlacementPolicy::new(ScreenSize::new(200, 150), ScreenSize::new(1, 1)),
    )
    .unwrap()
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).unwrap()
}

fn display_inventory() -> longhorn_display::DisplayInventory {
    let display_id = DisplayId::new("display:main").unwrap();
    let bounds = ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(1800, 1000));
    let facts = DisplayFacts::new(
        None,
        true,
        DisplayBuiltinStatus::BuiltIn,
        bounds,
        bounds,
        ScaleFactor::from_thousandths(1000).unwrap(),
    );
    let key = StrongDisplayKey::new("surface-host-test", "main").unwrap();
    let registry = KnownDisplayRegistry::from_displays([KnownDisplay::new(
        display_id,
        facts.clone(),
        DisplayEvidence::new().with_strong_key(key.clone()),
    )])
    .unwrap();
    let observed = [ObservedDisplay::new(
        ObservationId::new("observation-main").unwrap(),
        facts,
        DisplayEvidence::new().with_strong_key(key),
    )];
    struct NeverAllocate;
    impl DisplayIdAllocator for NeverAllocate {
        type Error = Infallible;

        fn allocate(&mut self, _observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
            unreachable!()
        }
    }
    reconcile_displays(&registry, observed, &mut NeverAllocate)
        .unwrap()
        .inventory()
        .clone()
}
