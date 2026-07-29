use longhorn_tauri_windowing::{
    ManagedWindowRegistryError, NativeWindowCall, NoWindowFactory, WindowApplyFailureKind,
    WindowApplyOutcome, execute_tauri_window_apply,
};
use longhorn_windowing::{ApplyGeneration, ProtectedPrimaryPolicy, WindowOperationKind};

use super::support::{
    RecordingBackend, StaticReadback, desired, handle, id, input, live, placement, registry,
};

#[test]
fn protected_primary_retags_bookkeeping_without_changing_native_label() {
    let placement = placement(10, 20, 600, 400);
    let (app, registry) = registry([(None, "primary")], Some("primary"));
    let input = input(
        [desired("window:main", placement, false, true)],
        [live(None, "primary", 10, 20, 600, 400, false, true, false)],
        11,
    )
    .with_protected_primary(ProtectedPrimaryPolicy::Reuse {
        transport_handle: handle("primary"),
        window_id: id("window:main"),
    });

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input,
        registry,
        NoWindowFactory,
        RecordingBackend::default(),
        StaticReadback::new([live(
            Some("window:main"),
            "primary",
            10,
            20,
            600,
            400,
            false,
            true,
            false,
        )]),
    )
    .unwrap();

    assert!(outcome.receipt().is_converged());
    assert_eq!(
        outcome.registry().managed_windows()[0].clone().window_id(),
        Some(&id("window:main"))
    );
    let retag = &outcome.receipt().attempts()[0];
    assert_eq!(retag.operation(), WindowOperationKind::Retag);
    assert!(matches!(
        retag.outcome(),
        WindowApplyOutcome::Succeeded { completed_calls }
            if completed_calls == &[NativeWindowCall::RegistryRetag]
    ));
}

#[test]
fn registry_protection_refuses_close_even_if_planner_policy_omits_it() {
    let (app, registry) = registry([(Some("window:main"), "primary")], Some("primary"));
    let stale = live(
        Some("window:main"),
        "primary",
        0,
        0,
        400,
        300,
        false,
        true,
        false,
    );
    let backend = RecordingBackend::default();
    let inspection = backend.clone();

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input(Vec::new(), [stale.clone()], 12),
        registry,
        NoWindowFactory,
        backend,
        StaticReadback::new([stale]),
    )
    .unwrap();

    assert!(inspection.calls().is_empty());
    assert!(matches!(
        outcome.receipt().attempts()[0].outcome(),
        WindowApplyOutcome::Failed { failure, .. }
            if failure.kind() == WindowApplyFailureKind::ProtectedPrimary
                && failure.call() == NativeWindowCall::ProtectPrimary
    ));
}

#[test]
fn stale_generation_is_rejected_before_native_execution() {
    let (app, mut registry) = registry([(Some("window:a"), "a")], None);
    registry.begin_generation(ApplyGeneration::new(20)).unwrap();
    let live = live(Some("window:a"), "a", 0, 0, 400, 300, false, true, false);
    let error = execute_tauri_window_apply(
        app.handle(),
        input(
            [desired(
                "window:a",
                placement(10, 10, 400, 300),
                false,
                true,
            )],
            [live.clone()],
            19,
        ),
        registry,
        NoWindowFactory,
        RecordingBackend::default(),
        StaticReadback::new([live]),
    )
    .err()
    .unwrap();

    assert!(matches!(
        error,
        longhorn_tauri_windowing::TauriApplyError::Registry(
            ManagedWindowRegistryError::StaleGeneration { .. }
        )
    ));
}
