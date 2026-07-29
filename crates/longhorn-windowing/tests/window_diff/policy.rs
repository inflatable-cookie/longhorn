use longhorn_windowing::{
    ApplyGeneration, HostCapabilities, HostCapability, WindowDiffDiagnostic, WindowDiffInput,
    WindowDiffReceipt, WindowOperationKind,
};

use super::support::*;

#[test]
fn unsupported_create_is_diagnostic_without_dependent_fabricated_operations() {
    let value = WindowDiffInput::new(
        [desired("window:main", 0, 0, 500, 400, true, true)],
        [],
        HostCapabilities::none(),
        ApplyGeneration::new(7),
    );
    let receipt = plan(&value);

    assert!(receipt.operations().is_empty());
    assert_eq!(
        receipt.diagnostics(),
        [WindowDiffDiagnostic::UnsupportedOperation {
            operation: WindowOperationKind::Create,
            window_id: id("window:main"),
            transport_handle: None,
            required_capability: HostCapability::Create,
        }]
    );
}

#[test]
fn generation_evidence_rejects_feedback_from_an_older_apply() {
    let receipt = plan(&input(
        [desired("window:main", 0, 0, 500, 400, false, false)],
        [],
    ));
    let evidence = receipt.operations()[0].feedback_evidence();

    assert!(evidence.is_current(ApplyGeneration::new(42)));
    assert!(!evidence.is_current(ApplyGeneration::new(43)));
    assert_eq!(evidence.window_id(), &id("window:main"));
    assert_eq!(evidence.operation(), WindowOperationKind::Create);
}

#[test]
fn desired_and_live_input_permutations_produce_identical_receipts() {
    let desired_a = desired("window:a", 10, 20, 500, 400, false, true);
    let desired_b = desired("window:b", 600, 20, 500, 400, false, true);
    let live_a = live(
        Some("window:a"),
        "native-z",
        0,
        0,
        500,
        400,
        450,
        350,
        false,
        false,
        false,
    );
    let stale = live(
        Some("window:stale"),
        "native-a",
        0,
        0,
        1,
        1,
        1,
        1,
        false,
        false,
        false,
    );

    let forward = plan(&input(
        [desired_a.clone(), desired_b.clone()],
        [live_a.clone(), stale.clone()],
    ));
    let reverse = plan(&input([desired_b, desired_a], [stale, live_a]));
    assert_eq!(forward, reverse);
}

#[test]
fn already_matching_snapshot_has_an_idempotent_empty_diff() {
    let receipt = plan(&input(
        [desired("window:main", -100, 40, 900, 700, false, true)],
        [live(
            Some("window:main"),
            "native-main",
            -100,
            40,
            940,
            760,
            900,
            700,
            false,
            true,
            false,
        )],
    ));
    assert!(receipt.is_empty());
}

#[test]
fn no_surface_and_hosted_surface_shapes_produce_the_same_window_plan() {
    let nucleus_windows = [
        desired("window:main", 0, 0, 1000, 700, false, true),
        desired("window:tool", 1000, 0, 500, 700, false, true),
    ];
    let loophole_surface_hosts = [
        desired("window:tool", 1000, 0, 500, 700, false, true),
        desired("window:main", 0, 0, 1000, 700, false, true),
    ];

    assert_eq!(
        plan(&input(nucleus_windows, [])),
        plan(&input(loophole_surface_hosts, []))
    );
}

#[test]
fn hidden_restore_suppresses_reveal_and_focus_until_the_host_gate() {
    let value = input(
        [desired("window:main", 0, 0, 500, 400, false, true)],
        [live(
            Some("window:main"),
            "native-main",
            0,
            0,
            520,
            440,
            500,
            400,
            false,
            true,
            true,
        )],
    )
    .for_hidden_restore();

    assert_eq!(kinds(&plan(&value)), [WindowOperationKind::Hide]);
}

#[test]
fn input_and_receipt_are_serializable_contract_evidence() {
    let value = input([desired("window:main", 0, 0, 500, 400, false, true)], []);
    let input_json = serde_json::to_string(&value).unwrap();
    assert_eq!(
        serde_json::from_str::<WindowDiffInput>(&input_json).unwrap(),
        value
    );

    let receipt = plan(&value);
    let receipt_json = serde_json::to_string(&receipt).unwrap();
    assert_eq!(
        serde_json::from_str::<WindowDiffReceipt>(&receipt_json).unwrap(),
        receipt
    );
}
