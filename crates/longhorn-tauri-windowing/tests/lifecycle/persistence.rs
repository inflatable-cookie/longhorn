use std::sync::Arc;

use longhorn_core::ScaleFactor;
use longhorn_tauri_windowing::{
    CapturedDisplayAssociation, TauriWindowCaptureBackend, TauriWindowLifecycleAction,
    UniformWindowGeometryMapper, WindowCaptureBackend, WindowFlushOutcome,
};
use longhorn_windowing::WindowLifecycleEvent;
use tauri::WebviewWindowBuilder;

use super::support::{FlushMode, TestCapture, TestSink, harness, id};

#[test]
fn soundcheck_two_second_failures_are_typed_and_destroy_forgets() {
    let window_id = id("window:soundcheck");
    let sink = Arc::new(TestSink::new(FlushMode::Fail("disk full".to_string())));
    let test = harness(
        "soundcheck",
        2_000,
        Arc::new(TestCapture::repeating(&window_id)),
        sink,
    );
    let close = test
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: test.window_id.clone(),
        })
        .unwrap();
    assert!(matches!(
        close.actions(),
        [
            TauriWindowLifecycleAction::Flushed {
                outcome: WindowFlushOutcome::SinkFailed { .. },
                ..
            },
            TauriWindowLifecycleAction::UserCloseReported
        ]
    ));
    assert_eq!(test.sink.requests()[0].timeout().as_millis(), 2_000);

    let destroyed = test
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
            window_id: test.window_id.clone(),
        })
        .unwrap();
    assert!(matches!(
        destroyed.actions().last(),
        Some(TauriWindowLifecycleAction::Forgotten)
    ));
    assert!(
        test.host
            .handle_lifecycle_event(WindowLifecycleEvent::Blurred {
                window_id: test.window_id,
            })
            .is_err()
    );
}

#[test]
fn timeout_disconnect_and_unresolved_direct_capture_are_explicit() {
    let timeout_id = id("window:timeout");
    let timeout = harness(
        "timeout",
        0,
        Arc::new(TestCapture::repeating(&timeout_id)),
        Arc::new(TestSink::new(FlushMode::Timeout)),
    );
    let receipt = timeout
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: timeout.window_id,
        })
        .unwrap();
    assert!(matches!(
        receipt.actions().first(),
        Some(TauriWindowLifecycleAction::Flushed {
            outcome: WindowFlushOutcome::TimedOut,
            ..
        })
    ));

    let disconnected_id = id("window:disconnected");
    let disconnected = harness(
        "disconnected",
        1_000,
        Arc::new(TestCapture::repeating(&disconnected_id)),
        Arc::new(TestSink::new(FlushMode::Disconnect)),
    );
    let receipt = disconnected
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: disconnected.window_id,
        })
        .unwrap();
    assert!(matches!(
        receipt.actions().first(),
        Some(TauriWindowLifecycleAction::Flushed {
            outcome: WindowFlushOutcome::Disconnected,
            ..
        })
    ));

    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, "direct-capture", Default::default())
        .build()
        .unwrap();
    let capture = TauriWindowCaptureBackend::new(Arc::new(UniformWindowGeometryMapper::new(
        ScaleFactor::from_thousandths(1000).unwrap(),
    )));
    let captured =
        WindowCaptureBackend::capture(&capture, &id("window:direct"), &window, None).unwrap();
    assert!(matches!(
        captured.display(),
        CapturedDisplayAssociation::Unresolved
    ));
}

#[test]
fn shutdown_flush_is_one_sorted_aggregate_for_dynamic_windows() {
    let first = id("window:b");
    let sink = Arc::new(TestSink::new(FlushMode::Succeed));
    let test = harness("b", 1_000, Arc::new(TestCapture::repeating(&first)), sink);
    let second_window = WebviewWindowBuilder::new(&test._app, "a", Default::default())
        .build()
        .unwrap();
    test.host
        .install_window(id("window:a"), second_window, None)
        .unwrap();

    let receipt = test.host.shutdown_flush().unwrap();

    assert_eq!(receipt.flush(), Some(&WindowFlushOutcome::Succeeded));
    let requests = test.sink.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .targets()
            .iter()
            .map(|target| target.window_id().as_str())
            .collect::<Vec<_>>(),
        ["window:a", "window:b"]
    );
}
