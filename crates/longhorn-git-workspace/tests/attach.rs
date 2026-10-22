//! Attach reading over explicit expected facts, through the public API: ready,
//! a switch offer, a dirty wrong branch, a wrong remote, a plain folder,
//! detached checkouts and remote comparison.

mod support;

use std::fs;

use longhorn_git_workspace::{AttachReading, remote_for_display, remote_identity, remotes_match};
use support::{Fixture, checked, handle};

const EXPECTED_BRANCH: &str = "client/tom";

#[test]
fn a_checkout_on_the_expected_branch_is_ready() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    checked(
        &fixture.root,
        &fixture.config,
        &["switch", "-c", EXPECTED_BRANCH],
    );
    let expected_remote = fixture.remote.to_str().expect("a fixture path");

    let reading = git.read_attach(&fixture.root, EXPECTED_BRANCH, expected_remote);

    let AttachReading::Ready { root } = reading else {
        panic!("expected a ready reading, got {reading:?}");
    };
    assert_eq!(
        root,
        fixture
            .root
            .canonicalize()
            .expect("the canonical root")
            .to_string_lossy()
    );
}

#[test]
fn a_clean_wrong_branch_offers_a_switch_that_creates_the_expected_branch() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::SwitchOffered {
        current_branch,
        create_from_integration_base,
        ..
    } = reading
    else {
        panic!("expected a switch offer, got {reading:?}");
    };
    assert_eq!(current_branch, "main");
    assert!(create_from_integration_base);
}

#[test]
fn an_observed_expected_branch_needs_no_creation() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    checked(&fixture.root, &fixture.config, &["branch", EXPECTED_BRANCH]);

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::SwitchOffered {
        create_from_integration_base,
        ..
    } = reading
    else {
        panic!("expected a switch offer, got {reading:?}");
    };
    assert!(!create_from_integration_base);
}

#[test]
fn untracked_content_is_in_progress_work_not_dirt() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    fs::write(fixture.root.join("content/notes/draft.md"), "draft\n").expect("a draft");

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    assert!(
        matches!(reading, AttachReading::SwitchOffered { .. }),
        "untracked content must still offer a switch, got {reading:?}"
    );
}

#[test]
fn a_dirty_wrong_branch_is_refused_without_mutation() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    fs::write(fixture.root.join("content.md"), "edited\n").expect("edited content");

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::WrongBranchDirty {
        current_branch,
        detached,
        ..
    } = reading
    else {
        panic!("expected a dirty refusal, got {reading:?}");
    };
    assert_eq!(current_branch, "main");
    assert!(!detached);
    assert_eq!(
        checked(
            &fixture.root,
            &fixture.config,
            &["symbolic-ref", "--short", "HEAD"]
        ),
        "main"
    );
}

#[test]
fn a_wrong_remote_is_refused_with_its_display_form() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let other = fixture.temp.path().join("other.git");
    checked(
        &fixture.root,
        &fixture.config,
        &[
            "remote",
            "set-url",
            "origin",
            other.to_str().expect("a path"),
        ],
    );

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::WrongRemote { observed, .. } = reading else {
        panic!("expected a wrong remote, got {reading:?}");
    };
    assert_eq!(observed.as_deref(), other.to_str());
}

#[test]
fn a_folder_that_is_not_a_repository_is_refused_with_its_failure() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let plain = fixture.temp.path().join("plain");
    fs::create_dir(&plain).expect("a plain folder");

    let reading = git.read_attach(
        &plain,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::NotRepository { error, .. } = reading else {
        panic!("expected a not-a-repository refusal, got {reading:?}");
    };
    assert_eq!(
        error.code(),
        longhorn_git_workspace::GitFailureCode::RepositoryUnavailable
    );
}

#[test]
fn a_clean_detached_checkout_offers_the_same_reattach() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    checked(
        &fixture.root,
        &fixture.config,
        &["checkout", "--detach", "HEAD"],
    );

    let reading = git.read_attach(
        &fixture.root,
        EXPECTED_BRANCH,
        fixture.remote.to_str().expect("a fixture path"),
    );

    let AttachReading::SwitchOffered { detached, .. } = reading else {
        panic!("expected a reattach offer, got {reading:?}");
    };
    assert!(detached);
}

#[test]
fn remote_aliases_name_the_same_repository() {
    for observed in [
        "git@github.com:example/content.git",
        "git@github.com:example/content",
        "https://github.com/example/content.git",
        "https://github.com/example/content",
        "ssh://git@github.com/example/content.git",
        "HTTPS://GitHub.com/example/content/",
    ] {
        assert!(
            remotes_match(observed, "https://github.com/example/content.git"),
            "{observed}"
        );
    }
    assert!(!remotes_match(
        "git@github.com:example/content-desktop.git",
        "https://github.com/example/content.git"
    ));
    assert_eq!(
        remote_identity("https://user@github.com/example/content.git"),
        "github.com/example/content"
    );
    assert_eq!(
        remote_for_display("https://x-access-token:secret@github.com/example/content.git"),
        "https://github.com/example/content.git"
    );
    assert_eq!(
        remote_for_display("ssh://git@github.com/example/content.git"),
        "ssh://github.com/example/content.git"
    );
}
