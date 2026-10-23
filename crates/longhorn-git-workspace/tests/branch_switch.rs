//! The explicit branch switch through the public API: local branches, remote
//! branches, an existing tracking branch, already-current, dirt and a missing
//! reference.

mod support;

use std::{fs, path::Path};

use longhorn_git_workspace::{GitFailureCode, SwitchReading};
use support::{Fixture, checked, handle};

fn switch(
    git: &longhorn_git_workspace::Git<'_>,
    fixture: &Fixture,
    branch_id: &str,
) -> Result<SwitchReading, longhorn_git_workspace::GitFailure> {
    let before = git
        .inspect_worktree(&fixture.root)
        .expect("the fixture is a worktree");
    git.switch_branch(before, branch_id, |root| {
        git.inspect_worktree(Path::new(root))
    })
}

#[test]
fn switches_to_a_clean_local_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let reading = switch(&git, &fixture, "local:client-a").expect("the branch exists");

    let SwitchReading::Switched { state } = reading else {
        panic!("expected a switch, got {reading:?}");
    };
    assert_eq!(state.current_branch, "client-a");
}

#[test]
fn creates_a_local_branch_from_a_remote_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let reading = switch(&git, &fixture, "remote:origin/client-b").expect("the branch exists");

    let SwitchReading::Switched { state } = reading else {
        panic!("expected a switch, got {reading:?}");
    };
    assert_eq!(state.current_branch, "client-b");
    assert_eq!(state.upstream.as_deref(), Some("origin/client-b"));
}

#[test]
fn a_remote_selection_uses_an_existing_tracking_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    switch(&git, &fixture, "local:client-a").expect("the branch exists");

    let reading = switch(&git, &fixture, "remote:origin/main").expect("the branch exists");

    let SwitchReading::Switched { state } = reading else {
        panic!("expected a switch, got {reading:?}");
    };
    assert_eq!(state.current_branch, "main");
}

#[test]
fn reports_the_current_branch_without_running_anything() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let reading = switch(&git, &fixture, "local:main").expect("the branch exists");

    let SwitchReading::AlreadyCurrent { branch, state } = reading else {
        panic!("expected an unchanged reading, got {reading:?}");
    };
    assert_eq!(branch, "main");
    assert_eq!(state.current_branch, "main");
}

#[test]
fn blocks_a_switch_while_the_worktree_holds_dirty_work() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    fs::write(fixture.root.join("content.md"), "local work\n").expect("a modified path");

    let reading = switch(&git, &fixture, "local:client-a").expect("the branch exists");

    let SwitchReading::Blocked { state } = reading else {
        panic!("expected a blocked switch, got {reading:?}");
    };
    assert_eq!(state.current_branch, "main");
    assert_eq!(state.status.modified, 1);
}

#[test]
fn refuses_a_branch_that_was_not_observed() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = switch(&git, &fixture, "local:absent").expect_err("the branch does not exist");

    assert_eq!(failure.code(), GitFailureCode::BranchNotFound);
}

#[test]
fn a_switch_leaves_the_worktree_on_the_requested_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    switch(&git, &fixture, "local:client-a").expect("the branch exists");

    assert_eq!(
        checked(
            &fixture.root,
            &fixture.config,
            &["symbolic-ref", "--short", "HEAD"]
        ),
        "client-a"
    );
}
