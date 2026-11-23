//! Exact worktree and repository identity reading, through the public API, over real Git.

mod support;

use std::{fs, path::Path, time::Duration};

use longhorn_git_workspace::{Git, GitFailure, GitFailureCode};
use longhorn_redact::{RedactionRules, Redactor};
use support::{Fixture, checked, commit, handle, redactor};

#[test]
fn distinct_repositories_whose_paths_redact_identically_compare_different() {
    let temp = tempfile::tempdir().expect("a test temp root");
    let config = temp.path().join("gitconfig");
    fs::write(&config, "# empty fixture config\n").expect("an empty config");

    let repo_a = temp.path().join("ghp_alphaRepoSecret123/checkout");
    let repo_b = temp.path().join("ghp_betaRepoSecret456/checkout");
    for repo in [&repo_a, &repo_b] {
        fs::create_dir_all(repo).expect("repo folder");
        fs::write(repo.join("README.md"), "init\n").expect("seed file");
        checked(repo, &config, &["init", "-b", "main"]);
        checked(repo, &config, &["config", "user.name", "Longhorn Test"]);
        checked(
            repo,
            &config,
            &["config", "user.email", "test@example.invalid"],
        );
        checked(repo, &config, &["add", "."]);
        checked(repo, &config, &["commit", "-m", "Initial"]);
    }

    let policy = support::Policy::isolated(&config);
    let git = handle(&policy);

    // Prove that redacted `Git::run` output collides for these two repositories.
    let redacted_a = git
        .run(
            &repo_a,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            GitFailureCode::RepositoryUnavailable,
            Duration::from_secs(15),
        )
        .expect("redacted common dir a");
    let redacted_b = git
        .run(
            &repo_b,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            GitFailureCode::RepositoryUnavailable,
            Duration::from_secs(15),
        )
        .expect("redacted common dir b");
    assert_eq!(redacted_a, redacted_b);

    let id_a = git.worktree_identity(&repo_a).expect("identity a");
    let id_b = git.worktree_identity(&repo_b).expect("identity b");

    assert!(!id_a.same_repository(&id_b));
    assert!(!id_a.same_worktree(&id_b));
    assert_ne!(id_a, id_b);
    assert_ne!(id_a.common_dir(), id_b.common_dir());
    assert_ne!(id_a.root(), id_b.root());
    assert_eq!(id_a.branch(), Some("main"));
    assert_eq!(id_b.branch(), Some("main"));

    #[cfg(unix)]
    assert_ne!(
        id_a.common_dir_filesystem_id(),
        id_b.common_dir_filesystem_id()
    );
}

#[test]
fn linked_worktree_and_main_checkout_share_repository_but_not_worktree() {
    let fixture = Fixture::worktree();
    let linked = fixture.temp.path().join("linked-worktree");
    checked(
        &fixture.root,
        &fixture.config,
        &[
            "worktree",
            "add",
            "-b",
            "linked-branch",
            linked.to_str().expect("linked path"),
        ],
    );
    let policy = fixture.policy();
    let git = handle(&policy);

    let main_id = git
        .worktree_identity(&fixture.root)
        .expect("main worktree identity");
    let linked_id = git
        .worktree_identity(&linked)
        .expect("linked worktree identity");

    assert!(main_id.same_repository(&linked_id));
    assert!(linked_id.same_repository(&main_id));
    assert!(!main_id.same_worktree(&linked_id));
    assert!(!linked_id.same_worktree(&main_id));
    assert_eq!(main_id.common_dir(), linked_id.common_dir());
    assert_ne!(main_id.root(), linked_id.root());
    assert_eq!(main_id.branch(), Some("main"));
    assert_eq!(linked_id.branch(), Some("linked-branch"));
    assert_eq!(main_id.head_oid(), linked_id.head_oid());

    #[cfg(unix)]
    {
        assert_eq!(
            main_id.common_dir_filesystem_id(),
            linked_id.common_dir_filesystem_id()
        );
        assert_ne!(main_id.root_filesystem_id(), linked_id.root_filesystem_id());
    }
}

#[test]
fn sibling_roots_with_trailing_space_keep_distinct_identities() {
    let temp = tempfile::tempdir().expect("a test temp root");
    let config = temp.path().join("gitconfig");
    fs::write(&config, "# empty fixture config\n").expect("an empty config");

    for name in ["repository", "repository "] {
        let repo = temp.path().join(name);
        fs::create_dir(&repo).expect("repo folder");
        fs::write(repo.join("README.md"), "init\n").expect("seed file");
        checked(&repo, &config, &["init", "-b", "main"]);
        checked(&repo, &config, &["config", "user.name", "Longhorn Test"]);
        checked(
            &repo,
            &config,
            &["config", "user.email", "test@example.invalid"],
        );
        checked(&repo, &config, &["add", "."]);
        checked(&repo, &config, &["commit", "-m", "Initial"]);
    }

    let policy = support::Policy::isolated(&config);
    let git = handle(&policy);
    let plain = temp.path().join("repository");
    let spaced = temp.path().join("repository ");
    let id_plain = git.worktree_identity(&plain).expect("plain identity");
    let id_spaced = git.worktree_identity(&spaced).expect("spaced identity");

    // Each identity reports its own exact root and common directory.
    assert_eq!(
        id_plain.root(),
        &plain.canonicalize().expect("canonical plain root")
    );
    assert_eq!(
        id_spaced.root(),
        &spaced.canonicalize().expect("canonical spaced root")
    );
    assert!(
        id_spaced.root().to_string_lossy().ends_with("repository "),
        "spaced root keeps its trailing space: {}",
        id_spaced.root().display()
    );
    assert_ne!(id_plain.root(), id_spaced.root());
    assert_ne!(id_plain.common_dir(), id_spaced.common_dir());
    assert!(!id_plain.same_repository(&id_spaced));
    assert!(!id_spaced.same_repository(&id_plain));
    assert!(!id_plain.same_worktree(&id_spaced));
    assert_ne!(id_plain, id_spaced);
}

#[cfg(unix)]
#[test]
fn non_utf8_worktree_name_keeps_exact_bytes() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let temp = tempfile::tempdir().expect("a test temp root");
    let config = temp.path().join("gitconfig");
    fs::write(&config, "# empty fixture config\n").expect("an empty config");

    // `\xff` alone is not valid UTF-8; its lossy rendering is the replacement
    // character, so the sibling below renders identically under a lossy decode.
    let raw_name = b"repo\xff".to_vec();
    let raw_dir = temp.path().join(OsStr::from_bytes(&raw_name));
    if let Err(error) = fs::create_dir(&raw_dir) {
        eprintln!(
            "SKIP: the platform filesystem refuses non-UTF-8 names ({error}); \
             the exact-bytes read is still covered by the unit tests and the \
             trailing-space sibling test"
        );
        return;
    }
    let sibling = temp.path().join("repo\u{FFFD}");
    fs::create_dir(&sibling).expect("sibling folder");

    for repo in [&raw_dir, &sibling] {
        fs::write(repo.join("README.md"), "init\n").expect("seed file");
        checked(repo, &config, &["init", "-b", "main"]);
        checked(repo, &config, &["config", "user.name", "Longhorn Test"]);
        checked(
            repo,
            &config,
            &["config", "user.email", "test@example.invalid"],
        );
        checked(repo, &config, &["add", "."]);
        checked(repo, &config, &["commit", "-m", "Initial"]);
    }

    let policy = support::Policy::isolated(&config);
    let git = handle(&policy);
    let id_raw = git.worktree_identity(&raw_dir).expect("raw identity");
    let id_sibling = git.worktree_identity(&sibling).expect("sibling identity");

    let root_bytes = id_raw.root().as_os_str().as_bytes();
    assert!(
        root_bytes.ends_with(&raw_name),
        "raw root keeps its exact bytes: {root_bytes:?}"
    );
    assert_eq!(
        id_raw.root().as_os_str().to_os_string().into_vec(),
        raw_dir
            .canonicalize()
            .expect("canonical raw root")
            .into_os_string()
            .into_vec(),
    );
    assert_ne!(id_raw.root(), id_sibling.root());
    assert_ne!(id_raw.common_dir(), id_sibling.common_dir());
    assert!(!id_raw.same_repository(&id_sibling));
    assert!(!id_raw.same_worktree(&id_sibling));
}

#[cfg(unix)]
#[test]
fn symlinked_path_and_subdirectory_resolve_to_the_same_worktree_identity() {
    let fixture = Fixture::worktree();
    let symlink = fixture.temp.path().join("worktree-symlink");
    std::os::unix::fs::symlink(&fixture.root, &symlink).expect("symlink to worktree");
    let subdir = fixture.root.join("content/notes");
    let policy = fixture.policy();
    let git = handle(&policy);

    let real_id = git
        .worktree_identity(&fixture.root)
        .expect("real worktree identity");
    let symlink_id = git
        .worktree_identity(&symlink)
        .expect("symlink worktree identity");
    let subdir_id = git
        .worktree_identity(&subdir)
        .expect("subdirectory worktree identity");

    assert_eq!(real_id, symlink_id);
    assert_eq!(real_id, subdir_id);
    assert!(real_id.same_repository(&symlink_id));
    assert!(real_id.same_worktree(&symlink_id));
    assert!(real_id.same_worktree(&subdir_id));
    assert_eq!(symlink_id.root(), real_id.root());
    assert_eq!(subdir_id.root(), real_id.root());
    assert!(real_id.root_filesystem_id().is_some());
    assert!(real_id.common_dir_filesystem_id().is_some());
}

#[test]
fn detached_head_reports_no_branch_and_full_head_oid() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let attached_id = git
        .worktree_identity(&fixture.root)
        .expect("attached identity");
    let expected_oid = checked(&fixture.root, &fixture.config, &["rev-parse", "HEAD"]);
    let short_oid = checked(
        &fixture.root,
        &fixture.config,
        &["rev-parse", "--short", "HEAD"],
    );
    assert_eq!(attached_id.head_oid(), expected_oid);
    assert_ne!(attached_id.head_oid(), short_oid);
    assert!(attached_id.head_oid().len() >= 40);

    checked(&fixture.root, &fixture.config, &["switch", "--detach"]);
    let detached_id = git
        .worktree_identity(&fixture.root)
        .expect("detached identity");

    assert_eq!(detached_id.branch(), None);
    assert!(detached_id.is_detached());
    assert_eq!(detached_id.head_oid(), expected_oid);
    assert!(attached_id.same_repository(&detached_id));
    assert!(attached_id.same_worktree(&detached_id));
    assert_ne!(attached_id, detached_id);

    // Advancing HEAD changes the OID while keeping repository and worktree identity.
    commit(
        &fixture.root,
        &fixture.config,
        "content.md",
        "detached edit\n",
        "Detached commit",
    );
    let next_oid = checked(&fixture.root, &fixture.config, &["rev-parse", "HEAD"]);
    let moved_id = git
        .worktree_identity(&fixture.root)
        .expect("moved detached identity");
    assert_eq!(moved_id.head_oid(), next_oid);
    assert_ne!(moved_id.head_oid(), expected_oid);
    assert!(detached_id.same_worktree(&moved_id));
    assert_ne!(detached_id, moved_id);
}

#[test]
fn non_worktree_folders_return_repository_unavailable_with_redacted_detail() {
    let fixture = Fixture::worktree();
    let plain = fixture.temp.path().join("ghp_plainFolderSecret789");
    fs::create_dir(&plain).expect("a plain folder");
    let missing = fixture.temp.path().join("ghp_missingFolderSecret999");

    let git_dir_redactor = Redactor::new(
        [
            "plainFolderSecret789".to_owned(),
            "missingFolderSecret999".to_owned(),
        ],
        RedactionRules::common(),
    );
    let policy = fixture.policy();
    let git = Git::new(&policy, git_dir_redactor);

    let plain_err = git
        .worktree_identity(&plain)
        .expect_err("plain folder is not a worktree");
    assert_eq!(plain_err.code(), GitFailureCode::RepositoryUnavailable);
    assert!(!plain_err.detail().is_empty());
    assert!(!plain_err.detail().contains("ghp_plainFolderSecret789"));

    let missing_err = git
        .worktree_identity(&missing)
        .expect_err("missing folder cannot be resolved");
    assert_eq!(missing_err.code(), GitFailureCode::RepositoryUnavailable);
    assert!(!missing_err.detail().is_empty());
    assert!(!missing_err.detail().contains("ghp_missingFolderSecret999"));

    let dot_git_err = git
        .worktree_identity(&fixture.root.join(".git"))
        .expect_err(".git directory is not a worktree");
    assert_eq!(dot_git_err.code(), GitFailureCode::RepositoryUnavailable);
    assert!(matches!(dot_git_err, GitFailure::NotWorktree { .. }));

    // Unborn repository (no commits yet) fails with BranchUnavailable.
    let unborn = fixture.temp.path().join("unborn-repo");
    fs::create_dir(&unborn).expect("unborn dir");
    checked(&unborn, &fixture.config, &["init", "-b", "main"]);
    let unborn_err = git
        .worktree_identity(&unborn)
        .expect_err("unborn HEAD has no object ID");
    assert_eq!(unborn_err.code(), GitFailureCode::BranchUnavailable);
}

#[test]
fn debug_formatting_leaks_no_paths_branch_or_oid() {
    let temp = tempfile::tempdir().expect("a test temp root");
    let config = temp.path().join("gitconfig");
    fs::write(&config, "# empty fixture config\n").expect("an empty config");
    let repo = temp
        .path()
        .join("ghp_secretSegment123/sensitive-checkout-dir");
    fs::create_dir_all(&repo).expect("repo dir");
    fs::write(repo.join("file.txt"), "hello\n").expect("seed file");
    checked(&repo, &config, &["init", "-b", "feature/sensitive-branch"]);
    checked(&repo, &config, &["config", "user.name", "Longhorn Test"]);
    checked(
        &repo,
        &config,
        &["config", "user.email", "test@example.invalid"],
    );
    checked(&repo, &config, &["add", "."]);
    checked(&repo, &config, &["commit", "-m", "Initial"]);

    let policy = support::Policy::isolated(&config);
    let git = Git::new(&policy, redactor());
    let identity = git.worktree_identity(&repo).expect("worktree identity");

    let debug = format!("{identity:?}");
    assert!(debug.contains("WorktreeIdentity"), "{debug}");
    assert!(
        !debug.contains(identity.root().to_string_lossy().as_ref()),
        "{debug}"
    );
    assert!(
        !debug.contains(identity.common_dir().to_string_lossy().as_ref()),
        "{debug}"
    );
    assert!(!debug.contains("ghp_secretSegment123"), "{debug}");
    assert!(!debug.contains("sensitive-checkout-dir"), "{debug}");
    assert!(!debug.contains("feature/sensitive-branch"), "{debug}");
    assert!(!debug.contains(identity.head_oid()), "{debug}");
    assert!(
        !debug.contains(&Path::new(".git").to_string_lossy().to_string()),
        "{debug}"
    );
}
