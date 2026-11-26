//! Execution rules through the public API, over real Git: refusal detail,
//! redaction, capture bounds, the wall clock, and the credential hook.

mod support;

use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

use longhorn_git_workspace::{
    FAILURE_DETAIL_BOUND, Git, GitFailure, GitFailureCode, MAX_CAPTURED_OUTPUT_BYTES,
    is_remote_credential_operation, redact_git_detail,
};
use longhorn_redact::{RedactionRules, Redactor};
use support::{Fixture, ext_remote_env, handle, redactor, rewrite_env};

const GITHUB_REMOTE: &str = "https://github.com/example/content";

#[test]
fn a_refusal_carries_gits_words_redacted_with_the_callers_redactor() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = Git::new(
        &policy,
        Redactor::new(["controlled-secret".to_owned()], RedactionRules::common()),
    );

    let failure = git
        .run(
            &fixture.root,
            &["show", "controlled-secret"],
            GitFailureCode::StatusFailed,
            Duration::from_secs(30),
        )
        .expect_err("Git has no such revision");

    let GitFailure::Refused { code, detail } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert_eq!(code, GitFailureCode::StatusFailed);
    assert!(!detail.contains("controlled-secret"), "{detail}");
    assert!(detail.contains("[redacted]"), "{detail}");
    assert!(
        detail.to_ascii_lowercase().contains("unknown revision")
            || detail.to_ascii_lowercase().contains("ambiguous argument"),
        "Git's own words are kept: {detail}"
    );
}

#[test]
fn a_detail_helper_redacts_the_invocation_secret_and_bounds_the_text() {
    let long = format!("{} controlled-secret", "x".repeat(40_000));

    let bounded = redact_git_detail(&redactor(), &long, Some("controlled-secret"));

    assert!(!bounded.contains("controlled-secret"));
    assert!(bounded.len() <= FAILURE_DETAIL_BOUND.bytes);
    assert!(bounded.starts_with("[earlier output omitted]"));
}

#[cfg(unix)]
#[test]
fn a_run_past_its_wall_clock_returns_at_the_bound_even_with_a_sleeping_descendant() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    // The alias starts a two-second descendant that holds the output pipes.
    // The bound is hard only when that descendant is ended with Git.
    let started = Instant::now();
    let failure = git
        .run(
            &fixture.root,
            &[
                "-c",
                "alias.longhorn-slow=!yes x | head -c 100000; sleep 2",
                "longhorn-slow",
            ],
            GitFailureCode::StatusFailed,
            Duration::from_millis(100),
        )
        .expect_err("the alias outlives its bound");
    let elapsed = started.elapsed();

    let GitFailure::TimedOut { code, after } = failure else {
        panic!("expected a timeout, got {failure:?}");
    };
    assert_eq!(code, GitFailureCode::StatusFailed);
    assert_eq!(after, Duration::from_millis(100));
    assert!(
        elapsed < Duration::from_secs(1),
        "the hard clock returned after {elapsed:?}"
    );
}

#[test]
fn successful_standard_output_is_redacted_before_it_is_returned() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["config", "user.name", "controlled-secret"],
    );
    let policy = fixture.policy();
    let git = Git::new(
        &policy,
        Redactor::new(["controlled-secret".to_owned()], RedactionRules::common()),
    );

    let output = git
        .run(
            &fixture.root,
            &["config", "--get", "user.name"],
            GitFailureCode::StatusFailed,
            Duration::from_secs(30),
        )
        .expect("Git prints the configured value");

    assert_eq!(output, "[redacted]");
    assert!(!output.contains("controlled-secret"));
}

#[cfg(unix)]
#[test]
fn a_run_over_the_capture_bound_fails_instead_of_returning_a_partial_answer() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &[
                "-c",
                "alias.longhorn-big=!head -c 200000 /dev/zero | tr '\\0' x",
                "longhorn-big",
            ],
            GitFailureCode::StatusFailed,
            Duration::from_secs(60),
        )
        .expect_err("the alias writes more than the bound");

    assert_eq!(failure.code(), GitFailureCode::OutputTooLarge);
    assert!(
        failure
            .detail()
            .contains(&MAX_CAPTURED_OUTPUT_BYTES.to_string()),
        "{failure}"
    );
}

#[test]
fn a_bare_fetch_to_a_non_origin_upstream_never_requests_the_origin_credential() {
    let fixture = Fixture::worktree();
    let root = &fixture.root;
    let config = &fixture.config;
    support::checked(
        root,
        config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let upstream = fixture.remote.to_str().expect("fixture path");
    support::checked(root, config, &["remote", "add", "upstream", upstream]);
    support::checked(root, config, &["config", "branch.main.remote", "upstream"]);
    let policy = fixture.policy();
    handle(&policy)
        .run(
            root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(15),
        )
        .expect("Git uses the local upstream");

    assert!(policy.credential_calls().is_empty());
    assert!(is_remote_credential_operation(
        &["fetch", "--prune", "origin"],
        GITHUB_REMOTE
    ));
    assert!(!is_remote_credential_operation(
        &["fetch", "--all", "origin"],
        GITHUB_REMOTE
    ));
    assert!(!is_remote_credential_operation(
        &["push", "upstream", "origin"],
        GITHUB_REMOTE
    ));
}

#[test]
fn a_push_remotes_override_and_a_foreign_pushurl_both_exclude_the_origin_credential() {
    let fixture = Fixture::worktree();
    let root = &fixture.root;
    let config = &fixture.config;
    let local = fixture.remote.to_str().expect("fixture path");
    support::checked(
        root,
        config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    support::checked(root, config, &["remote", "add", "publisher", local]);
    support::checked(
        root,
        config,
        &["config", "branch.main.pushRemote", "publisher"],
    );
    support::checked(root, config, &["config", "push.default", "current"]);
    let policy = fixture.policy();
    let git = handle(&policy);

    git.run(
        root,
        &["push"],
        GitFailureCode::PushFailed,
        Duration::from_secs(15),
    )
    .expect("the branch push remote is local");
    support::checked(
        root,
        config,
        &["remote", "set-url", "--push", "origin", local],
    );
    git.push_capability(root).expect("origin pushurl is local");
    assert!(policy.credential_calls().is_empty());
}

#[cfg(unix)]
#[test]
fn a_none_credential_cannot_use_ambient_credentials() {
    let fixture = Fixture::worktree();
    let marker = fixture.temp.path().join("ambient-called");
    let helper = support::script(
        fixture.temp.path(),
        "ambient-helper",
        &format!(
            "touch '{}'
printf '%s\n' username=ambient password=ambient",
            marker.display()
        ),
    );
    let recorder = support::script(
        fixture.temp.path(),
        "fill-credential",
        "printf 'protocol=https\nhost=github.com\n\n' | git credential fill",
    );
    fs::write(
        &fixture.config,
        format!(
            "[credential \"https://github.com\"]
    helper = {}
[core]
    askPass = {}
",
            helper.display(),
            helper.display()
        ),
    )
    .expect("the ambient helper configuration");
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let policy = fixture.policy().with_fetch_isolation(vec![
        (
            format!("url.ext::{}.insteadOf", recorder.display()),
            GITHUB_REMOTE.to_owned(),
        ),
        ("protocol.ext.allow".to_owned(), "always".to_owned()),
    ]);
    let failure = handle(&policy)
        .run(
            &fixture.root,
            &["fetch", "origin"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(15),
        )
        .expect_err("no credential can be supplied");
    assert_eq!(policy.credential_calls(), vec![GITHUB_REMOTE.to_owned()]);
    assert!(
        failure.detail().contains("terminal prompts disabled"),
        "{failure}"
    );
    assert!(!marker.exists());
}

#[test]
fn inherited_git_dir_cannot_replace_the_requested_worktree() {
    const ROOT: &str = "LONGHORN_INHERITED_GIT_DIR_TEST_ROOT";
    const CONFIG: &str = "LONGHORN_INHERITED_GIT_DIR_TEST_CONFIG";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = Path::new(&root);
        let config = std::env::var_os(CONFIG).expect("fixture config");
        let policy = support::Policy::isolated(Path::new(&config));
        let actual = handle(&policy)
            .run(
                root,
                &["rev-parse", "--show-toplevel"],
                GitFailureCode::StatusFailed,
                Duration::from_secs(15),
            )
            .expect("Git found the requested worktree");
        assert_eq!(
            Path::new(&actual).canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
        return;
    }

    let fixture = Fixture::worktree();
    let foreign = fixture.peer();
    let output = Command::new(std::env::current_exe().expect("the current test binary"))
        .args([
            "--exact",
            "inherited_git_dir_cannot_replace_the_requested_worktree",
        ])
        .env(ROOT, &fixture.root)
        .env(CONFIG, &fixture.config)
        .env("GIT_DIR", foreign.join(".git"))
        .env("GIT_WORK_TREE", &foreign)
        .env("GIT_CONFIG_PARAMETERS", "'invalid injection'")
        .output()
        .expect("the subprocess test runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(unix)]
#[test]
fn a_local_permission_failure_does_not_refresh_the_credential() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let refusal = support::script(
        fixture.temp.path(),
        "refuse-local",
        "echo 'fatal: cannot open .git/FETCH_HEAD: Permission denied' >&2
exit 1",
    );
    let policy = fixture
        .policy()
        .with_credential(ext_remote_env(&refusal, GITHUB_REMOTE), "access-token");
    let failure = handle(&policy)
        .run(
            &fixture.root,
            &["fetch", "origin"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(15),
        )
        .expect_err("the local file is unreadable");
    assert!(failure.detail().contains("Permission denied"), "{failure}");
    assert_eq!(policy.refresh_calls(), 0);
}

#[test]
fn a_credential_scoped_fetch_receives_the_supplied_environment() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let policy = fixture.policy().with_credential(
        rewrite_env(&fixture.remote, GITHUB_REMOTE),
        "controlled-secret",
    );
    let git = handle(&policy);

    git.run(
        &fixture.root,
        &["fetch"],
        GitFailureCode::FetchFailed,
        Duration::from_secs(60),
    )
    .expect("the supplied credential environment rewrites the remote locally");

    assert_eq!(policy.credential_calls(), vec![GITHUB_REMOTE.to_owned()]);
}

#[test]
fn a_reading_of_a_github_worktree_never_asks_for_a_credential() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let policy = fixture.policy();
    let git = handle(&policy);

    git.inspect_worktree(&fixture.root)
        .expect("a status reading");

    assert!(policy.credential_calls().is_empty());
}

#[test]
fn a_credential_failure_stops_a_remote_read_before_git_runs() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let policy = fixture.policy().with_credential_error(GitFailure::Refused {
        code: GitFailureCode::FetchFailed,
        detail: "the application holds no credential".to_owned(),
    });
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(30),
        )
        .expect_err("a credential error stops the operation");

    assert_eq!(failure.code(), GitFailureCode::FetchFailed);
    assert_eq!(failure.detail(), "the application holds no credential");
    assert_eq!(policy.credential_calls(), vec![GITHUB_REMOTE.to_owned()]);
}

#[cfg(unix)]
#[test]
fn a_run_whose_git_exits_before_its_descendants_is_still_timed_out() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    // Git itself exits successfully at once, but the backgrounded descendant
    // inherits the output pipes. The call has not finished until they close.
    let started = Instant::now();
    let failure = git
        .run(
            &fixture.root,
            &["-c", "alias.longhorn-orphan=!sleep 2 &", "longhorn-orphan"],
            GitFailureCode::StatusFailed,
            Duration::from_millis(100),
        )
        .expect_err("output was still held at the bound");
    let elapsed = started.elapsed();

    assert!(
        matches!(failure, GitFailure::TimedOut { .. }),
        "expected a timeout, got {failure:?}"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "the hard clock returned after {elapsed:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_rejected_credential_is_refreshed_once_and_the_retry_succeeds() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let refusal = support::script(
        fixture.temp.path(),
        "refuse.sh",
        "echo 'fatal: Authentication failed for the remote' >&2\nexit 1",
    );
    let policy = fixture
        .policy()
        .with_credential(ext_remote_env(&refusal, GITHUB_REMOTE), "rejected-secret")
        .with_refresh(
            rewrite_env(&fixture.remote, GITHUB_REMOTE),
            "refreshed-secret",
        );
    let git = handle(&policy);

    git.run(
        &fixture.root,
        &["fetch"],
        GitFailureCode::FetchFailed,
        Duration::from_secs(60),
    )
    .expect("the refreshed credential reaches the remote");

    assert_eq!(policy.refresh_calls(), 1);
}

#[cfg(unix)]
#[test]
fn a_credential_refresh_never_restarts_the_wall_clock() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    // Each attempt is refused for authentication after one second, so a
    // retry that restarted the clock would return a refusal after two.
    let refusal = support::script(
        fixture.temp.path(),
        "slow-refuse.sh",
        "sleep 1\necho 'fatal: Authentication failed for the remote' >&2\nexit 1",
    );
    let env = ext_remote_env(&refusal, GITHUB_REMOTE);
    let policy = fixture
        .policy()
        .with_credential(env.clone(), "rejected-secret")
        .with_refresh(env, "refreshed-secret");
    let git = handle(&policy);

    let started = Instant::now();
    let failure = git
        .run(
            &fixture.root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            Duration::from_millis(1500),
        )
        .expect_err("the retry ran out of the call's bound");
    let elapsed = started.elapsed();

    assert!(
        matches!(failure, GitFailure::TimedOut { .. }),
        "expected a timeout, got {failure:?}"
    );
    assert_eq!(policy.refresh_calls(), 1);
    assert!(
        elapsed < Duration::from_millis(1900),
        "the refreshed retry returned after {elapsed:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_declined_refresh_that_answers_late_is_a_timeout() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let refusal = support::script(
        fixture.temp.path(),
        "refuse.sh",
        "echo 'fatal: Authentication failed for the remote' >&2\nexit 1",
    );
    // The policy declines the refresh, but only after the call's bound.
    let policy = fixture
        .policy()
        .with_credential(ext_remote_env(&refusal, GITHUB_REMOTE), "rejected-secret")
        .with_refresh_delay(Duration::from_millis(1200));
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            Duration::from_millis(800),
        )
        .expect_err("the refresh answered after the bound");

    assert!(
        matches!(failure, GitFailure::TimedOut { .. }),
        "expected a timeout, got {failure:?}"
    );
    assert_eq!(policy.refresh_calls(), 1);
}

#[test]
fn a_credential_error_that_answers_late_is_a_timeout() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    // The lookup fails, but only after the call's bound.
    let policy = fixture
        .policy()
        .with_credential_error(GitFailure::Unavailable {
            code: GitFailureCode::CommandUnavailable,
            detail: "the credential store is locked".to_owned(),
        })
        .with_credential_delay(Duration::from_millis(1200));
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &["fetch"],
            GitFailureCode::FetchFailed,
            Duration::from_millis(800),
        )
        .expect_err("the lookup answered after the bound");

    assert!(
        matches!(failure, GitFailure::TimedOut { .. }),
        "expected a timeout, got {failure:?}"
    );
    assert_eq!(policy.credential_calls(), vec![GITHUB_REMOTE.to_owned()]);
}

#[cfg(unix)]
#[test]
fn a_long_stderr_never_returns_private_key_body_text() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    // Filler first, then a private-key block longer than any tail window, so a
    // tail cut would drop the BEGIN line and leave the body behind.
    let failure = git
        .run(
            &fixture.root,
            &[
                "-c",
                "alias.longhorn-key=!head -c 100000 /dev/zero | tr '\\0' x >&2; printf -- '-----BEGIN PRIVATE KEY-----\\n' >&2; yes SENSITIVE_PRIVATE_KEY_BODY | head -n 20000 >&2; printf -- '-----END PRIVATE KEY-----\\n' >&2; exit 1",
                "longhorn-key",
            ],
            GitFailureCode::StatusFailed,
            Duration::from_secs(60),
        )
        .expect_err("the alias exits non-zero");

    let GitFailure::Refused { detail, .. } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(!detail.contains("SENSITIVE_PRIVATE_KEY_BODY"), "{detail}");
    assert!(detail.contains("[redacted]"), "{detail}");
}

#[cfg(unix)]
#[test]
fn stderr_past_the_capture_bound_is_withheld_with_a_note() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &[
                "-c",
                "alias.longhorn-flood=!head -c 1500000 /dev/zero | tr '\\0' x >&2; exit 1",
                "longhorn-flood",
            ],
            GitFailureCode::StatusFailed,
            Duration::from_secs(60),
        )
        .expect_err("the alias exits non-zero");

    let GitFailure::Refused { detail, .. } = failure else {
        panic!("expected a refusal, got {failure:?}");
    };
    assert!(detail.contains("later output was not retained"), "{detail}");
}

#[test]
fn a_folder_failure_detail_goes_through_the_callers_redactor() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let missing = fixture.temp.path().join("missing-worktree");
    // The operating system's own words for the missing folder: the caller's
    // redactor must reach them like any other failure detail.
    let message = fs::canonicalize(&missing)
        .expect_err("the folder does not exist")
        .to_string();
    let git = Git::new(
        &policy,
        Redactor::new([message.clone()], RedactionRules::exact_only()),
    );

    let failure = git
        .inspect_worktree(&missing)
        .expect_err("the folder is missing");

    let GitFailure::FolderUnavailable { ref detail, .. } = failure else {
        panic!("expected an unavailable folder, got {failure:?}");
    };
    assert!(!detail.contains(&message), "{detail}");
    assert!(detail.contains("[redacted]"), "{detail}");
}
