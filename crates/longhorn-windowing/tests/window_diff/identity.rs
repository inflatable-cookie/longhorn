use longhorn_windowing::{
    ProtectedPrimaryPolicy, WindowDiffDiagnostic, WindowDiffError, WindowOperationKind,
    plan_window_diff,
};

use super::support::*;

#[test]
fn stable_identity_wins_over_deceptive_transport_labels() {
    let receipt = plan(&input(
        [desired("window:a", 0, 0, 500, 400, false, true)],
        [
            live(
                Some("window:a"),
                "window:b",
                0,
                0,
                520,
                440,
                500,
                400,
                false,
                true,
                false,
            ),
            live(
                Some("window:b"),
                "window:a",
                0,
                0,
                520,
                440,
                500,
                400,
                false,
                true,
                false,
            ),
        ],
    ));

    assert_eq!(kinds(&receipt), [WindowOperationKind::Close]);
    assert_eq!(
        receipt.operations()[0].operation().window_id(),
        &id("window:b")
    );
}

#[test]
fn protected_primary_reuse_is_an_explicit_retag_and_never_a_close() {
    let receipt = plan(
        &input(
            [desired("window:main", 0, 0, 500, 400, false, true)],
            [live(
                Some("window:bootstrap"),
                "primary",
                0,
                0,
                520,
                440,
                500,
                400,
                false,
                true,
                false,
            )],
        )
        .with_protected_primary(ProtectedPrimaryPolicy::Reuse {
            transport_handle: handle("primary"),
            window_id: id("window:main"),
        }),
    );

    assert_eq!(kinds(&receipt), [WindowOperationKind::Retag]);
    assert_eq!(
        receipt.operations()[0].operation().transport_handle(),
        Some(&handle("primary"))
    );
}

#[test]
fn protected_reuse_conflict_is_diagnostic_and_does_not_infer_a_close() {
    let receipt = plan(
        &input(
            [desired("window:main", 0, 0, 500, 400, false, true)],
            [
                live(
                    None, "primary", 0, 0, 520, 440, 500, 400, false, true, false,
                ),
                live(
                    Some("window:main"),
                    "secondary",
                    0,
                    0,
                    520,
                    440,
                    500,
                    400,
                    false,
                    true,
                    false,
                ),
            ],
        )
        .with_protected_primary(ProtectedPrimaryPolicy::Reuse {
            transport_handle: handle("primary"),
            window_id: id("window:main"),
        }),
    );

    assert!(receipt.operations().is_empty());
    assert_eq!(
        receipt.diagnostics(),
        [WindowDiffDiagnostic::ProtectedReuseConflict {
            protected_handle: handle("primary"),
            window_id: id("window:main"),
            matched_handle: handle("secondary"),
        }]
    );
}

#[test]
fn duplicate_snapshot_identity_returns_a_typed_error() {
    let desired = desired("window:main", 0, 0, 500, 400, false, true);
    let duplicate_desired = input([desired.clone(), desired], []);
    assert_eq!(
        plan_window_diff(&duplicate_desired),
        Err(WindowDiffError::DuplicateDesiredWindowId(id("window:main")))
    );

    let first = live(
        Some("window:main"),
        "one",
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
    let second = live(
        Some("window:main"),
        "two",
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
    assert_eq!(
        plan_window_diff(&input([], [first, second])),
        Err(WindowDiffError::DuplicateLiveWindowId(id("window:main")))
    );
}

#[test]
fn unidentified_unprotected_live_slot_is_reported_not_interpreted() {
    let receipt = plan(&input(
        [],
        [live(
            None,
            "looks-like-window:main",
            0,
            0,
            1,
            1,
            1,
            1,
            false,
            false,
            false,
        )],
    ));

    assert!(receipt.operations().is_empty());
    assert_eq!(
        receipt.diagnostics(),
        [WindowDiffDiagnostic::UnidentifiedLiveWindow {
            transport_handle: handle("looks-like-window:main"),
        }]
    );
}
