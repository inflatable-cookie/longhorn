//! Clone staging and the same-parent exclusive rename, through the public API,
//! over real local Git: staging names, success, an existing target, a refused
//! clone, cancellation and redacted diagnostics.

mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use longhorn_git_workspace::{
    CloneFailure, GitCredential, create_owned_staging, rename_exclusive, staging_parent,
    target_already_present,
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

    assert_ne!(first.path(), second.path());
    assert!(first.path().is_dir());
    assert!(
        first
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("workspace.creating-"))
    );
    assert_eq!(staging_parent(first.path()), Some(parent.as_path()));
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
        staging.path(),
        &AtomicBool::new(false),
        None,
    )
    .expect("the local bare remote clones");
    rename_exclusive(staging.path(), &target).expect("the target is absent");

    assert!(target.join("content.md").is_file());
    assert!(!staging.path().exists());
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
        staging.path(),
        &AtomicBool::new(false),
        None,
    )
    .expect("the local bare remote clones");

    let error = rename_exclusive(staging.path(), &target).expect_err("the target already exists");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(target.join("keep.md")).expect("the planted path"),
        "keep\n"
    );
    let staging_path = staging.path().to_path_buf();
    staging
        .remove()
        .expect("the invocation removes its own staging");
    assert!(!staging_path.exists());
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
            staging.path(),
            &AtomicBool::new(false),
            None,
        )
        .expect_err("the remote does not exist");
    let staging_path = staging.path().to_path_buf();
    staging
        .remove()
        .expect("the invocation removes its own staging");

    let CloneFailure::Refused { detail } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(detail.contains("does not exist"), "{detail}");
    assert!(!target.exists());
    assert!(!staging_path.exists());
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
            staging.path(),
            &AtomicBool::new(false),
            Some(&credential),
        )
        .expect_err("the remote does not exist");
    let staging_path = staging.path().to_path_buf();
    staging
        .remove()
        .expect("the invocation removes its own staging");

    let CloneFailure::Refused { detail } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(!detail.contains("controlled-secret"), "{detail}");
    assert!(detail.contains("[redacted]"), "{detail}");
    assert!(!staging_path.exists());
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
            staging.path(),
            &AtomicBool::new(true),
            None,
        )
        .expect_err("the caller cancelled before the clone");
    let staging_path = staging.path().to_path_buf();
    staging
        .remove()
        .expect("the invocation removes its own staging");

    assert!(matches!(failure, CloneFailure::Cancelled));
    assert!(!target.exists());
    assert!(!staging_path.exists());
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
        .clone_into("ext::sleep 2", staging.path(), &cancel, None)
        .expect_err("the caller cancelled");
    let elapsed = started.elapsed();
    setter.join().expect("the cancelling thread");
    staging
        .remove()
        .expect("the invocation removes its own staging");

    assert!(matches!(failure, CloneFailure::Cancelled), "{failure:?}");
    assert!(
        elapsed < Duration::from_secs(1),
        "cancellation returned after {elapsed:?}"
    );
    assert!(staging_leftovers(&parent).is_empty());
}

#[cfg(unix)]
/// What one cancelled clone left behind.
struct DrainCancel {
    failure: CloneFailure,
    elapsed: Duration,
    leftovers: Vec<PathBuf>,
    zombies: Vec<String>,
}

/// Clones with a remote program that exits at once while a backgrounded
/// descendant keeps Git's standard error open, and cancels after 300 ms.
fn cancel_clone_during_stderr_drain(fixture: &Fixture) -> DrainCancel {
    // The remote program exits at once and Git refuses, but a backgrounded
    // descendant keeps Git's standard error open for three seconds.
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
    // Cancel only once Git has exited while its standard error is still held:
    // that is the drain this helper exists to exercise.
    #[cfg(unix)]
    let zombies_before = zombie_children();
    #[cfg(unix)]
    let before_for_setter = zombies_before.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let setter = thread::spawn(move || {
        #[cfg(unix)]
        wait_for_new_zombie(&before_for_setter, Duration::from_secs(10));
        #[cfg(not(unix))]
        thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::SeqCst);
    });

    let started = Instant::now();
    let remote = format!("ext::{}", program.display());
    let failure = git
        .clone_into(&remote, staging.path(), &cancel, None)
        .expect_err("the caller cancelled");
    let elapsed = started.elapsed();
    // A clone running beside this one in the same process can hold a zombie
    // for one of its own polls; that clears at once. A leaked child does not,
    // so the check waits about a second before it reports one.
    #[cfg(unix)]
    let zombies: Vec<String> = {
        let mut found = Vec::new();
        for _ in 0..50 {
            found = zombie_children()
                .into_iter()
                .filter(|pid| !zombies_before.contains(pid))
                .collect();
            if found.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        found
    };
    #[cfg(not(unix))]
    let zombies = Vec::new();
    setter.join().expect("the cancelling thread");
    staging
        .remove()
        .expect("the invocation removes its own staging");

    DrainCancel {
        failure,
        elapsed,
        leftovers: staging_leftovers(&parent),
        zombies,
    }
}

/// Waits, for at most `bound`, until this process has a zombie child that was
/// not in `before`: the clone's own child, exited and not yet reaped.
#[cfg(unix)]
fn wait_for_new_zombie(before: &[String], bound: Duration) {
    let started = Instant::now();
    while started.elapsed() < bound {
        if zombie_children().iter().any(|pid| !before.contains(pid)) {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// The process ids of this process's zombie children. `ps` is on macOS and
/// Linux; a zombie shows no usable name or arguments there, so only its state
/// and parent identify it.
#[cfg(unix)]
fn zombie_children() -> Vec<String> {
    let output = std::process::Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,stat="])
        .output()
        .expect("ps lists the process table");
    let me = std::process::id().to_string();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields.len() >= 3 && fields[1] == me && fields[2].starts_with('Z'))
                .then(|| fields[0].to_owned())
        })
        .collect()
}

#[test]
fn a_cancel_during_the_stderr_drain_returns_at_the_cancel() {
    let fixture = Fixture::worktree();

    let cancelled = cancel_clone_during_stderr_drain(&fixture);

    assert!(
        matches!(cancelled.failure, CloneFailure::Cancelled),
        "{:?}",
        cancelled.failure
    );
    assert!(
        cancelled.elapsed < Duration::from_millis(1500),
        "cancellation returned after {:?}",
        cancelled.elapsed
    );
    assert!(cancelled.leftovers.is_empty());
}

#[cfg(unix)]
#[test]
fn a_cancel_during_the_stderr_drain_reaps_the_clone_child() {
    let fixture = Fixture::worktree();

    let cancelled = cancel_clone_during_stderr_drain(&fixture);

    assert!(
        matches!(cancelled.failure, CloneFailure::Cancelled),
        "{:?}",
        cancelled.failure
    );
    assert!(
        cancelled.zombies.is_empty(),
        "the cancelled clone left its child unreaped: {:?}",
        cancelled.zombies
    );
}

#[test]
fn cleanup_removes_only_the_folder_the_creator_made() {
    let fixture = Fixture::worktree();
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let target = parent.join("workspace");
    fs::create_dir(&target).expect("a planted destination");
    fs::write(target.join("keep.md"), "keep\n").expect("planted content");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let staging_path = staging.path().to_path_buf();
    fs::write(staging.path().join("partial.md"), "partial\n").expect("partial content");

    staging
        .remove()
        .expect("the invocation removes its own staging");

    assert!(!staging_path.exists());
    assert_eq!(
        fs::read_to_string(target.join("keep.md")).expect("the planted path"),
        "keep\n"
    );
    assert!(staging_leftovers(&parent).is_empty());
}

#[cfg(unix)]
#[test]
fn a_failed_cleanup_reports_the_error_and_leaves_the_folder() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::worktree();
    let parent = fixture.temp.path().join("destination");
    fs::create_dir(&parent).expect("a destination parent");
    let staging =
        create_owned_staging(&parent, std::ffi::OsStr::new("workspace")).expect("staging");
    let staging_path = staging.path().to_path_buf();
    let original = fs::metadata(&parent)
        .expect("the destination parent")
        .permissions();
    // A parent that refuses the unlink: the recursive removal cannot empty the
    // folder's entry out of it.
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555))
        .expect("a read-only destination parent");

    let error = staging
        .remove()
        .expect_err("the read-only parent refuses the removal");

    fs::set_permissions(&parent, original).expect("the destination parent is writable again");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(
        error.to_string().contains("workspace.creating-"),
        "the failure names the staging folder: {error}"
    );
    assert!(staging_path.is_dir(), "the staging folder is still there");
    fs::remove_dir_all(&staging_path).expect("the fixture removes its own staging");
    assert!(staging_leftovers(&parent).is_empty());
}
