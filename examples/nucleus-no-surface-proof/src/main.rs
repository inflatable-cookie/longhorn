//! Direct window-to-layout composition without optional Surface packages.

use longhorn_core::{
    LayoutContainerId, LayoutRevision, LayoutSchemaId, RegionId, ScreenPoint, ScreenSize, WindowId,
    WindowPlacement,
};
use longhorn_layout::{LayoutContainer, LayoutDocument, RegionState};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, WindowDiffInput, plan_window_diff,
};

fn main() {
    let window_id = WindowId::new("window:main").unwrap();
    let container_id = LayoutContainerId::new("container:main").unwrap();
    let layout = LayoutDocument::new(
        LayoutRevision::new(1),
        [LayoutContainer::new(
            container_id.clone(),
            LayoutSchemaId::new("schema:nucleus").unwrap(),
            [RegionState::new(
                RegionId::new("region:main").unwrap(),
                [],
                None,
                None,
            )],
            [],
        )],
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
    assert!(layout.container(&container_id).is_some());
    assert_eq!(window_id.as_str(), "window:main");
}
