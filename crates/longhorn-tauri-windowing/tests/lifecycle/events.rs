use std::sync::{Arc, atomic::Ordering};

use longhorn_core::{
    PhysicalPoint, PhysicalSize as LonghornPhysicalSize, ScaleFactor, ScreenPoint, ScreenSize,
};
use longhorn_tauri_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, ProgrammaticApplyObserver,
    TauriWindowLifecycleAction, UniformWindowGeometryMapper, WindowGeometryMapper,
    WindowScaleGeometryMapper, translate_tauri_window_event,
};
use longhorn_windowing::{ApplyGeneration, WindowLifecycleEvent, WindowOperation};
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindowBuilder, WindowEvent};

use super::support::{FlushMode, TestCapture, TestSink, harness, id, placement};

#[test]
fn native_event_translation_maps_geometry_and_blur() {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, "translate", Default::default())
        .build()
        .unwrap();
    let window_id = id("window:translate");
    let mapper = UniformWindowGeometryMapper::new(ScaleFactor::from_thousandths(1000).unwrap());

    assert!(matches!(
        translate_tauri_window_event(
            &window_id,
            &window,
            &WindowEvent::Moved(PhysicalPosition::new(12, 34)),
            &mapper,
        )
        .unwrap(),
        Some(WindowLifecycleEvent::Moved { outer_origin, .. })
            if outer_origin == ScreenPoint::new(12, 34)
    ));
    assert!(matches!(
        translate_tauri_window_event(
            &window_id,
            &window,
            &WindowEvent::Resized(PhysicalSize::new(640, 480)),
            &mapper,
        )
        .unwrap(),
        Some(WindowLifecycleEvent::Resized { inner_size, .. })
            if inner_size == ScreenSize::new(640, 480)
    ));
    assert!(matches!(
        translate_tauri_window_event(&window_id, &window, &WindowEvent::Focused(false), &mapper,)
            .unwrap(),
        Some(WindowLifecycleEvent::Blurred { .. })
    ));
}

#[test]
fn live_window_scale_prevents_double_sized_capture_geometry() {
    let mapper = WindowScaleGeometryMapper;
    let scale = ScaleFactor::from_thousandths(2_000).unwrap();

    assert_eq!(
        mapper
            .map_outer_origin(PhysicalPoint::new(200, 100), scale)
            .unwrap(),
        ScreenPoint::new(100, 50)
    );
    assert_eq!(
        mapper
            .map_inner_size(LonghornPhysicalSize::new(3_160, 2_026), scale)
            .unwrap(),
        ScreenSize::new(1_580, 1_013)
    );
}

#[test]
fn loophole_programmatic_geometry_is_suppressed_before_sink_mutation() {
    let window_id = id("window:loophole");
    let test = harness(
        "loophole",
        1_000,
        Arc::new(TestCapture::repeating(&window_id)),
        Arc::new(TestSink::new(FlushMode::Succeed)),
    );
    let target = placement(40, 50, 900, 700);
    ProgrammaticApplyObserver::register_apply(
        test.host.as_ref(),
        ApplyGeneration::new(7),
        &WindowOperation::Move {
            window_id: test.window_id.clone(),
            transport_handle: None,
            outer_origin: target.outer_origin(),
        },
    )
    .unwrap();

    let receipt = test
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::Moved {
            window_id: test.window_id,
            outer_origin: target.outer_origin(),
        })
        .unwrap();

    assert!(matches!(
        receipt.actions(),
        [TauriWindowLifecycleAction::Ignored { .. }]
    ));
    assert!(test.sink.staged.lock().unwrap().is_empty());
}

#[test]
fn sink_reentry_proves_stage_runs_without_coordinator_or_registry_locks() {
    let window_id = id("window:reentrant");
    let sink = Arc::new(TestSink::new(FlushMode::Succeed));
    let test = harness(
        "reentrant",
        1_000,
        Arc::new(TestCapture::repeating(&window_id)),
        sink.clone(),
    );
    let weak_host = Arc::downgrade(&test.host);
    let target_id = test.window_id.clone();
    sink.set_stage_hook(Arc::new(move || {
        let host = weak_host.upgrade().unwrap();
        ProgrammaticApplyObserver::register_apply(
            host.as_ref(),
            ApplyGeneration::new(11),
            &WindowOperation::Hide {
                window_id: target_id.clone(),
                transport_handle: None,
            },
        )
        .unwrap();
    }));

    let receipt = test
        .host
        .handle_lifecycle_event(WindowLifecycleEvent::Blurred {
            window_id: test.window_id,
        })
        .unwrap();

    assert!(matches!(
        receipt.actions().first(),
        Some(TauriWindowLifecycleAction::PlacementStaged { .. })
    ));
}

#[test]
fn nucleus_settles_captures_blur_and_uses_one_second_close_bound() {
    let window_id = id("window:nucleus");
    let test = harness(
        "nucleus",
        1_000,
        Arc::new(TestCapture::repeating(&window_id)),
        Arc::new(TestSink::new(FlushMode::Succeed)),
    );
    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Moved {
            window_id: test.window_id.clone(),
            outer_origin: ScreenPoint::new(1, 2),
        })
        .unwrap();
    let capture_wake = test.scheduler.wakes()[0].clone();
    test.clock.set(capture_wake.due_at().get());
    test.host.handle_scheduled_wake(capture_wake).unwrap();
    assert_eq!(test.sink.staged.lock().unwrap().len(), 1);

    test.clock.set(350);
    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Blurred {
            window_id: test.window_id.clone(),
        })
        .unwrap();
    assert_eq!(test.sink.staged.lock().unwrap().len(), 2);

    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: test.window_id,
        })
        .unwrap();
    assert_eq!(test.user_close.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        test.sink.requests().last().unwrap().timeout().as_millis(),
        1_000
    );
}

#[test]
fn stale_generation_cannot_publish_and_failed_capture_can_retry() {
    let window_id = id("window:retry");
    let successful = CapturedWindowPlacement::new(
        window_id.clone(),
        placement(10, 20, 640, 480),
        false,
        CapturedDisplayAssociation::Unresolved,
    );
    let test = harness(
        "retry",
        1_000,
        Arc::new(TestCapture::outcomes([
            Err("probe failed".to_string()),
            Ok(successful),
        ])),
        Arc::new(TestSink::new(FlushMode::Succeed)),
    );
    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Moved {
            window_id: test.window_id.clone(),
            outer_origin: ScreenPoint::new(1, 1),
        })
        .unwrap();
    let stale = test.scheduler.wakes()[0].clone();
    test.clock.set(50);
    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::Resized {
            window_id: test.window_id.clone(),
            inner_size: ScreenSize::new(500, 400),
        })
        .unwrap();
    test.clock.set(stale.due_at().get());
    let stale_receipt = test.host.handle_scheduled_wake(stale).unwrap();
    assert!(matches!(
        stale_receipt.actions(),
        [TauriWindowLifecycleAction::Ignored { .. }]
    ));
    assert!(test.sink.staged.lock().unwrap().is_empty());

    let current = test.scheduler.wakes()[1].clone();
    test.clock.set(current.due_at().get());
    let failed = test.host.handle_scheduled_wake(current).unwrap();
    assert!(matches!(
        failed.actions(),
        [TauriWindowLifecycleAction::CaptureFailed { .. }]
    ));
    test.host
        .handle_lifecycle_event(WindowLifecycleEvent::FlushRequested {
            window_id: test.window_id,
        })
        .unwrap();
    assert_eq!(test.sink.staged.lock().unwrap().len(), 1);
    assert_eq!(test.sink.requests().len(), 1);
}
