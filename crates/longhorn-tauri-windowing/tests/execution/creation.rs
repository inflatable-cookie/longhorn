use longhorn_core::WindowId;
use longhorn_tauri_windowing::{
    NativeWindowCall, TauriWindowFactory, WindowApplyFailureKind, WindowApplyOutcome,
    WindowFactoryError, execute_tauri_window_apply, tauri_host_capabilities,
};
use longhorn_windowing::{HostCapability, WindowOperationKind};
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use super::support::{RecordingBackend, StaticReadback, desired, input, live, placement, registry};

enum FactoryMode {
    Hidden,
    Visible,
    Failed,
}

struct TestFactory {
    mode: FactoryMode,
    label: &'static str,
}

impl TauriWindowFactory<MockRuntime> for TestFactory {
    fn can_create(&self) -> bool {
        true
    }

    fn create(
        &mut self,
        app: &AppHandle<MockRuntime>,
        _window_id: &WindowId,
    ) -> Result<WebviewWindow<MockRuntime>, WindowFactoryError> {
        match self.mode {
            FactoryMode::Failed => Err(WindowFactoryError::Failed {
                detail: "injected factory failure".to_string(),
            }),
            FactoryMode::Hidden => WebviewWindowBuilder::new(app, self.label, Default::default())
                .visible(false)
                .build()
                .map_err(|error| WindowFactoryError::Failed {
                    detail: error.to_string(),
                }),
            FactoryMode::Visible => WebviewWindowBuilder::new(app, self.label, Default::default())
                .build()
                .map_err(|error| WindowFactoryError::Failed {
                    detail: error.to_string(),
                }),
        }
    }

    fn validate_neutral(
        &mut self,
        _window: &WebviewWindow<MockRuntime>,
    ) -> Result<(), WindowFactoryError> {
        match self.mode {
            FactoryMode::Hidden => Ok(()),
            FactoryMode::Visible => Err(WindowFactoryError::Visible),
            FactoryMode::Failed => unreachable!(),
        }
    }
}

#[test]
fn capability_derivation_tracks_factory_availability() {
    assert!(tauri_host_capabilities(true).supports(HostCapability::Create));
    assert!(!tauri_host_capabilities(false).supports(HostCapability::Create));
    assert!(tauri_host_capabilities(false).supports(HostCapability::Move));
    assert!(tauri_host_capabilities(false).supports(HostCapability::Resize));
}

#[test]
fn injected_factory_creates_a_managed_neutral_slot() {
    let target = placement(30, 40, 700, 500);
    let (app, registry) = registry([], None);
    let fresh = live(
        Some("window:new"),
        "dynamic",
        30,
        40,
        700,
        500,
        false,
        true,
        false,
    );

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input([desired("window:new", target, false, true)], Vec::new(), 30),
        registry,
        TestFactory {
            mode: FactoryMode::Hidden,
            label: "dynamic",
        },
        RecordingBackend::default(),
        StaticReadback::new([fresh]),
    )
    .unwrap();

    assert!(outcome.receipt().is_converged());
    assert_eq!(outcome.registry().managed_windows().len(), 1);
    let create = outcome
        .receipt()
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Create)
        .unwrap();
    assert!(matches!(
        create.outcome(),
        WindowApplyOutcome::Succeeded { completed_calls }
            if completed_calls == &[
                NativeWindowCall::FactoryCreate,
                NativeWindowCall::ValidateHidden,
                NativeWindowCall::ValidateUnmaximized,
                NativeWindowCall::RegistryInsert,
            ]
    ));
}

#[test]
fn failed_or_non_neutral_factory_results_are_inspectable_and_unmanaged() {
    for (mode, expected_call) in [
        (FactoryMode::Failed, NativeWindowCall::FactoryCreate),
        (FactoryMode::Visible, NativeWindowCall::ValidateHidden),
    ] {
        let (app, registry) = registry([], None);
        let outcome = execute_tauri_window_apply(
            app.handle(),
            input(
                [desired(
                    "window:new",
                    placement(0, 0, 400, 300),
                    false,
                    true,
                )],
                Vec::new(),
                31,
            ),
            registry,
            TestFactory {
                mode,
                label: match expected_call {
                    NativeWindowCall::FactoryCreate => "factory-failed",
                    _ => "factory-visible",
                },
            },
            RecordingBackend::default(),
            StaticReadback::new(Vec::new()),
        )
        .unwrap();

        assert!(outcome.registry().managed_windows().is_empty());
        assert!(matches!(
            outcome.receipt().attempts()[0].outcome(),
            WindowApplyOutcome::Failed { failure, .. }
                if failure.kind() == WindowApplyFailureKind::Factory
                    && failure.call() == expected_call
        ));
        assert!(outcome.receipt().attempts().iter().skip(1).all(|attempt| {
            matches!(
                attempt.outcome(),
                WindowApplyOutcome::DependencySkipped {
                    blocked_by: WindowOperationKind::Create
                }
            )
        }));
    }
}
