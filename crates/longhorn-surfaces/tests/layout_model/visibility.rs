use longhorn_surfaces::{
    RegionVisibilityState, VisibilityProjectionError, project_region_visibility,
};

use super::support::*;

#[test]
fn occupancy_and_empty_policy_drive_normal_visibility() {
    let projection = project_region_visibility(
        &registry(),
        &document(),
        &surface_id("surface:primary"),
        None,
    )
    .unwrap();

    assert_eq!(
        projection
            .iter()
            .map(|region| (region.region_id().as_str(), region.state()))
            .collect::<Vec<_>>(),
        vec![
            ("left", RegionVisibilityState::Visible),
            ("center", RegionVisibilityState::Visible),
            ("right", RegionVisibilityState::Hidden),
        ]
    );
}

#[test]
fn eligible_movable_panel_transiently_reveals_without_mutation() {
    let registry = registry();
    let document = document();
    let before = serde_json::to_vec(&document).unwrap();
    let revision = document.revision();

    let projection = project_region_visibility(
        &registry,
        &document,
        &surface_id("surface:primary"),
        Some(&definition_id("panel:tool")),
    )
    .unwrap();

    assert_eq!(
        projection[2].state(),
        RegionVisibilityState::TransientlyRevealed
    );
    assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    assert_eq!(document.revision(), revision);
}

#[test]
fn immovable_or_ineligible_panels_do_not_reveal_hidden_regions() {
    let projection = project_region_visibility(
        &registry(),
        &document(),
        &surface_id("surface:primary"),
        Some(&definition_id("panel:activity")),
    )
    .unwrap();

    assert_eq!(projection[2].state(), RegionVisibilityState::Hidden);
}

#[test]
fn projection_rejects_unknown_targets() {
    let unknown_surface = project_region_visibility(
        &registry(),
        &document(),
        &surface_id("surface:missing"),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        unknown_surface,
        VisibilityProjectionError::UnknownSurface(_)
    ));

    let unknown_definition = project_region_visibility(
        &registry(),
        &document(),
        &surface_id("surface:primary"),
        Some(&definition_id("panel:missing")),
    )
    .unwrap_err();
    assert!(matches!(
        unknown_definition,
        VisibilityProjectionError::UnknownPanelDefinition(_)
    ));
}
