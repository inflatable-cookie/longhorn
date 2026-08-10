//! Direct window-to-layout composition with no Surface chrome.
//!
//! Card 179 removed the container, so the no-Surface shape is what it always
//! was underneath: exactly one Surface, unlabelled, never presented as a tab.

use longhorn_core::{
    DomainId, LayoutSchemaId, RegionId, ScreenPoint, ScreenSize, SurfaceId, SurfaceRevision,
    TransferHostBindingId, WindowId, WindowPlacement,
};
use longhorn_surfaces::{RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceRecord};
use longhorn_transfer::{PanelHostBinding, PanelHostBindings};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, WindowDiffInput, plan_window_diff,
};

fn main() {
    let window_id = WindowId::new("window:main").unwrap();
    let surface_id = SurfaceId::new("surface:main").unwrap();
    let layout = SurfaceDocument::new(
        SurfaceRevision::new(1),
        [SurfaceRecord::new(
            surface_id.clone(),
            LayoutSchemaId::new("schema:nucleus").unwrap(),
            None,
            [RegionState::new(
                RegionId::new("region:main").unwrap(),
                [],
                None,
                None,
            )],
            [],
            [SurfaceHostPreference::new(window_id.clone(), 0)],
        )],
        [],
        [],
    );
    let desired = DesiredWindow::new(
        window_id.clone(),
        WindowPlacement::new(ScreenPoint::new(0, 0), ScreenSize::new(1000, 700)),
        false,
        true,
    );
    let host_plan = plan_window_diff(&WindowDiffInput::new(
        [desired],
        [],
        HostCapabilities::none(),
        ApplyGeneration::new(1),
    ))
    .unwrap();

    assert_eq!(host_plan.generation(), ApplyGeneration::new(1));
    assert!(layout.surface(&surface_id).is_some());
    let _transfer_bindings = PanelHostBindings::new([PanelHostBinding::direct_window(
        TransferHostBindingId::new("binding:main").unwrap(),
        window_id.clone(),
        DomainId::new("layout.workspace").unwrap(),
        surface_id,
    )])
    .unwrap();
    assert_eq!(window_id.as_str(), "window:main");
}
