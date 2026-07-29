use longhorn_core::WindowId;
use longhorn_tauri_windowing::{
    ApplyReadback, HostProbeOperation, ManagedWindowRegistryError, NativeWindowCall,
    NoWindowFactory, PredeclaredTauriWindow, TauriObservationError, TauriProbeError,
    TauriWindowFactory, TauriWindowHostError, TauriWindowHostInitializationError,
    WindowApplyOutcome, WindowFactoryError, WindowFlushOutcome, assemble_tauri_window_host,
};
use longhorn_windowing::WindowLifecycleEvent;
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use super::support::{
    RecordingBackend, SinkMode, StaticReadback, desired, handle, id, input, live, placement,
    policy, services,
};

struct FailingFactory;

impl TauriWindowFactory<MockRuntime> for FailingFactory {
    fn can_create(&self) -> bool {
        true
    }

    fn create(
        &mut self,
        _app: &AppHandle<MockRuntime>,
        _window_id: &WindowId,
    ) -> Result<WebviewWindow<MockRuntime>, WindowFactoryError> {
        Err(WindowFactoryError::Failed {
            detail: "injected factory failure".to_string(),
        })
    }
}

#[test]
fn initialization_rejects_duplicate_stable_identity_before_listeners() {
    let app = tauri::test::mock_app();
    let first = WebviewWindowBuilder::new(&app, "first", Default::default())
        .build()
        .unwrap();
    let second = WebviewWindowBuilder::new(&app, "second", Default::default())
        .build()
        .unwrap();
    let error = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        services(SinkMode::Succeed).services,
        [
            PredeclaredTauriWindow::new(id("window:duplicate"), first),
            PredeclaredTauriWindow::new(id("window:duplicate"), second),
        ],
        None,
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        TauriWindowHostInitializationError::Registry {
            source: ManagedWindowRegistryError::DuplicateWindowId(window_id)
        } if window_id == id("window:duplicate")
    ));
}

#[test]
fn apply_faults_keep_generation_identity_handle_and_exact_stage() {
    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        services(SinkMode::Succeed).services,
        [PredeclaredTauriWindow::new(id("window:main"), main)],
        Some(handle("main")),
    )
    .unwrap();
    let original = placement(0, 0, 400, 300);
    let target = placement(50, 60, 800, 600);
    let probe_error = TauriObservationError::Probe(TauriProbeError::Host {
        operation: HostProbeOperation::OuterPosition,
        handle: Some(handle("main")),
        detail: "injected probe failure".to_string(),
    });
    let receipt = initialized
        .host()
        .apply(
            app.handle(),
            input(
                [desired("window:main", target, true)],
                [live("window:main", "main", original, true)],
                41,
            ),
            NoWindowFactory,
            RecordingBackend::failing(NativeWindowCall::SetOuterPosition),
            StaticReadback::failed(probe_error.clone()),
        )
        .unwrap();
    let failed = receipt
        .apply()
        .attempts()
        .iter()
        .find(|attempt| matches!(attempt.outcome(), WindowApplyOutcome::Failed { .. }))
        .unwrap();
    assert_eq!(failed.generation().get(), 41);
    assert_eq!(failed.window_id(), &id("window:main"));
    assert_eq!(failed.transport_handle(), Some(&handle("main")));
    assert!(matches!(
        failed.outcome(),
        WindowApplyOutcome::Failed { failure, .. }
            if failure.call() == NativeWindowCall::SetOuterPosition
    ));
    assert_eq!(
        receipt.apply().readback(),
        &ApplyReadback::Failed(probe_error)
    );
}

#[test]
fn planning_and_factory_failures_leave_the_host_reusable() {
    let app = tauri::test::mock_app();
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        services(SinkMode::Succeed).services,
        [],
        None,
    )
    .unwrap();
    let target = placement(0, 0, 500, 400);
    let planning = initialized.host().apply(
        app.handle(),
        input(
            [
                desired("window:duplicate", target, true),
                desired("window:duplicate", target, true),
            ],
            [],
            50,
        ),
        FailingFactory,
        RecordingBackend::default(),
        StaticReadback::complete([]),
    );
    assert!(matches!(planning, Err(TauriWindowHostError::Apply(_))));

    let receipt = initialized
        .host()
        .apply(
            app.handle(),
            input([desired("window:new", target, true)], [], 51),
            FailingFactory,
            RecordingBackend::default(),
            StaticReadback::complete([]),
        )
        .unwrap();
    let failed = &receipt.apply().attempts()[0];
    assert_eq!(failed.generation().get(), 51);
    assert_eq!(failed.window_id(), &id("window:new"));
    assert_eq!(failed.transport_handle(), None);
    assert!(matches!(
        failed.outcome(),
        WindowApplyOutcome::Failed { failure, .. }
            if failure.call() == NativeWindowCall::FactoryCreate
    ));
}

#[test]
fn event_sink_and_flush_failures_remain_typed_through_teardown() {
    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let service_fixture = services(SinkMode::RequestFail("sink offline".to_string()));
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(1_500),
        service_fixture.services,
        [PredeclaredTauriWindow::new(id("window:main"), main)],
        Some(handle("main")),
    )
    .unwrap();
    assert!(matches!(
        initialized
            .host()
            .handle_lifecycle_event(WindowLifecycleEvent::Blurred {
                window_id: id("window:missing")
            }),
        Err(TauriWindowHostError::Lifecycle(
            longhorn_tauri_windowing::TauriWindowLifecycleError::UnknownWindow { .. }
        ))
    ));

    let close = initialized
        .host()
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: id("window:main"),
        })
        .unwrap();
    assert!(close.actions().iter().any(|action| matches!(
        action,
        longhorn_tauri_windowing::TauriWindowLifecycleAction::Flushed {
            outcome: WindowFlushOutcome::RequestFailed { .. },
            ..
        }
    )));
    let teardown = initialized.host().teardown().unwrap();
    assert!(matches!(
        teardown.shutdown().and_then(|receipt| receipt.flush()),
        Some(WindowFlushOutcome::RequestFailed { .. })
    ));
}
