//! Clone staging and the same-parent exclusive rename, through the public API,
//! over real local Git: staging names, success, an existing target, a refused
//! clone, cancellation and redacted diagnostics.

mod support;

use std::{fs, path::Path, sync::atomic::AtomicBool};

use longhorn_git_workspace::{
    CloneFailure, GitCredential, create_owned_staging, remove_owned_staging, rename_exclusive,
    staging_parent, target_already_present,
};
use support::{Fixture, handle, staging_leftovers};

#[test]
fn staging_is_named_beside_its_target_and_never_repeats() {
    let fixture = Fixture::worktree();
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");

    let first = create_owned_staging(&parent, std::ffi::OsStr::new("workspace"))
        .expect("one staging folder");
    let second = create_owned_staging(&parent, std::ffi::OsStr::new("workspace"))
        .expect("a second staging folder");

    assert_ne!(first, second);
    assert!(first.is_dir());
    assert!(
        first
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("workspace.creating-"))
    );
    assert_eq!(staging_parent(&first), Some(parent.as_path()));
    assert_eq!(staging_parent(Path::new("workspace")), Some(Path::new(".")));
}

#[test]
fn clones_into_staging_and_moves_the_workspace_into_place() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let target = parent.join("workspace");
    assert!(!target_already_present(&target));
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");

    git.clone_into(
        fixture.remote.to_str().expect("a remote path"),
        &staging,
        &AtomicBool::new(false),
        None,
    )
    .expect("the local bare remote clones");
    rename_exclusive(&staging, &target).expect("the target is absent");

    assert!(target.join("content.md").is_file());
    assert!(!staging.exists());
    assert!(staging_leftovers(&parent).is_empty());
}

#[test]
fn an_existing_target_is_refused_and_left_untouched() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let target = parent.join("workspace");
    fs::create_dir(&target).expect("a planted target");
    fs::write(target.join("keep.md"), "keep\n").expect("planted content");
    assert!(target_already_present(&target));
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    git.clone_into(
        fixture.remote.to_str().expect("a remote path"),
        &staging,
        &AtomicBool::new(false),
        None,
    )
    .expect("the local bare remote clones");

    let error = rename_exclusive(&staging, &target).expect_err("the target already exists");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(target.join("keep.md")).expect("the planted path"),
        "keep\n"
    );
    remove_owned_staging(&staging);
    assert!(!staging.exists());
    assert!(staging_leftovers(&parent).is_empty());
}

#[test]
fn a_refused_clone_leaves_no_partial_target() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let target = parent.join("workspace");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let absent_remote = fixture.temp.path().join("absent-remote.git");

    let failure = git
        .clone_into(
            absent_remote.to_str().expect("a fixture path"),
            &staging,
            &AtomicBool::new(false),
            None,
        )
        .expect_err("the remote does not exist");
    remove_owned_staging(&staging);

    let CloneFailure::Refused { detail } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(detail.contains("does not exist"), "{detail}");
    assert!(!target.exists());
    assert!(!staging.exists());
    assert!(staging_leftovers(&parent).is_empty());
}

#[test]
fn a_refused_clone_redacts_the_supplied_secret() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let absent_remote = fixture.temp.path().join("controlled-secret-remote.git");
    let credential = GitCredential::new(Vec::new(), "controlled-secret");

    let failure = git
        .clone_into(
            absent_remote.to_str().expect("a fixture path"),
            &staging,
            &AtomicBool::new(false),
            Some(&credential),
        )
        .expect_err("the remote does not exist");
    remove_owned_staging(&staging);

    let CloneFailure::Refused { detail } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(!detail.contains("controlled-secret"), "{detail}");
    assert!(detail.contains("[redacted]"), "{detail}");
}

#[test]
fn a_cancelled_clone_starts_nothing_and_keeps_no_staging() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let target = parent.join("workspace");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");

    let failure = git
        .clone_into(
            fixture.remote.to_str().expect("a remote path"),
            &staging,
            &AtomicBool::new(true),
            None,
        )
        .expect_err("the caller cancelled before the clone");
    remove_owned_staging(&staging);

    assert!(matches!(failure, CloneFailure::Cancelled));
    assert!(!target.exists());
    assert!(staging_leftovers(&parent).is_empty());
}
