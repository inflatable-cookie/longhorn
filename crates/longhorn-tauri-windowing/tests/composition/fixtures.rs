use std::sync::{Arc, atomic::Ordering};

use longhorn_core::WindowId;
use longhorn_tauri_windowing::{
    NativeWindowCall, NoWindowFactory, PredeclaredTauriWindow, TauriWindowFactory,
    TauriWindowHostInitializationStatus, TauriWindowHostTeardownStatus, WindowApplyOutcome,
    WindowFactoryError, WindowFlushOutcome, WindowRevealStatus, assemble_tauri_window_host,
};
use longhorn_windowing::{HostCapability, WindowLifecycleEvent, WindowOperationKind};
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use super::support::{
    RecordingBackend, SinkMode, StaticReadback, desired, handle, id, input, live, placement,
    policy, services,
};

struct WorkspaceFactory;

impl TauriWindowFactory<MockRuntime> for WorkspaceFactory {
    fn can_create(&self) -> bool {
        true
    }

    fn create(
        &mut self,
        app: &AppHandle<MockRuntime>,
        window_id: &WindowId,
    ) -> Result<WebviewWindow<MockRuntime>, WindowFactoryError> {
        let label = window_id.as_str().replace("window:", "");
        WebviewWindowBuilder::new(app, label, Default::default())
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

#[test]
fn nucleus_single_window_restore_shutdown_and_repeated_init_share_one_host() {
    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "main", Default::default())
        .visible(false)
        .build()
        .unwrap();
    let service_fixture = services(SinkMode::Succeed);
    let sink = service_fixture.sink.clone();
    let reveal = service_fixture.reveal.clone();
    let initial = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        service_fixture.services,
        [PredeclaredTauriWindow::new(id("window:main"), main)],
        Some(handle("main")),
    )
    .unwrap();
    assert_eq!(
        initial.receipt().status(),
        TauriWindowHostInitializationStatus::Initialized
    );
    assert_eq!(initial.receipt().registrations().len(), 1);

    let reused = assemble_tauri_window_host(
        app.handle(),
        policy(99),
        services(SinkMode::RequestFail("unused".to_string())).services,
        [],
        None,
    )
    .unwrap();
    assert_eq!(
        reused.receipt().status(),
        TauriWindowHostInitializationStatus::Reused
    );
    assert!(Arc::ptr_eq(initial.host(), reused.host()));
    assert_eq!(initial.host().installed_window_count().unwrap(), 1);
    assert!(
        !initial
            .host()
            .capabilities(false)
            .supports(HostCapability::Create)
    );

    let original = placement(0, 0, 400, 300);
    let target = placement(40, 50, 900, 700);
    let receipt = initial
        .host()
        .apply(
            app.handle(),
            input(
                [desired("window:main", target, true)],
                [live("window:main", "main", original, false)],
                1,
            )
            .for_hidden_restore(),
            NoWindowFactory,
            RecordingBackend::default(),
            StaticReadback::complete([live("window:main", "main", target, false)]),
        )
        .unwrap();
    assert!(receipt.apply().is_converged());
    let reveal_receipt = initial.host().mark_page_ready(&id("window:main")).unwrap();
    assert_eq!(reveal_receipt.status(), &WindowRevealStatus::Revealed);
    assert_eq!(reveal.0.load(Ordering::SeqCst), 1);
    initial
        .host()
        .handle_lifecycle_event(WindowLifecycleEvent::Blurred {
            window_id: id("window:main"),
        })
        .unwrap();

    let torn_down = initial.host().teardown().unwrap();
    assert_eq!(torn_down.status(), TauriWindowHostTeardownStatus::TornDown);
    assert_eq!(torn_down.deactivated_listeners(), 1);
    assert_eq!(
        torn_down.shutdown().and_then(|receipt| receipt.flush()),
        Some(&WindowFlushOutcome::Succeeded)
    );
    assert!(sink.staged_count() >= 1);
    assert_eq!(sink.requests().len(), 1);
    assert_eq!(
        initial.host().teardown().unwrap().status(),
        TauriWindowHostTeardownStatus::AlreadyTornDown
    );
}

#[test]
fn loophole_protected_main_and_dynamic_workspace_use_the_same_assembly() {
    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "main", Default::default())
        .visible(false)
        .build()
        .unwrap();
    let service_fixture = services(SinkMode::Succeed);
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(1_000),
        service_fixture.services,
        [PredeclaredTauriWindow::new(id("window:main"), main)],
        Some(handle("main")),
    )
    .unwrap();
    assert!(
        initialized
            .host()
            .capabilities(true)
            .supports(HostCapability::Create)
    );

    let main_placement = placement(0, 0, 800, 600);
    let workspace_placement = placement(900, 20, 700, 500);
    let receipt = initialized
        .host()
        .apply(
            app.handle(),
            input(
                [
                    desired("window:main", main_placement, false),
                    desired("window:workspace-a", workspace_placement, true),
                ],
                [live("window:main", "main", main_placement, false)],
                2,
            ),
            WorkspaceFactory,
            RecordingBackend::default(),
            StaticReadback::complete([
                live("window:main", "main", main_placement, false),
                live(
                    "window:workspace-a",
                    "workspace-a",
                    workspace_placement,
                    true,
                ),
            ]),
        )
        .unwrap();
    assert!(receipt.apply().is_converged());
    assert_eq!(initialized.host().installed_window_count().unwrap(), 2);
    let create = receipt
        .apply()
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Create)
        .unwrap();
    assert!(matches!(
        create.outcome(),
        WindowApplyOutcome::Succeeded { completed_calls }
            if completed_calls.ends_with(&[
                NativeWindowCall::RegistryInsert,
                NativeWindowCall::InstallLifecycleListener,
            ])
    ));

    let close = initialized
        .host()
        .apply(
            app.handle(),
            input(
                [desired("window:workspace-a", workspace_placement, true)],
                [
                    live("window:main", "main", main_placement, false),
                    live(
                        "window:workspace-a",
                        "workspace-a",
                        workspace_placement,
                        true,
                    ),
                ],
                3,
            ),
            WorkspaceFactory,
            RecordingBackend::default(),
            StaticReadback::complete([
                live("window:main", "main", main_placement, false),
                live(
                    "window:workspace-a",
                    "workspace-a",
                    workspace_placement,
                    true,
                ),
            ]),
        )
        .unwrap();
    assert!(close.apply().attempts().iter().any(|attempt| {
        attempt.operation() == WindowOperationKind::Close
            && matches!(attempt.outcome(), WindowApplyOutcome::Failed { failure, .. }
                if failure.call() == NativeWindowCall::ProtectPrimary)
    }));
}

#[test]
fn soundcheck_minimal_close_uses_its_two_second_flush_policy() {
    let app = tauri::test::mock_app();
    let main = WebviewWindowBuilder::new(&app, "soundcheck", Default::default())
        .build()
        .unwrap();
    let service_fixture = services(SinkMode::Fail("disk full".to_string()));
    let sink = service_fixture.sink.clone();
    let user_close = service_fixture.user_close.clone();
    let initialized = assemble_tauri_window_host(
        app.handle(),
        policy(2_000),
        service_fixture.services,
        [PredeclaredTauriWindow::new(id("window:soundcheck"), main)],
        Some(handle("soundcheck")),
    )
    .unwrap();

    let receipt = initialized
        .host()
        .handle_lifecycle_event(WindowLifecycleEvent::CloseRequested {
            window_id: id("window:soundcheck"),
        })
        .unwrap();
    assert_eq!(sink.requests()[0].timeout().as_millis(), 2_000);
    assert_eq!(user_close.0.load(Ordering::SeqCst), 1);
    assert!(receipt.actions().iter().any(|action| matches!(
        action,
        longhorn_tauri_windowing::TauriWindowLifecycleAction::Flushed {
            outcome: WindowFlushOutcome::SinkFailed { .. },
            ..
        }
    )));
}
