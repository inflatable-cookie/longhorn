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

#[test]
fn destroyed_event_defers_flush_off_the_event_thread() {
    let window_id = id("window:deferred");
    let sink = Arc::new(TestSink::new(FlushMode::Timeout));
    let test = harness(
        "deferred",
        2_000,
        Arc::new(TestCapture::repeating(&window_id)),
        sink,
    );

    let start = std::time::Instant::now();
    let receipt = test
        .host
        .handle_tauri_event(&test.window_id, &tauri::WindowEvent::Destroyed)
        .unwrap()
        .unwrap();
    assert!(
        start.elapsed() < std::time::Duration::from_millis(1_000),
        "event-thread handling waited on the flush timeout"
    );
    assert!(
        receipt
            .actions()
            .iter()
            .any(|action| matches!(action, TauriWindowLifecycleAction::FlushDeferred { .. }))
    );
    assert!(
        !receipt
            .actions()
            .iter()
            .any(|action| matches!(action, TauriWindowLifecycleAction::Flushed { .. }))
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let reported = test.reporter.reports().iter().any(|report| {
            matches!(
                report.result(),
                Ok(receipt) if receipt.actions().iter().any(|action| matches!(
                    action,
                    TauriWindowLifecycleAction::Flushed {
                        outcome: WindowFlushOutcome::TimedOut,
                        ..
                    }
                ))
            )
        });
        if reported {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "deferred flush outcome was never reported"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[test]
fn scheduled_wake_outcomes_reach_the_reporter() {
    use longhorn_tauri_windowing::WindowLifecycleWakeHandler;

    let window_id = id("window:wake-report");
    let sink = Arc::new(TestSink::new(FlushMode::Succeed));
    let test = harness(
        "wake-report",
        1_000,
        Arc::new(TestCapture::repeating(&window_id)),
        sink,
    );

    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Moved {
            window_id: test.window_id.clone(),
            outer_origin: longhorn_core::ScreenPoint::new(5, 5),
        })
        .unwrap();
    let wake = test
        .scheduler
        .wakes()
        .first()
        .cloned()
        .expect("moved event schedules a capture wake");

    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Destroyed {
            window_id: test.window_id.clone(),
        })
        .unwrap();

    WindowLifecycleWakeHandler::handle_scheduled_wake(&*test.host, wake).unwrap_err();
    assert!(test.reporter.reports().iter().any(|report| matches!(
        report.result(),
        Err(longhorn_tauri_windowing::TauriWindowLifecycleError::UnknownWindow { .. })
    )));
}

#[test]
fn oversized_label_installation_fails_typed_with_no_partial_state() {
    let window_id = id("window:invalid-label");
    let sink = Arc::new(TestSink::new(FlushMode::Succeed));
    let test = harness(
        "valid",
        1_000,
        Arc::new(TestCapture::repeating(&window_id)),
        sink,
    );
    let long_label = "l".repeat(300);
    let window = WebviewWindowBuilder::new(&test._app, &long_label, Default::default())
        .build()
        .unwrap();

    let error = test
        .host
        .install_window(window_id.clone(), window, None)
        .unwrap_err();
    assert!(matches!(
        error,
        longhorn_tauri_windowing::TauriWindowLifecycleError::InvalidWindowLabel { .. }
    ));
    assert!(matches!(
        test.host
            .handle_lifecycle_event(WindowLifecycleEvent::Blurred { window_id })
            .unwrap_err(),
        longhorn_tauri_windowing::TauriWindowLifecycleError::UnknownWindow { .. }
    ));
}
