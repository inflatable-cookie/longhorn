//! Contract 020: "Placement application — execute the plans
//! `longhorn-windowing` produces. The planning is pure and shared; only
//! execution is per-host."
//!
//! This is where contract 020 bends, and the bends are GPUI-caused.

use longhorn_gpui_windowing::{
    GpuiDiagnosticDisposition, GpuiLogicalRect, GpuiLogicalSize, GpuiWindowRegistry,
    ManagedGpuiWindow, WITHHELD_CAPABILITIES, execute_gpui_window_apply, gpui_host_capabilities,
};
use longhorn_windowing::{
    HostCapability, LiveWindow, WindowDiffDiagnostic, WindowOperationKind, plan_window_diff,
};

use super::support::{
    Call, FakeGpuiHost, SuppliedDisplayFacts, desired, handle_of, id, placement, plan,
};

fn live_at(key: longhorn_gpui_windowing::GpuiWindowKey, maximized: bool) -> LiveWindow {
    LiveWindow::new(
        Some(id("main")),
        handle_of(key),
        longhorn_core::LiveWindowMetrics::new(
            longhorn_core::ScreenRect::new(
                longhorn_core::ScreenPoint::new(0, 0),
                longhorn_core::ScreenSize::new(800, 600),
            ),
            longhorn_core::ScreenSize::new(800, 600),
        ),
        maximized,
        true,
        false,
    )
}

#[test]
fn placement_is_applied_at_creation_because_gpui_cannot_move_a_window_afterwards() {
    // The plan says: create a neutral hidden unmaximized slot, then move and
    // resize it, then show it. GPUI has no move, no show, and no neutral slot
    // — bounds and maximized state are `WindowOptions` fields. So the adapter
    // opens the window in its final placement and reports that the plan's
    // MoveResize was satisfied at creation rather than skipped.
    let mut host = FakeGpuiHost::new();
    let mut displays = SuppliedDisplayFacts::new();

    let plan = plan(
        [desired("main", placement(300, 200, 1024, 768), false, true)],
        1,
    );
    let bundle = execute_gpui_window_apply(
        plan.input,
        &plan.desired,
        GpuiWindowRegistry::default(),
        &mut host,
        &mut displays,
    )
    .unwrap();

    assert_eq!(host.calls, vec![Call::Create(id("main"))]);
    let satisfied = bundle
        .dispositions()
        .iter()
        .filter(|disposition| {
            matches!(
                disposition,
                GpuiDiagnosticDisposition::SatisfiedAtCreate {
                    operation: WindowOperationKind::MoveResize,
                    ..
                }
            )
        })
        .count();
    assert_eq!(satisfied, 1);
    assert!(bundle.desired_state_reached());
}

#[test]
fn an_existing_window_cannot_be_moved_and_the_refusal_is_named() {
    // The same desired placement against a window that already exists is
    // genuinely unreachable. Nothing is faked and nothing silently succeeds:
    // the disposition is `Unsatisfiable` and desired state is not reached.
    let (host, key) = FakeGpuiHost::new().with_existing_window(
        GpuiLogicalRect::new(0.0, 0.0, 800.0, 600.0),
        GpuiLogicalSize::new(800.0, 600.0),
        false,
    );
    let mut host = host;
    let mut displays = SuppliedDisplayFacts::new();
    let registry =
        GpuiWindowRegistry::new([ManagedGpuiWindow::new(key, Some(id("main")))], None).unwrap();

    let plan = plan(
        [desired("main", placement(300, 200, 1024, 768), false, true)],
        1,
    )
    .with_live_windows([live_at(key, false)]);
    let bundle = execute_gpui_window_apply(
        plan.input,
        &plan.desired,
        registry,
        &mut host,
        &mut displays,
    )
    .unwrap();

    assert!(host.calls.is_empty());
    assert!(bundle.dispositions().iter().any(|disposition| matches!(
        disposition,
        GpuiDiagnosticDisposition::Unsatisfiable {
            capability: HostCapability::MoveResize,
            ..
        }
    )));
    assert!(!bundle.desired_state_reached());
}

#[test]
fn maximize_and_unmaximize_execute_against_a_toggle_only_host() {
    // GPUI has `zoom()`, which toggles, and `is_maximized()`. The absolute
    // operations the plan emits are satisfiable by reading first, so this is
    // an execution difference and not a contract bend.
    let (host, key) = FakeGpuiHost::new().with_existing_window(
        GpuiLogicalRect::new(0.0, 0.0, 800.0, 600.0),
        GpuiLogicalSize::new(800.0, 600.0),
        false,
    );
    let mut host = host;
    let mut displays = SuppliedDisplayFacts::new();
    let registry =
        GpuiWindowRegistry::new([ManagedGpuiWindow::new(key, Some(id("main")))], None).unwrap();

    let plan = plan([desired("main", placement(0, 0, 800, 600), true, true)], 1)
        .with_live_windows([live_at(key, false)]);
    let bundle = execute_gpui_window_apply(
        plan.input,
        &plan.desired,
        registry,
        &mut host,
        &mut displays,
    )
    .unwrap();

    assert!(host.calls.contains(&Call::SetMaximized(key, true)));
    assert!(host.is_maximized(key));
    assert!(
        bundle
            .receipt()
            .attempts()
            .iter()
            .any(|attempt| attempt.operation() == WindowOperationKind::Maximize)
    );
}

#[test]
fn the_planner_names_every_withheld_capability_without_the_adapter_teaching_it_to() {
    // The mechanism that carries a capability shortfall is host-neutral and
    // needed no change for a second backend: `plan_window_diff` turns a
    // withheld capability into an `UnsupportedOperation` diagnostic.
    // A live visible window desired hidden and moved exercises all three:
    // Show is the only one a create-only plan cannot reach, because a window
    // that does not exist yet is never asked to hide.
    let key = longhorn_gpui_windowing::GpuiWindowKey::new(1);
    let receipt = plan_window_diff(
        &plan(
            [desired("main", placement(10, 10, 640, 480), false, false)],
            1,
        )
        .input
        .with_capabilities(gpui_host_capabilities(true))
        .with_live_windows([live_at(key, false)]),
    )
    .unwrap();

    let refused: Vec<HostCapability> = receipt
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            WindowDiffDiagnostic::UnsupportedOperation {
                required_capability,
                ..
            } => Some(*required_capability),
            _ => None,
        })
        .collect();

    for withheld in WITHHELD_CAPABILITIES {
        if withheld.capability == HostCapability::Show {
            // A window desired hidden is never asked to show.
            continue;
        }
        assert!(
            refused.contains(&withheld.capability),
            "{:?} was not reported: {}",
            withheld.capability,
            withheld.reason
        );
    }
}

#[test]
fn a_window_desired_hidden_is_a_state_gpui_cannot_reach() {
    // Show is an artefact: a GPUI window is always on screen. Hide is real
    // and unreachable, and the two are distinguished rather than lumped.
    let (host, key) = FakeGpuiHost::new().with_existing_window(
        GpuiLogicalRect::new(0.0, 0.0, 800.0, 600.0),
        GpuiLogicalSize::new(800.0, 600.0),
        false,
    );
    let mut host = host;
    let mut displays = SuppliedDisplayFacts::new();
    let registry =
        GpuiWindowRegistry::new([ManagedGpuiWindow::new(key, Some(id("main")))], None).unwrap();

    let plan = plan(
        [desired("main", placement(0, 0, 800, 600), false, false)],
        1,
    )
    .with_live_windows([live_at(key, false)]);
    let bundle = execute_gpui_window_apply(
        plan.input,
        &plan.desired,
        registry,
        &mut host,
        &mut displays,
    )
    .unwrap();

    assert!(bundle.dispositions().iter().any(|disposition| matches!(
        disposition,
        GpuiDiagnosticDisposition::Unsatisfiable {
            capability: HostCapability::Hide,
            ..
        }
    )));
    assert!(!bundle.desired_state_reached());
}
