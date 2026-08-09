use longhorn_windowing::{
    FocusPolicy, ProtectedPrimaryPolicy, WindowOperation, WindowOperationKind,
};

use super::support::*;

#[test]
fn creation_precedes_geometry_maximize_visibility_and_focus() {
    let receipt = plan(
        &input([desired("window:main", 40, 60, 900, 700, true, true)], [])
            .with_focus_policy(FocusPolicy::Focus(id("window:main"))),
    );

    assert_eq!(
        kinds(&receipt),
        [
            WindowOperationKind::Create,
            WindowOperationKind::Move,
            WindowOperationKind::Resize,
            WindowOperationKind::Maximize,
            WindowOperationKind::Show,
            WindowOperationKind::Focus,
        ]
    );
    assert!(
        receipt
            .operations()
            .iter()
            .all(|op| op.generation().get() == 42)
    );
}

#[test]
fn placement_compares_outer_origin_and_inner_size_not_outer_extent() {
    let wanted = desired("window:main", 10, 20, 800, 600, false, true);
    let frame_extent_only = live(
        Some("window:main"),
        "native-main",
        10,
        20,
        840,
        670,
        800,
        600,
        false,
        true,
        false,
    );
    assert!(plan(&input([wanted.clone()], [frame_extent_only])).is_empty());

    let wrong_inner_size = live(
        Some("window:main"),
        "native-main",
        10,
        20,
        800,
        600,
        760,
        540,
        false,
        true,
        false,
    );
    // Only the size is wrong, so only the size is corrected. Before the axes
    // were split this emitted a compound that also reapplied an origin the
    // window already had.
    let receipt = plan(&input([wanted.clone()], [wrong_inner_size]));
    assert_eq!(kinds(&receipt), [WindowOperationKind::Resize]);
    assert!(matches!(
        receipt.operations()[0].operation(),
        WindowOperation::Resize { inner_size: value, .. }
            if *value == placement(10, 20, 800, 600).inner_size()
    ));

    let wrong_origin = live(
        Some("window:main"),
        "native-main",
        11,
        20,
        800,
        600,
        800,
        600,
        false,
        true,
        false,
    );
    let receipt = plan(&input([wanted], [wrong_origin]));
    assert_eq!(kinds(&receipt), [WindowOperationKind::Move]);
    assert!(matches!(
        receipt.operations()[0].operation(),
        WindowOperation::Move { outer_origin: value, .. }
            if *value == placement(10, 20, 800, 600).outer_origin()
    ));
}

#[test]
fn maximize_and_unmaximize_are_explicit_and_normal_geometry_is_retained() {
    let maximize_receipt = plan(&input(
        [desired("window:main", 10, 20, 800, 600, true, true)],
        [live(
            Some("window:main"),
            "native-main",
            10,
            20,
            820,
            640,
            800,
            600,
            false,
            true,
            false,
        )],
    ));
    assert_eq!(kinds(&maximize_receipt), [WindowOperationKind::Maximize]);

    let unmaximize_receipt = plan(&input(
        [desired("window:main", 10, 20, 800, 600, false, true)],
        [live(
            Some("window:main"),
            "native-main",
            0,
            0,
            1920,
            1080,
            1920,
            1050,
            true,
            true,
            false,
        )],
    ));
    assert_eq!(
        kinds(&unmaximize_receipt),
        [
            WindowOperationKind::Unmaximize,
            WindowOperationKind::Move,
            WindowOperationKind::Resize,
        ]
    );
}

#[test]
fn visibility_and_focus_changes_are_explicit() {
    let receipt = plan(
        &input(
            [
                desired("window:hide", 0, 0, 500, 400, false, false),
                desired("window:show", 500, 0, 500, 400, false, true),
            ],
            [
                live(
                    Some("window:hide"),
                    "hide-handle",
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
                    Some("window:show"),
                    "show-handle",
                    500,
                    0,
                    520,
                    440,
                    500,
                    400,
                    false,
                    false,
                    false,
                ),
            ],
        )
        .with_focus_policy(FocusPolicy::Focus(id("window:show"))),
    );

    assert_eq!(
        kinds(&receipt),
        [
            WindowOperationKind::Show,
            WindowOperationKind::Hide,
            WindowOperationKind::Focus,
        ]
    );
}

#[test]
fn closes_are_last_sorted_by_stable_id_and_protected_slots_survive() {
    let receipt = plan(
        &input(
            [desired("window:new", 0, 0, 500, 400, false, false)],
            [
                live(
                    Some("window:z"),
                    "handle-z",
                    0,
                    0,
                    1,
                    1,
                    1,
                    1,
                    false,
                    false,
                    false,
                ),
                live(
                    Some("window:a"),
                    "handle-a",
                    0,
                    0,
                    1,
                    1,
                    1,
                    1,
                    false,
                    false,
                    false,
                ),
                live(
                    Some("window:protected"),
                    "primary",
                    0,
                    0,
                    1,
                    1,
                    1,
                    1,
                    false,
                    false,
                    false,
                ),
            ],
        )
        .with_protected_primary(ProtectedPrimaryPolicy::Preserve {
            transport_handle: handle("primary"),
        }),
    );

    assert_eq!(
        kinds(&receipt),
        [
            WindowOperationKind::Create,
            WindowOperationKind::Move,
            WindowOperationKind::Resize,
            WindowOperationKind::Close,
            WindowOperationKind::Close,
        ]
    );
    let closed: Vec<_> = receipt.operations()[3..]
        .iter()
        .map(|op| op.operation().window_id().as_str())
        .collect();
    assert_eq!(closed, ["window:a", "window:z"]);
}
