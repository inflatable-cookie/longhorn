//! Execution rules through the public API, over real Git: refusal detail,
//! redaction, capture bounds, the wall clock, and the credential hook.

mod support;

use std::time::{Duration, Instant};

use longhorn_git_workspace::{
    FAILURE_DETAIL_BOUND, Git, GitFailure, GitFailureCode, MAX_CAPTURED_OUTPUT_BYTES,
    is_remote_credential_operation, redact_git_detail,
};
use longhorn_redact::{RedactionRules, Redactor};
use support::{Fixture, handle, redactor, rewrite_env};

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
fn only_a_remote_github_operation_asks_the_policy_for_a_credential() {
    assert!(is_remote_credential_operation(&["fetch"], GITHUB_REMOTE));
    assert!(is_remote_credential_operation(
        &["fetch", "--prune", "origin"],
        GITHUB_REMOTE
    ));
    assert!(is_remote_credential_operation(
        &["push", "--", "origin", "HEAD:refs/heads/main"],
        GITHUB_REMOTE
    ));
    assert!(is_remote_credential_operation(
        &["ls-remote", "--heads", "origin"],
        GITHUB_REMOTE
    ));
    assert!(!is_remote_credential_operation(&["status"], GITHUB_REMOTE));
    assert!(!is_remote_credential_operation(
        &["fetch", "--all"],
        GITHUB_REMOTE
    ));
    assert!(!is_remote_credential_operation(
        &["fetch"],
        "https://gitlab.com/example/content"
    ));
    assert!(!is_remote_credential_operation(
        &["fetch"],
        "git@github.com:example/content.git"
    ));
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
