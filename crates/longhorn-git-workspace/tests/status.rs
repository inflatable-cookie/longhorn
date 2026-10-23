//! Worktree status and branch reading, through the public API, over real Git.

mod support;

use std::fs;

use longhorn_git_workspace::{
    GitBranchKind, GitFailure, GitFailureCode, GitStatus, has_switch_blocking_dirt,
};
use support::{Fixture, checked, commit, handle, succeeds};

#[test]
fn reads_local_and_remote_branches_and_a_clean_status() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("the fixture is a worktree");

    assert_eq!(state.current_branch, "main");
    assert!(!state.detached);
    assert_eq!(state.upstream.as_deref(), Some("origin/main"));
    assert!(state.status.clean);
    assert!(state.has_named_ref("client-a"));
    assert!(state.branches.iter().any(|branch| branch.name == "client-a"
        && branch.kind == GitBranchKind::Local
        && !branch.current
        && branch.local_target.is_none()));
    assert!(
        state
            .branches
            .iter()
            .any(|branch| branch.name == "origin/client-b" && branch.kind == GitBranchKind::Remote)
    );
    // The current branch is the one local branch marked current.
    assert_eq!(
        state
            .branches
            .iter()
            .filter(|branch| branch.current)
            .count(),
        1
    );
}

#[test]
fn counts_staged_modified_and_untracked_work() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    fs::write(fixture.root.join("content.md"), "modified\n").expect("a modified path");
    fs::write(fixture.root.join("staged.md"), "staged\n").expect("a staged path");
    fs::write(fixture.root.join("untracked.md"), "untracked\n").expect("an untracked path");
    checked(&fixture.root, &fixture.config, &["add", "staged.md"]);

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert_eq!(
        state.status,
        GitStatus {
            staged: 1,
            modified: 1,
            untracked: 1,
            conflicted: 0,
            ahead: 0,
            behind: 0,
            clean: false,
        }
    );
    assert!(has_switch_blocking_dirt(&state));
}

#[test]
fn untracked_content_alone_does_not_block_a_switch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    fs::write(fixture.root.join("untracked.md"), "untracked\n").expect("an untracked path");

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert_eq!(state.status.untracked, 1);
    assert!(!state.status.clean);
    assert!(!has_switch_blocking_dirt(&state));
}

#[test]
fn reads_one_conflict_and_blocks_a_switch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    commit(
        &fixture.root,
        &fixture.config,
        "content.md",
        "main\n",
        "Main change",
    );
    checked(&fixture.root, &fixture.config, &["switch", "client-a"]);
    commit(
        &fixture.root,
        &fixture.config,
        "content.md",
        "client\n",
        "Client change",
    );
    checked(&fixture.root, &fixture.config, &["switch", "main"]);
    assert!(!succeeds(
        &fixture.root,
        &fixture.config,
        &["merge", "client-a"]
    ));

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert_eq!(state.status.conflicted, 1);
    assert!(!state.status.clean);
    assert!(has_switch_blocking_dirt(&state));
}

#[test]
fn reads_ahead_and_behind_distances_from_the_upstream() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let peer = fixture.peer();
    commit(
        &peer,
        &fixture.config,
        "incoming.md",
        "incoming\n",
        "Incoming",
    );
    checked(&peer, &fixture.config, &["push"]);
    let _ = git
        .run(
            &fixture.root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            std::time::Duration::from_secs(60),
        )
        .expect("a local fetch");
    commit(
        &fixture.root,
        &fixture.config,
        "local.md",
        "local\n",
        "Local change",
    );

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert_eq!(state.status.ahead, 1);
    assert_eq!(state.status.behind, 1);
}

#[test]
fn reads_a_detached_checkout_as_its_revision_without_an_upstream() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    checked(&fixture.root, &fixture.config, &["switch", "--detach"]);
    let revision = checked(
        &fixture.root,
        &fixture.config,
        &["rev-parse", "--short", "HEAD"],
    );

    let state = git
        .inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert!(state.detached);
    assert_eq!(state.current_branch, revision);
    assert_eq!(state.upstream, None);
    assert!(state.branches.iter().all(|branch| !branch.current));
}

#[test]
fn refuses_a_folder_that_is_not_a_worktree() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let plain = fixture.temp.path().join("plain");
    fs::create_dir(&plain).expect("a plain folder");

    let failure = git
        .inspect_worktree(&plain)
        .expect_err("a plain folder is not a worktree");

    assert_eq!(failure.code(), GitFailureCode::RepositoryUnavailable);
    assert!(!failure.detail().is_empty());
}

#[test]
fn refuses_a_folder_that_does_not_exist() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = git
        .inspect_worktree(&fixture.temp.path().join("absent"))
        .expect_err("an absent folder cannot be read");

    assert_eq!(failure.code(), GitFailureCode::RepositoryUnavailable);
    assert!(matches!(failure, GitFailure::FolderUnavailable { .. }));
}
