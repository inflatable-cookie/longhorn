//! Legacy Sync through the public API, over real local Git and a bare local
//! remote: clean, upstream-only, fetch-then-compare, `--ff-only` forward,
//! ahead-only push.

mod support;

use std::path::Path;

use longhorn_git_workspace::{GitFailureCode, SyncReading, SyncStop};
use support::{Fixture, checked, commit, handle};

fn sync(git: &longhorn_git_workspace::Git<'_>, fixture: &Fixture) -> SyncReading {
    let before = git
        .inspect_worktree(&fixture.root)
        .expect("the fixture is a worktree");
    git.sync_workspace(before, |root| git.inspect_worktree(Path::new(root)))
        .expect("the local remote answers")
}

#[test]
fn reports_an_up_to_date_workspace() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let reading = sync(&git, &fixture);

    let SyncReading::Completed {
        fetched,
        fast_forwarded,
        pushed,
        ..
    } = reading
    else {
        panic!("expected a completed sync, got {reading:?}");
    };
    assert!(fetched);
    assert!(!fast_forwarded);
    assert!(!pushed);
}

#[test]
fn pushes_an_ahead_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    commit(
        &fixture.root,
        &fixture.config,
        "local.md",
        "local\n",
        "Local change",
    );

    let reading = sync(&git, &fixture);

    let SyncReading::Completed {
        pushed, ref state, ..
    } = reading
    else {
        panic!("expected a completed sync, got {reading:?}");
    };
    assert!(pushed);
    assert_eq!(state.status.ahead, 0);
    assert_eq!(
        checked(&fixture.root, &fixture.config, &["rev-parse", "HEAD"]),
        checked(
            &fixture.root,
            &fixture.config,
            &["rev-parse", "origin/main"]
        )
    );
}

#[test]
fn fast_forwards_a_behind_branch() {
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

    let reading = sync(&git, &fixture);

    let SyncReading::Completed {
        fast_forwarded,
        pushed,
        ..
    } = reading
    else {
        panic!("expected a completed sync, got {reading:?}");
    };
    assert!(fast_forwarded);
    assert!(!pushed);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("incoming.md")).expect("the fetched path"),
        "incoming\n"
    );
}

#[test]
fn stops_on_divergent_history_with_both_distances() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let peer = fixture.peer();
    commit(
        &fixture.root,
        &fixture.config,
        "local.md",
        "local\n",
        "Local change",
    );
    commit(
        &peer,
        &fixture.config,
        "incoming.md",
        "incoming\n",
        "Incoming",
    );
    checked(&peer, &fixture.config, &["push"]);

    let reading = sync(&git, &fixture);

    let SyncReading::Stopped {
        stop,
        ref state,
        fetched,
        fast_forwarded,
    } = reading
    else {
        panic!("expected a stopped sync, got {reading:?}");
    };
    assert_eq!(stop, SyncStop::DivergedAfterFetch);
    assert!(fetched);
    assert!(!fast_forwarded);
    assert_eq!(state.status.ahead, 1);
    assert_eq!(state.status.behind, 1);
}

#[test]
fn stops_on_dirty_detached_and_upstream_less_workspaces() {
    let dirty = Fixture::worktree();
    std::fs::write(dirty.root.join("content.md"), "dirty\n").expect("dirty content");
    let dirty_policy = dirty.policy();
    let dirty_git = handle(&dirty_policy);
    let SyncReading::Stopped { stop, fetched, .. } = sync(&dirty_git, &dirty) else {
        panic!("a dirty worktree must stop the sync");
    };
    assert_eq!(stop, SyncStop::Dirty);
    assert!(!fetched);

    let detached = Fixture::worktree();
    checked(&detached.root, &detached.config, &["switch", "--detach"]);
    let detached_policy = detached.policy();
    let detached_git = handle(&detached_policy);
    let SyncReading::Stopped { stop, fetched, .. } = sync(&detached_git, &detached) else {
        panic!("a detached checkout must stop the sync");
    };
    assert_eq!(stop, SyncStop::Detached);
    assert!(!fetched);

    let upstream_less = Fixture::worktree();
    checked(
        &upstream_less.root,
        &upstream_less.config,
        &["branch", "--unset-upstream", "main"],
    );
    let upstream_less_policy = upstream_less.policy();
    let upstream_less_git = handle(&upstream_less_policy);
    let SyncReading::Stopped { stop, fetched, .. } = sync(&upstream_less_git, &upstream_less)
    else {
        panic!("a branch without an upstream must stop the sync");
    };
    assert_eq!(stop, SyncStop::NoUpstream);
    assert!(!fetched);
}

#[test]
fn returns_a_stable_fetch_refusal() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    checked(
        &fixture.root,
        &fixture.config,
        &[
            "remote",
            "set-url",
            "origin",
            "/missing/longhorn-remote.git",
        ],
    );
    let before = git
        .inspect_worktree(&fixture.root)
        .expect("the fixture is a worktree");

    let failure = git
        .sync_workspace(before, |root| git.inspect_worktree(Path::new(root)))
        .expect_err("the remote does not exist");

    assert_eq!(failure.code(), GitFailureCode::FetchFailed);
    assert!(failure.detail().contains("longhorn-remote.git"));
}
