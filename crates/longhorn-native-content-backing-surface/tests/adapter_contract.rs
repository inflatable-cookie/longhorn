//! Production backing-surface contracts over deterministic consumer ports.

mod support;

use std::sync::{Arc, Mutex};

use longhorn_core::PhysicalPoint;
use longhorn_native_content::{
    AttachmentLifecycle, DesiredPresence, DesiredVisibility, InputRoutingMode, OperationOutcome,
};
use longhorn_native_content_backing_surface::{
    BackingSurfaceAdapterEvent, BackingSurfaceDetachOutcome, BackingSurfaceError,
    BackingSurfaceHostDestroyOutcome, BackingSurfaceRuntimeEvent, BackingSurfaceRuntimeEventKind,
    InputAdmission, InputRejection,
};

use support::{
    Call, FakeRuntime, adapter, attach_generation, coordinator, desired_update, host_window_id,
    island_id, rect, scale, viewport,
};

#[test]
fn full_host_storage_stays_distinct_and_listener_precedes_attach() {
    let runtime = FakeRuntime::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), Arc::clone(&events));
    let mut authority = coordinator(1);

    apply_and_observe(&adapter, &mut authority);

    let current = runtime.current();
    assert_eq!(current.storage_bounds, rect(0, 0, 1_600, 1_000));
    assert_eq!(current.clip, rect(240, 180, 840, 560));
    assert!(current.presentation_enabled);
    assert_eq!(current.input_routing, InputRoutingMode::RendererForwarded);
    assert!(current.native_storage_attached);
    let events = events.lock().unwrap();
    assert!(matches!(
        events[0],
        BackingSurfaceAdapterEvent::ListenerInstalled { .. }
    ));
    assert!(matches!(
        events[1],
        BackingSurfaceAdapterEvent::AttachStarted { .. }
    ));
}

#[test]
fn viewport_move_collapse_and_restore_clip_output_without_moving_storage() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut authority = coordinator(1);
    apply_and_observe(&adapter, &mut authority);
    let storage = runtime.current().storage_bounds;

    for (target, expected_pixels) in [
        (viewport(180.0, 120.0, 300.0, 220.0), 600_u64 * 440),
        (viewport(180.0, 120.0, 0.0, 0.0), 0_u64),
        (viewport(120.0, 90.0, 420.0, 280.0), 840_u64 * 560),
    ] {
        authority
            .update_desired(
                authority.desired().revision(),
                desired_update(
                    1,
                    target,
                    scale(2_000),
                    DesiredPresence::Present,
                    DesiredVisibility::Visible,
                    InputRoutingMode::RendererForwarded,
                ),
            )
            .unwrap();
        apply_and_observe(&adapter, &mut authority);
        assert_eq!(runtime.current().storage_bounds, storage);
        assert_eq!(runtime.pixels().lit_pixels, expected_pixels);
        assert_eq!(runtime.pixels().outside_clip_lit_pixels, 0);
    }
}

#[test]
fn physical_input_gate_is_complete_and_never_accepts_a_semantic_payload() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime, Arc::default());
    let mut authority = coordinator(1);
    apply_and_observe(&adapter, &mut authority);
    let generation = attach_generation(1);

    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::HostUnfocused)
    );
    adapter.update_host_focus(generation, true).unwrap();
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(100, 100))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::OutsideViewport)
    );
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Admitted
    );

    authority
        .update_desired(
            authority.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(2_000),
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                InputRoutingMode::Disabled,
            ),
        )
        .unwrap();
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::RoutingDisabled)
    );

    authority
        .update_desired(
            authority.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(2_000),
                DesiredPresence::Present,
                DesiredVisibility::Hidden {
                    reason: longhorn_core::VisibilityReasonId::new("overlay").unwrap(),
                },
                InputRoutingMode::Disabled,
            ),
        )
        .unwrap();
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    assert_eq!(
        adapter
            .admit_input(generation, PhysicalPoint::new(300, 300))
            .unwrap(),
        InputAdmission::Rejected(InputRejection::PresentationDisabled)
    );
}

#[test]
fn host_resize_changes_storage_without_rewriting_clip() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut authority = coordinator(1);
    apply_and_observe(&adapter, &mut authority);
    let clip = runtime.current().clip;

    runtime.set_storage(rect(0, 0, 1_920, 1_200), scale(2_000));
    let refreshed = adapter.refresh_host_geometry(attach_generation(1)).unwrap();

    assert_eq!(refreshed.storage_bounds, rect(0, 0, 1_920, 1_200));
    assert_eq!(refreshed.clip, clip);
}

#[test]
fn stale_plan_event_and_render_result_leave_current_state_unchanged() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut authority = coordinator(2);
    let initial_plan = authority.plan().unwrap();
    apply_and_observe(&adapter, &mut authority);

    authority
        .update_desired(
            authority.desired().revision(),
            desired_update(
                2,
                viewport(180.0, 120.0, 300.0, 220.0),
                scale(2_000),
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    apply_and_observe(&adapter, &mut authority);
    let before = runtime.current();
    let calls = runtime.calls();

    assert!(matches!(
        adapter.apply(&authority, &initial_plan),
        Err(BackingSurfaceError::Receipt(_))
    ));
    assert_eq!(runtime.calls(), calls);
    assert!(matches!(
        adapter.admit_runtime_event(BackingSurfaceRuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: attach_generation(1),
            sequence: 90,
            kind: BackingSurfaceRuntimeEventKind::FramePresented { sequence: 90 },
        }),
        Err(BackingSurfaceError::StaleGeneration { .. })
    ));
    runtime.emit(
        2,
        2,
        BackingSurfaceRuntimeEventKind::FramePresented {
            sequence: before.frame_sequence,
        },
    );
    assert!(matches!(
        adapter.admit_runtime_event(BackingSurfaceRuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: attach_generation(2),
            sequence: 2,
            kind: BackingSurfaceRuntimeEventKind::StorageChanged {
                bounds: rect(0, 0, 9, 9),
            },
        }),
        Err(BackingSurfaceError::StaleEventSequence { .. })
    ));
    runtime.set_frame_sequence(before.frame_sequence - 1);
    assert!(matches!(
        adapter.refresh_host_geometry(attach_generation(2)),
        Err(BackingSurfaceError::StaleFrameSequence { .. })
    ));
}

#[test]
fn host_destroy_invalidates_before_retryable_reversible_detach() {
    let runtime = FakeRuntime::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = adapter(runtime.clone(), Arc::clone(&events));
    let mut authority = coordinator(1);
    apply_and_observe(&adapter, &mut authority);
    runtime.fail_detach_times(1);

    authority
        .host_destroyed(&host_window_id(), authority.observed().revision())
        .unwrap();
    assert!(matches!(
        adapter.host_destroyed(&host_window_id(), attach_generation(1)),
        Err(BackingSurfaceError::Runtime {
            operation: "detach",
            ..
        })
    ));
    assert!(matches!(
        adapter.admit_runtime_event(BackingSurfaceRuntimeEvent {
            island_id: island_id(),
            host_window_id: host_window_id(),
            generation: attach_generation(1),
            sequence: 99,
            kind: BackingSurfaceRuntimeEventKind::StorageChanged {
                bounds: rect(0, 0, 10, 10),
            },
        }),
        Err(BackingSurfaceError::GenerationInvalidated(_))
    ));
    let receipt = adapter
        .host_destroyed(&host_window_id(), attach_generation(1))
        .unwrap();
    assert_eq!(
        receipt.outcome(),
        BackingSurfaceHostDestroyOutcome::AlreadyInvalidated
    );
    assert_eq!(receipt.detach(), BackingSurfaceDetachOutcome::Detached);
    let again = adapter
        .host_destroyed(&host_window_id(), attach_generation(1))
        .unwrap();
    assert_eq!(again.detach(), BackingSurfaceDetachOutcome::AlreadyDetached);
    assert_eq!(
        adapter.observe(attach_generation(1)).unwrap().lifecycle(),
        AttachmentLifecycle::Absent
    );

    let events = events.lock().unwrap();
    let invalidated = events
        .iter()
        .position(|event| matches!(event, BackingSurfaceAdapterEvent::HostInvalidated { .. }))
        .unwrap();
    let detached = events
        .iter()
        .position(|event| matches!(event, BackingSurfaceAdapterEvent::Detached { .. }))
        .unwrap();
    assert!(invalidated < detached);
}

#[test]
fn runtime_failure_is_an_exact_partial_receipt_and_retry_is_idempotent() {
    let runtime = FakeRuntime::default();
    runtime.fail_next_clip();
    let adapter = adapter(runtime.clone(), Arc::default());
    let authority = coordinator(1);
    let plan = authority.plan().unwrap();

    let receipt = adapter.apply(&authority, &plan).unwrap();
    assert_eq!(receipt.steps()[0].outcome(), &OperationOutcome::Applied);
    assert!(matches!(
        receipt.steps()[1].outcome(),
        OperationOutcome::Failed { .. }
    ));
    assert!(
        receipt.steps()[2..]
            .iter()
            .all(|step| matches!(step.outcome(), OperationOutcome::DependencySkipped { .. }))
    );

    let retry = adapter.apply(&authority, &plan).unwrap();
    assert!(
        retry
            .steps()
            .iter()
            .all(|step| step.outcome() == &OperationOutcome::Applied)
    );
    assert_eq!(
        runtime
            .calls()
            .iter()
            .filter(|call| matches!(call, Call::Attach { .. }))
            .count(),
        1
    );
}

#[test]
fn explicit_one_and_two_x_conversion_preserves_unknown_native_focus_visibility() {
    let runtime = FakeRuntime::default();
    let adapter = adapter(runtime.clone(), Arc::default());
    let mut authority = coordinator(1);
    apply_and_observe(&adapter, &mut authority);

    authority
        .update_desired(
            authority.desired().revision(),
            desired_update(
                1,
                viewport(120.0, 90.0, 420.0, 280.0),
                scale(1_000),
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    adapter
        .apply(&authority, &authority.plan().unwrap())
        .unwrap();
    assert_eq!(runtime.current().clip, rect(120, 90, 420, 280));
    let observation = adapter.observe(attach_generation(1)).unwrap();
    assert_eq!(observation.lifecycle(), AttachmentLifecycle::Attached);
    authority
        .admit_observation(authority.observed().revision(), observation)
        .unwrap();
    assert_eq!(
        authority.observed().visibility(),
        longhorn_native_content::EffectiveVisibility::Unknown
    );
    assert_eq!(
        authority.observed().focus(),
        longhorn_native_content::EffectiveFocus::Unknown
    );
}

fn apply_and_observe(
    adapter: &longhorn_native_content_backing_surface::BackingSurfaceAdapter<FakeRuntime>,
    authority: &mut longhorn_native_content::NativeContentCoordinator,
) {
    adapter
        .apply(authority, &authority.plan().unwrap())
        .unwrap();
    let observation = adapter.observe(authority.desired().generation()).unwrap();
    authority
        .admit_observation(authority.observed().revision(), observation)
        .unwrap();
}
