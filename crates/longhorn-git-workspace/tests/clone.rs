//! Clone staging and the same-parent exclusive rename, through the public API,
//! over real local Git: staging names, success, an existing target, a refused
//! clone, cancellation and redacted diagnostics.

mod support;

use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

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

#[cfg(unix)]
#[test]
fn a_cancelled_clone_returns_at_the_cancel_even_when_the_remote_hangs() {
    // A remote that is a two-second local program: Git waits on it, and its
    // process is what holds the clone's output pipe.
    let fixture = Fixture::worktree();
    let policy = fixture.policy().with_isolation(vec![
        ("GIT_CONFIG_COUNT".to_owned(), "1".to_owned()),
        (
            "GIT_CONFIG_KEY_0".to_owned(),
            "protocol.ext.allow".to_owned(),
        ),
        ("GIT_CONFIG_VALUE_0".to_owned(), "always".to_owned()),
    ]);
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let setter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        flag.store(true, Ordering::SeqCst);
    });

    let started = Instant::now();
    let failure = git
        .clone_into("ext::sleep 2", &staging, &cancel, None)
        .expect_err("the caller cancelled");
    let elapsed = started.elapsed();
    setter.join().expect("the cancelling thread");
    remove_owned_staging(&staging);

    assert!(matches!(failure, CloneFailure::Cancelled), "{failure:?}");
    assert!(
        elapsed < Duration::from_secs(1),
        "cancellation returned after {elapsed:?}"
    );
    assert!(staging_leftovers(&parent).is_empty());
}

#[cfg(unix)]
#[test]
fn a_cancel_during_the_stderr_drain_returns_at_the_cancel() {
    // The remote program exits at once and Git refuses, but a backgrounded
    // descendant keeps Git's standard error open for three seconds.
    let fixture = Fixture::worktree();
    let program = support::script(
        fixture.temp.path(),
        "orphan.sh",
        "sleep 3 </dev/null >/dev/null &\nexit 1",
    );
    let policy = fixture.policy().with_isolation(vec![
        ("GIT_CONFIG_COUNT".to_owned(), "1".to_owned()),
        (
            "GIT_CONFIG_KEY_0".to_owned(),
            "protocol.ext.allow".to_owned(),
        ),
        ("GIT_CONFIG_VALUE_0".to_owned(), "always".to_owned()),
    ]);
    let git = handle(&policy);
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let setter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::SeqCst);
    });

    let started = Instant::now();
    let remote = format!("ext::{}", program.display());
    let failure = git
        .clone_into(&remote, &staging, &cancel, None)
        .expect_err("the caller cancelled");
    let elapsed = started.elapsed();
    setter.join().expect("the cancelling thread");
    remove_owned_staging(&staging);

    assert!(matches!(failure, CloneFailure::Cancelled), "{failure:?}");
    assert!(
        elapsed < Duration::from_millis(1500),
        "cancellation returned after {elapsed:?}"
    );
    assert!(staging_leftovers(&parent).is_empty());
}
