use longhorn_tauri_windowing::{
    ApplyConvergence, ApplyReadback, NativeWindowCall, NoWindowFactory, WindowApplyOutcome,
    execute_tauri_window_apply,
};
use longhorn_windowing::{FocusPolicy, WindowOperationKind};

use super::support::{
    RecordingBackend, StaticReadback, desired, id, input, live, placement, registry,
};

#[test]
fn every_native_operation_executes_and_fresh_readback_converges() {
    let original = placement(10, 10, 400, 300);
    let target = placement(50, 60, 800, 600);
    let restored = placement(80, 90, 500, 350);
    let hidden = placement(0, 0, 300, 200);
    let stale = placement(5, 5, 100, 100);
    let desired_windows = vec![
        desired("window:a", target, true, true),
        desired("window:b", hidden, false, false),
        desired("window:d", restored, false, true),
    ];
    let live_windows = vec![
        live(
            Some("window:a"),
            "a",
            original.outer_origin().x().get(),
            original.outer_origin().y().get(),
            original.inner_size().width(),
            original.inner_size().height(),
            false,
            false,
            false,
        ),
        live(Some("window:b"), "b", 0, 0, 300, 200, false, true, false),
        live(
            Some("window:stale"),
            "stale",
            stale.outer_origin().x().get(),
            stale.outer_origin().y().get(),
            stale.inner_size().width(),
            stale.inner_size().height(),
            false,
            true,
            false,
        ),
        live(Some("window:d"), "d", 20, 20, 450, 320, true, true, false),
    ];
    let fresh = vec![
        live(Some("window:a"), "a", 50, 60, 800, 600, true, true, true),
        live(Some("window:b"), "b", 0, 0, 300, 200, false, false, false),
        live(Some("window:d"), "d", 80, 90, 500, 350, false, true, false),
    ];
    let (app, registry) = registry(
        [
            (Some("window:a"), "a"),
            (Some("window:b"), "b"),
            (Some("window:stale"), "stale"),
            (Some("window:d"), "d"),
        ],
        None,
    );
    let backend = RecordingBackend::default();
    let inspection = backend.clone();
    let input = input(desired_windows, live_windows, 7)
        .with_focus_policy(FocusPolicy::Focus(id("window:a")));

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input,
        registry,
        NoWindowFactory,
        backend,
        StaticReadback::new(fresh),
    )
    .unwrap();

    assert_eq!(
        inspection.calls(),
        vec![
            NativeWindowCall::Unmaximize,
            NativeWindowCall::SetOuterPosition,
            NativeWindowCall::SetInnerSize,
            NativeWindowCall::SetOuterPosition,
            NativeWindowCall::SetInnerSize,
            NativeWindowCall::Maximize,
            NativeWindowCall::Show,
            NativeWindowCall::Hide,
            NativeWindowCall::Focus,
            NativeWindowCall::Close,
        ]
    );
    assert!(outcome.receipt().is_converged());
    assert_eq!(
        outcome.registry().evidence().len(),
        outcome.receipt().attempts().len()
    );
    assert!(outcome.registry().evidence().iter().any(|evidence| {
        evidence.generation().get() == 7
            && evidence.window_id() == &id("window:stale")
            && evidence.operation_kind() == WindowOperationKind::Close
            && evidence.transport_handle().map(|handle| handle.as_str()) == Some("stale")
    }));
    assert!(
        outcome
            .registry()
            .managed_windows()
            .iter()
            .all(|window| window.window_id() != Some(&id("window:stale")))
    );
    assert!(matches!(
        outcome.receipt().readback(),
        ApplyReadback::Complete {
            convergence: ApplyConvergence::Planned(receipt),
            ..
        } if receipt.is_empty()
    ));
}

#[test]
fn matching_fresh_state_repeats_as_an_empty_apply() {
    let target = placement(15, 25, 640, 480);
    let matching = live(Some("window:a"), "a", 15, 25, 640, 480, false, true, false);
    let (app, registry) = registry([(Some("window:a"), "a")], None);
    let backend = RecordingBackend::default();
    let inspection = backend.clone();

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input(
            [desired("window:a", target, false, true)],
            [matching.clone()],
            9,
        ),
        registry,
        NoWindowFactory,
        backend,
        StaticReadback::new([matching]),
    )
    .unwrap();

    assert!(outcome.receipt().plan().is_empty());
    assert!(outcome.receipt().attempts().is_empty());
    assert!(outcome.receipt().is_converged());
    assert!(inspection.calls().is_empty());
}

#[test]
fn partial_move_resize_failure_skips_dependents_but_not_other_windows() {
    let target = placement(50, 60, 800, 600);
    let other = placement(0, 0, 300, 200);
    let desired_windows = vec![
        desired("window:a", target, true, true),
        desired("window:b", other, false, false),
    ];
    let live_windows = vec![
        live(Some("window:a"), "a", 10, 10, 400, 300, false, false, false),
        live(Some("window:b"), "b", 0, 0, 300, 200, false, true, false),
    ];
    let (app, registry) = registry([(Some("window:a"), "a"), (Some("window:b"), "b")], None);
    let backend = RecordingBackend::failing(NativeWindowCall::SetInnerSize);
    let inspection = backend.clone();

    let outcome = execute_tauri_window_apply(
        app.handle(),
        input(desired_windows, live_windows.clone(), 8),
        registry,
        NoWindowFactory,
        backend,
        StaticReadback::new(live_windows),
    )
    .unwrap();

    let move_attempt = outcome
        .receipt()
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::MoveResize)
        .unwrap();
    assert!(matches!(
        move_attempt.outcome(),
        WindowApplyOutcome::Failed {
            completed_calls,
            failure,
        } if completed_calls == &[NativeWindowCall::SetOuterPosition]
            && failure.call() == NativeWindowCall::SetInnerSize
    ));
    assert!(outcome.receipt().attempts().iter().any(|attempt| {
        attempt.window_id() == &id("window:a")
            && matches!(
                attempt.outcome(),
                WindowApplyOutcome::DependencySkipped {
                    blocked_by: WindowOperationKind::MoveResize
                }
            )
    }));
    assert!(outcome.receipt().attempts().iter().any(|attempt| {
        attempt.window_id() == &id("window:b")
            && attempt.operation() == WindowOperationKind::Hide
            && matches!(attempt.outcome(), WindowApplyOutcome::Succeeded { .. })
    }));
    assert_eq!(
        inspection.calls(),
        vec![
            NativeWindowCall::SetOuterPosition,
            NativeWindowCall::SetInnerSize,
            NativeWindowCall::Hide,
        ]
    );
    assert!(!outcome.receipt().is_converged());
}
