use longhorn_core::WindowId;
use longhorn_tauri_windowing::{
    NativeWindowCall, TauriWindowFactory, WindowApplyFailureKind, WindowApplyOutcome,
    WindowFactoryError, execute_tauri_window_apply_in_place, tauri_host_capabilities,
};
use longhorn_windowing::{HostCapability, WindowOperationKind};
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use super::support::{RecordingBackend, StaticReadback, desired, input, live, placement, registry};

enum FactoryMode {
    Hidden,
    Visible,
    Failed,
}

struct TestFactory<'a> {
    mode: FactoryMode,
    label: &'a str,
}

impl TauriWindowFactory<MockRuntime> for TestFactory<'_> {
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
    let (app, mut registry) = registry([], None);
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

    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input([desired("window:new", target, false, true)], Vec::new(), 30),
        &mut registry,
        &mut TestFactory {
            mode: FactoryMode::Hidden,
            label: "dynamic",
        },
        &mut RecordingBackend::default(),
        &mut StaticReadback::new([fresh]),
    )
    .unwrap();

    assert!(receipt.is_converged());
    assert_eq!(registry.managed_windows().len(), 1);
    let create = receipt
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
        let (app, mut registry) = registry([], None);
        let receipt = execute_tauri_window_apply_in_place(
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
            &mut registry,
            &mut TestFactory {
                mode,
                label: match expected_call {
                    NativeWindowCall::FactoryCreate => "factory-failed",
                    _ => "factory-visible",
                },
            },
            &mut RecordingBackend::default(),
            &mut StaticReadback::new(Vec::new()),
        )
        .unwrap();

        assert!(registry.managed_windows().is_empty());
        assert!(matches!(
            receipt.attempts()[0].outcome(),
            WindowApplyOutcome::Failed { failure, .. }
                if failure.kind() == WindowApplyFailureKind::Factory
                    && failure.call() == expected_call
        ));
        assert!(receipt.attempts().iter().skip(1).all(|attempt| {
            matches!(
                attempt.outcome(),
                WindowApplyOutcome::DependencySkipped {
                    blocked_by: WindowOperationKind::Create
                }
            )
        }));
    }
}

#[test]
fn failed_retag_blocks_creation_of_its_source_id() {
    let target = placement(0, 0, 400, 300);
    let (app, mut registry) = registry(
        [
            (Some("window:source"), "primary"),
            (Some("window:target"), "occupied"),
        ],
        Some("primary"),
    );
    let primary = live(
        Some("window:source"),
        "primary",
        0,
        0,
        400,
        300,
        false,
        true,
        false,
    );
    let input = input(
        [
            desired("window:source", target, false, true),
            desired("window:target", target, false, true),
            desired("window:independent", target, false, true),
        ],
        [primary.clone()],
        32,
    )
    .with_protected_primary(longhorn_windowing::ProtectedPrimaryPolicy::Reuse {
        transport_handle: super::support::handle("primary"),
        window_id: super::support::id("window:target"),
    });

    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input,
        &mut registry,
        &mut TestFactory {
            mode: FactoryMode::Hidden,
            label: "independent",
        },
        &mut RecordingBackend::default(),
        &mut StaticReadback::new([primary]),
    )
    .unwrap();

    assert!(matches!(
        receipt.attempts()[0].outcome(),
        WindowApplyOutcome::Failed { failure, .. }
            if failure.call() == NativeWindowCall::RegistryRetag
    ));
    let create = receipt
        .attempts()
        .iter()
        .find(|attempt| {
            attempt.operation() == WindowOperationKind::Create
                && attempt.window_id().as_str() == "window:source"
        })
        .unwrap();
    assert!(matches!(
        create.outcome(),
        WindowApplyOutcome::DependencySkipped {
            blocked_by: WindowOperationKind::Retag
        }
    ));
    assert_eq!(registry.managed_windows().len(), 3);
}

#[test]
fn registry_rejection_discards_the_native_factory_window() {
    let (app, mut registry) = registry([], None);
    let label = "x".repeat(1024);
    let mut backend = RecordingBackend::default();
    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input(
            [desired(
                "window:new",
                placement(0, 0, 400, 300),
                false,
                true,
            )],
            [],
            33,
        ),
        &mut registry,
        &mut TestFactory {
            mode: FactoryMode::Hidden,
            label: &label,
        },
        &mut backend,
        &mut StaticReadback::new([]),
    )
    .unwrap();

    assert!(registry.managed_windows().is_empty());
    assert_eq!(backend.calls(), vec![NativeWindowCall::DiscardCreated]);
    assert!(matches!(
        receipt.attempts()[0].outcome(),
        WindowApplyOutcome::Failed { completed_calls, failure }
            if failure.call() == NativeWindowCall::RegistryInsert
                && completed_calls.last() == Some(&NativeWindowCall::DiscardCreated)
    ));
}

#[test]
fn failed_native_cleanup_is_retried_without_reopening_the_rejected_window() {
    use longhorn_tauri_windowing::{ApplyReadback, NoWindowFactory, TauriObservationError};

    let (app, mut registry) = registry([], None);
    let label = "x".repeat(1024);
    let mut refusal = RecordingBackend::failing(NativeWindowCall::DiscardCreated);
    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input(
            [desired(
                "window:new",
                placement(0, 0, 400, 300),
                false,
                true,
            )],
            [],
            34,
        ),
        &mut registry,
        &mut TestFactory {
            mode: FactoryMode::Hidden,
            label: &label,
        },
        &mut refusal,
        &mut StaticReadback::new([]),
    )
    .unwrap();
    assert!(matches!(
        receipt.readback(),
        ApplyReadback::Failed(TauriObservationError::PendingWindowDiscard(id))
            if id.as_str() == "window:new"
    ));

    let mut backend = RecordingBackend::default();
    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input([], [], 35),
        &mut registry,
        &mut NoWindowFactory,
        &mut backend,
        &mut StaticReadback::new([]),
    )
    .unwrap();
    assert_eq!(backend.calls(), vec![NativeWindowCall::DiscardCreated]);
    assert!(receipt.is_converged());
}
