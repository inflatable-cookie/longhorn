use longhorn_config::{DebounceTerminal, DebouncedMutation, FlushOutcome};

use crate::common::Fixture;

use super::support::{
    DesktopDomain, DesktopPatch, DesktopStrategy, FixedClock, Geometry, Presentation, loaded,
    policy,
};

#[test]
fn loophole_style_layout_commands_coalesce_over_fresh_external_state() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = DesktopDomain::new();
    store.register(&domain).unwrap();
    let mut lane =
        DebouncedMutation::new(&store, &domain, DesktopStrategy, FixedClock, policy()).unwrap();
    lane.stage(DesktopPatch {
        sidebar_width: Some(320),
        ..DesktopPatch::default()
    })
    .unwrap();
    lane.stage(DesktopPatch {
        active_panel: Some("history".to_owned()),
        ..DesktopPatch::default()
    })
    .unwrap();

    store
        .mutate(&domain, policy().mutation_options(), |value| {
            value.theme = "dark".to_owned();
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        lane.flush_forced(),
        FlushOutcome::Terminal(DebounceTerminal::Published { generation: 2, .. })
    ));

    let state = loaded(&store, &domain);
    assert_eq!(state.layout.sidebar_width, 320);
    assert_eq!(state.layout.active_panel, "history");
    assert_eq!(state.theme, "dark");
}

#[test]
fn nucleus_style_geometry_is_bounded_last_value_replacement() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = DesktopDomain::new();
    store.register(&domain).unwrap();
    let mut lane =
        DebouncedMutation::new(&store, &domain, DesktopStrategy, FixedClock, policy()).unwrap();
    let first = Geometry {
        x: 10,
        y: 20,
        width: 1000,
        height: 700,
    };
    let latest = Geometry {
        x: 30,
        y: 40,
        width: 1400,
        height: 900,
    };
    let first_receipt = lane
        .stage(DesktopPatch {
            geometry: Some(first),
            ..DesktopPatch::default()
        })
        .unwrap();
    let latest_receipt = lane
        .stage(DesktopPatch {
            geometry: Some(latest.clone()),
            ..DesktopPatch::default()
        })
        .unwrap();

    assert_eq!(first_receipt.pending_weight, latest_receipt.pending_weight);
    lane.flush_forced();
    assert_eq!(loaded(&store, &domain).geometry, latest);
}

#[test]
fn split_shell_style_presentation_replacement_preserves_other_projections() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = DesktopDomain::new();
    store.register(&domain).unwrap();
    let mut lane =
        DebouncedMutation::new(&store, &domain, DesktopStrategy, FixedClock, policy()).unwrap();
    let latest = Presentation {
        navigation_percent: 36,
        selected_node: "module:e3".to_owned(),
    };
    lane.stage(DesktopPatch {
        presentation: Some(Presentation {
            navigation_percent: 30,
            selected_node: "pathway:acca".to_owned(),
        }),
        ..DesktopPatch::default()
    })
    .unwrap();
    lane.stage(DesktopPatch {
        presentation: Some(latest.clone()),
        ..DesktopPatch::default()
    })
    .unwrap();
    lane.flush_forced();

    let state = loaded(&store, &domain);
    assert_eq!(state.presentation, latest);
    assert_eq!(state.layout.sidebar_width, 240);
    assert_eq!(state.geometry.width, 1200);
}
