use longhorn_tauri_windowing::{
    PredeclaredTauriWindow, assemble_tauri_single_window_lifecycle_host,
};
use longhorn_windowing::WindowLifecycleEvent;
use tauri::WebviewWindowBuilder;

use super::support::{SinkMode, id, policy, services};

#[test]
fn lifecycle_only_helper_installs_one_predeclared_window() {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let fixture = services(SinkMode::Succeed);

    let host = assemble_tauri_single_window_lifecycle_host(
        policy(2_000),
        fixture.services,
        PredeclaredTauriWindow::new(id("window:main"), window),
    )
    .unwrap();

    assert!(
        host.handle_lifecycle_event(WindowLifecycleEvent::Blurred {
            window_id: id("window:main"),
        })
        .is_ok()
    );
}
