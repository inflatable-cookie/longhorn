use std::sync::{Arc, Mutex};

use longhorn_core::SurfaceRevision;
use longhorn_surface_windowing::shutdown_surface_window_host;
use longhorn_tauri_windowing::{
    NativeWindowCall, NoWindowFactory, PredeclaredTauriWindow, TauriWindowHostTeardownStatus,
    WindowApplyOutcome, WindowRevealStatus, assemble_tauri_window_host,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, WindowDiffDiagnostic, WindowDiffInput, WindowLifecycleDuration,
    WindowOperationKind,
};
use tauri::WebviewWindowBuilder;

use super::support::{
    RecordingBackend, SinkMode, StaticReadback, handle, id, live, placement, policy, services,
};

mod support;

use support::{SurfaceWindowFactory, surface_document, surface_plan};

#[test]
fn full_surface_plan_uses_existing_host_failure_retry_reveal_and_shutdown() {
    let source = surface_document();
    let plan = surface_plan(&source);
    let main_placement = plan.windows()[0].desired_window().placement();
    let workspace_placement = plan.windows()[1].desired_window().placement();

    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "main", Default::default())
        .visible(false)
        .build()
        .unwrap();
    let service_fixture = services(SinkMode::Succeed);
    let reveal = Arc::clone(&service_fixture.reveal);
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        service_fixture.services,
        [PredeclaredTauriWindow::new(id("window:main"), main)],
        Some(handle("main")),
    )
    .unwrap();
    let desired = plan.desired_windows().cloned().collect::<Vec<_>>();
    let missing = initialized
        .host()
        .apply(
            app.handle(),
            WindowDiffInput::new(
                desired.clone(),
                [live("window:main", "main", main_placement, false)],
                initialized.host().capabilities(false),
                ApplyGeneration::new(69),
            )
            .for_hidden_restore(),
            NoWindowFactory,
            RecordingBackend::default(),
            StaticReadback::complete([live("window:main", "main", main_placement, false)]),
        )
        .unwrap();
    assert!(
        missing
            .apply()
            .plan()
            .diagnostics()
            .iter()
            .any(|diagnostic| {
                matches!(
                    diagnostic,
                    WindowDiffDiagnostic::UnsupportedOperation {
                        operation: WindowOperationKind::Create,
                        window_id,
                        ..
                    } if window_id == &id("window:workspace")
                )
            })
    );
    assert_eq!(initialized.host().installed_window_count().unwrap(), 1);
    assert_eq!(source, surface_document());

    let apply = initialized
        .host()
        .apply(
            app.handle(),
            WindowDiffInput::new(
                desired.clone(),
                [live("window:main", "main", main_placement, false)],
                initialized.host().capabilities(true),
                ApplyGeneration::new(70),
            )
            .for_hidden_restore(),
            SurfaceWindowFactory,
            RecordingBackend::default(),
            StaticReadback::complete([
                live("window:main", "main", main_placement, false),
                live("window:workspace", "workspace", workspace_placement, false),
            ]),
        )
        .unwrap();
    assert!(apply.apply().is_converged());
    assert_eq!(initialized.host().installed_window_count().unwrap(), 2);
    assert_eq!(
        initialized
            .host()
            .mark_page_ready(&id("window:main"))
            .unwrap()
            .status(),
        &WindowRevealStatus::Revealed
    );
    assert_eq!(
        initialized
            .host()
            .mark_page_ready(&id("window:workspace"))
            .unwrap()
            .status(),
        &WindowRevealStatus::Revealed
    );
    assert_eq!(reveal.0.load(std::sync::atomic::Ordering::SeqCst), 2);

    let moved_main = placement(120, 80, 900, 650);
    let moved = desired
        .iter()
        .map(|window| {
            if window.window_id() == &id("window:main") {
                DesiredWindow::new(id("window:main"), moved_main, false, true)
            } else {
                window.clone()
            }
        })
        .collect::<Vec<_>>();
    let failed = initialized
        .host()
        .apply(
            app.handle(),
            WindowDiffInput::new(
                moved.clone(),
                [
                    live("window:main", "main", main_placement, true),
                    live("window:workspace", "workspace", workspace_placement, true),
                ],
                initialized.host().capabilities(true),
                ApplyGeneration::new(71),
            ),
            SurfaceWindowFactory,
            RecordingBackend::failing(NativeWindowCall::SetOuterPosition),
            StaticReadback::complete([
                live("window:main", "main", main_placement, true),
                live("window:workspace", "workspace", workspace_placement, true),
            ]),
        )
        .unwrap();
    assert!(failed.apply().attempts().iter().any(|attempt| matches!(
        attempt.outcome(),
        WindowApplyOutcome::Failed { failure, .. }
            if failure.call() == NativeWindowCall::SetOuterPosition
    )));
    assert_eq!(source, surface_document());

    let retry = initialized
        .host()
        .apply(
            app.handle(),
            WindowDiffInput::new(
                moved,
                [
                    live("window:main", "main", main_placement, true),
                    live("window:workspace", "workspace", workspace_placement, true),
                ],
                initialized.host().capabilities(true),
                ApplyGeneration::new(72),
            ),
            SurfaceWindowFactory,
            RecordingBackend::default(),
            StaticReadback::complete([
                live("window:main", "main", moved_main, true),
                live("window:workspace", "workspace", workspace_placement, true),
            ]),
        )
        .unwrap();
    assert!(retry.apply().is_converged());

    let order = Mutex::new(Vec::new());
    let shutdown = shutdown_surface_window_host(
        WindowLifecycleDuration::from_millis(900),
        |timeout| {
            order.lock().unwrap().push("surface");
            assert_eq!(timeout.as_millis(), 900);
            Ok::<_, &'static str>(source.revision())
        },
        || {
            order.lock().unwrap().push("window");
            initialized.host().teardown()
        },
    )
    .unwrap();
    assert_eq!(order.into_inner().unwrap(), ["surface", "window"]);
    assert_eq!(*shutdown.surface(), SurfaceRevision::new(12));
    assert_eq!(
        shutdown.window().status(),
        TauriWindowHostTeardownStatus::TornDown
    );
}
