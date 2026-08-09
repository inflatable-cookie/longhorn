//! Contract 020: "Windows — create, destroy, and observe, with a stable
//! opaque identity Longhorn does not interpret."

use longhorn_gpui_windowing::{
    GpuiApplyOutcome, GpuiWindowCall, GpuiWindowKey, GpuiWindowRegistry, ManagedGpuiWindow,
    execute_gpui_window_apply,
};
use longhorn_windowing::WindowOperationKind;

use super::support::{
    BareDisplayFacts, Call, FakeGpuiHost, SuppliedDisplayFacts, desired, handle_of, id, placement,
    plan,
};

#[test]
fn a_window_is_created_placed_from_a_shared_plan_and_observed() {
    let mut host = FakeGpuiHost::new();
    let mut displays = SuppliedDisplayFacts::new();
    let plan = plan(
        [desired("main", placement(120, 80, 900, 640), false, true)],
        1,
    );

    let bundle = execute_gpui_window_apply(
        plan,
        GpuiWindowRegistry::default(),
        &mut host,
        &mut displays,
    )
    .unwrap();

    let created = bundle
        .receipt()
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Create)
        .expect("a create was planned and attempted");
    assert!(matches!(
        created.outcome(),
        GpuiApplyOutcome::Succeeded { completed_calls }
            if completed_calls == &[
                GpuiWindowCall::ComposeCreateRequest,
                GpuiWindowCall::OpenWindow,
                GpuiWindowCall::RegistryInsert,
            ]
    ));
    // The origin arrives with the window because GPUI takes bounds at
    // creation; the size is then applied as its own operation, which a GPUI
    // host can do.
    assert_eq!(
        host.calls,
        vec![
            Call::Create(id("main")),
            Call::Resize(GpuiWindowKey::new(1))
        ]
    );

    // Observed: the readback found the window it just opened, at the placement
    // the shared plan asked for.
    let live = match bundle.receipt().readback() {
        longhorn_gpui_windowing::GpuiApplyReadback::Complete { observation, .. } => {
            observation.windows().to_vec()
        }
        longhorn_gpui_windowing::GpuiApplyReadback::Failed(error) => {
            panic!("readback failed: {error}")
        }
    };
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].window_id(), Some(&id("main")));
    assert_eq!(live[0].metrics().outer_bounds().origin().x().get(), 120);
    assert_eq!(live[0].metrics().outer_bounds().origin().y().get(), 80);
}

#[test]
fn the_transport_handle_is_opaque_and_longhorn_never_reads_a_gpui_slot_out_of_it() {
    // Contract 020 requires "a stable opaque identity Longhorn does not
    // interpret". GPUI identifies a window by a slot index, not a label, so
    // the adapter renders one. Nothing recovers the slot from the rendering:
    // the mapping lives in the adapter's registry.
    let key = GpuiWindowKey::new(42);

    assert_eq!(key.transport_handle().as_str(), "gpui-window:42");
    let registry =
        GpuiWindowRegistry::new([ManagedGpuiWindow::new(key, Some(id("main")))], None).unwrap();
    assert_eq!(registry.managed_windows()[0].key(), key);
}

#[test]
fn a_stale_window_is_destroyed_when_it_leaves_desired_state() {
    let (host, key) = FakeGpuiHost::new().with_existing_window(
        longhorn_gpui_windowing::GpuiLogicalRect::new(0.0, 0.0, 800.0, 600.0),
        longhorn_gpui_windowing::GpuiLogicalSize::new(800.0, 600.0),
        false,
    );
    let mut host = host;
    let mut displays = SuppliedDisplayFacts::new();
    let registry =
        GpuiWindowRegistry::new([ManagedGpuiWindow::new(key, Some(id("stale")))], None).unwrap();
    let live = longhorn_windowing::LiveWindow::new(
        Some(id("stale")),
        handle_of(key),
        longhorn_core::LiveWindowMetrics::new(
            longhorn_core::ScreenRect::new(
                longhorn_core::ScreenPoint::new(0, 0),
                longhorn_core::ScreenSize::new(800, 600),
            ),
            longhorn_core::ScreenSize::new(800, 600),
        ),
        false,
        true,
        false,
    );

    let plan = plan([], 1).with_live_windows([live]);
    let bundle = execute_gpui_window_apply(plan, registry, &mut host, &mut displays).unwrap();

    assert_eq!(host.calls, vec![Call::Close(key)]);
    assert!(!host.is_open(key));
    assert!(bundle.desired_state_reached());
}

#[test]
fn a_host_that_cannot_create_says_so_rather_than_failing_the_apply() {
    let mut host = FakeGpuiHost::new().without_create();
    let mut displays = BareDisplayFacts;

    let plan = plan([desired("main", placement(0, 0, 800, 600), false, true)], 1);
    let bundle = execute_gpui_window_apply(
        plan,
        GpuiWindowRegistry::default(),
        &mut host,
        &mut displays,
    )
    .unwrap();

    assert!(bundle.receipt().attempts().is_empty());
    assert!(!bundle.desired_state_reached());
    assert!(host.calls.is_empty());
}
