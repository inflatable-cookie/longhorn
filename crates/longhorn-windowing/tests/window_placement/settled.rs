use longhorn_windowing::{
    HomeDisplayAdoption, HomeDisplayChange, WindowRole, propose_settled_placement,
};

use super::support::*;

#[test]
fn settled_move_always_proposes_attached_display_memory_without_mutation() {
    let config =
        config(WindowRole::RequiredPrimary).with_home_display(Some(display_id("display:home")));
    let before = config.clone();
    let normal = placement(1700, 40, 900, 700);

    let proposal = propose_settled_placement(
        &config,
        display_id("display:attached"),
        normal,
        false,
        HomeDisplayAdoption::Preserve,
    );

    assert_eq!(config, before);
    assert_eq!(
        proposal.memory_update().display_id(),
        &display_id("display:attached")
    );
    assert_eq!(proposal.memory_update().normal_placement(), normal);
    assert_eq!(
        proposal.home_display_change(),
        &HomeDisplayChange::Unchanged
    );
}

#[test]
fn configured_home_changes_only_under_explicit_adoption_policy() {
    let config =
        config(WindowRole::RequiredPrimary).with_home_display(Some(display_id("display:home")));

    let proposal = propose_settled_placement(
        &config,
        display_id("display:new-home"),
        placement(20, 30, 800, 600),
        false,
        HomeDisplayAdoption::AdoptAttachedDisplay,
    );

    assert_eq!(
        proposal.home_display_change(),
        &HomeDisplayChange::Adopt {
            display_id: display_id("display:new-home"),
        }
    );

    let unchanged = propose_settled_placement(
        &config,
        display_id("display:home"),
        placement(20, 30, 800, 600),
        false,
        HomeDisplayAdoption::AdoptAttachedDisplay,
    );
    assert_eq!(
        unchanged.home_display_change(),
        &HomeDisplayChange::Unchanged
    );
}

#[test]
fn maximized_settled_state_retains_separate_normal_placement() {
    let normal = placement(-900, 60, 1000, 700);
    let proposal = propose_settled_placement(
        &config(WindowRole::RequiredPrimary),
        display_id("display:side"),
        normal,
        true,
        HomeDisplayAdoption::Preserve,
    );

    assert!(proposal.is_maximized());
    assert_eq!(proposal.memory_update().normal_placement(), normal);
}
