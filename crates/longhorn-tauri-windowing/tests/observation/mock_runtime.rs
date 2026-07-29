use longhorn_core::WindowId;
use longhorn_tauri_windowing::{ManagedWebviewWindow, TauriProbeError, probe_managed_windows};
use tauri::WebviewWindowBuilder;

#[test]
fn mock_runtime_observes_only_explicitly_managed_windows() {
    let app = tauri::test::mock_app();
    let managed = WebviewWindowBuilder::new(&app, "managed", Default::default())
        .build()
        .unwrap();
    let _unmanaged = WebviewWindowBuilder::new(&app, "unmanaged", Default::default())
        .build()
        .unwrap();
    let managed =
        ManagedWebviewWindow::new(Some(WindowId::new("window:managed").unwrap()), managed);

    let observed = probe_managed_windows(&[managed]).unwrap();

    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].window_id(),
        Some(&WindowId::new("window:managed").unwrap())
    );
    assert_eq!(observed[0].transport_handle().as_str(), "managed");
    assert!(observed[0].is_visible());
}

#[test]
fn failed_managed_batch_returns_no_partial_snapshot() {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, "managed", Default::default())
        .build()
        .unwrap();
    let managed = ManagedWebviewWindow::new(Some(WindowId::new("window:managed").unwrap()), window);

    assert!(matches!(
        probe_managed_windows(&[managed.clone(), managed]),
        Err(TauriProbeError::DuplicateTransportHandle(_))
    ));
}
