use longhorn_tauri_windowing::{
    ApplyConvergence, ApplyReadback, NativeWindowCall, NoWindowFactory, WindowApplyOutcome,
    execute_tauri_window_apply_in_place,
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
    let (app, mut registry) = registry(
        [
            (Some("window:a"), "a"),
            (Some("window:b"), "b"),
            (Some("window:stale"), "stale"),
            (Some("window:d"), "d"),
        ],
        None,
    );
    let mut backend = RecordingBackend::default();
    let inspection = backend.clone();
    let input = input(desired_windows, live_windows, 7)
        .with_focus_policy(FocusPolicy::Focus(id("window:a")));

    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input,
        &mut registry,
        &mut NoWindowFactory,
        &mut backend,
        &mut StaticReadback::new(fresh),
    )
    .unwrap();

    // Operations sort by kind, so placement now batches per axis across
    // windows rather than per window across axes: every origin, then every
    // size. Independent windows do not care, and a host that has one axis and
    // not the other gets a clean split instead of a half-executed compound.
    assert_eq!(
        inspection.calls(),
        vec![
            NativeWindowCall::Unmaximize,
            NativeWindowCall::SetOuterPosition,
            NativeWindowCall::SetOuterPosition,
            NativeWindowCall::SetInnerSize,
            NativeWindowCall::SetInnerSize,
            NativeWindowCall::Maximize,
            NativeWindowCall::Show,
            NativeWindowCall::Hide,
            NativeWindowCall::Focus,
            NativeWindowCall::Close,
        ]
    );
    assert!(receipt.is_converged());
    assert_eq!(registry.evidence().len(), receipt.attempts().len());
    assert!(registry.evidence().iter().any(|evidence| {
        evidence.generation().get() == 7
            && evidence.window_id() == &id("window:stale")
            && evidence.operation_kind() == WindowOperationKind::Close
            && evidence.transport_handle().map(|handle| handle.as_str()) == Some("stale")
    }));
    assert!(
        registry
            .managed_windows()
            .iter()
            .all(|window| window.window_id() != Some(&id("window:stale")))
    );
    assert!(matches!(
        receipt.readback(),
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
    let (app, mut registry) = registry([(Some("window:a"), "a")], None);
    let mut backend = RecordingBackend::default();
    let inspection = backend.clone();

    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input(
            [desired("window:a", target, false, true)],
            [matching.clone()],
            9,
        ),
        &mut registry,
        &mut NoWindowFactory,
        &mut backend,
        &mut StaticReadback::new([matching]),
    )
    .unwrap();

    assert!(receipt.plan().is_empty());
    assert!(receipt.attempts().is_empty());
    assert!(receipt.is_converged());
    assert!(inspection.calls().is_empty());
}

#[test]
fn a_failed_resize_skips_dependents_but_not_other_windows() {
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
    let (app, mut registry) = registry([(Some("window:a"), "a"), (Some("window:b"), "b")], None);
    let mut backend = RecordingBackend::failing(NativeWindowCall::SetInnerSize);
    let inspection = backend.clone();

    let receipt = execute_tauri_window_apply_in_place(
        app.handle(),
        input(desired_windows, live_windows.clone(), 8),
        &mut registry,
        &mut NoWindowFactory,
        &mut backend,
        &mut StaticReadback::new(live_windows),
    )
    .unwrap();

    // The axes are separate operations now, so the move succeeds on its own
    // and only the resize fails. Before the split both lived in one attempt
    // and a successful move was reported inside a failure.
    let moved = receipt
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Move)
        .unwrap();
    assert!(matches!(
        moved.outcome(),
        WindowApplyOutcome::Succeeded { completed_calls }
            if completed_calls == &[NativeWindowCall::SetOuterPosition]
    ));

    let resized = receipt
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Resize)
        .unwrap();
    assert!(matches!(
        resized.outcome(),
        WindowApplyOutcome::Failed {
            completed_calls,
            failure,
        } if completed_calls.is_empty()
            && failure.call() == NativeWindowCall::SetInnerSize
    ));
    assert!(receipt.attempts().iter().any(|attempt| {
        attempt.window_id() == &id("window:a")
            && matches!(
                attempt.outcome(),
                WindowApplyOutcome::DependencySkipped {
                    blocked_by: WindowOperationKind::Resize
                }
            )
    }));
    assert!(receipt.attempts().iter().any(|attempt| {
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
    assert!(!receipt.is_converged());
}
