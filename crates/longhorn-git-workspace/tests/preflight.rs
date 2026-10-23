//! Remote-refusal classification and preflight, through the public API, with
//! no network: the only remote ever contacted is a local path.

mod support;

use std::fs;

use longhorn_git_workspace::{
    GitCredential, RemoteAccessFailure, classify_remote_failure, credential_bridge_configured,
    is_https_github,
};
use support::{Fixture, Policy, handle, rewrite_env};

const GITHUB_REMOTE: &str = "https://github.com/example/content";

#[test]
fn github_over_https_is_the_one_credential_scoped_route() {
    assert!(is_https_github(GITHUB_REMOTE));
    assert!(is_https_github("HTTPS://GitHub.com/example/content"));
    assert!(is_https_github("http://github.com/example/content"));
    assert!(!is_https_github("git@github.com:example/content.git"));
    assert!(!is_https_github("ssh://git@github.com/example/content.git"));
    assert!(!is_https_github("https://gitlab.com/example/content"));
    assert!(!is_https_github("/tmp/remote.git"));
}

#[test]
fn an_https_github_remote_without_a_credential_is_refused_before_git_runs() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = git
        .preflight_remote(GITHUB_REMOTE, None)
        .expect_err("no credential was supplied");

    assert_eq!(failure, RemoteAccessFailure::CredentialHelperMissing);
    assert!(policy.credential_calls().is_empty());
    assert!(!credential_bridge_configured(None));
}

#[test]
fn an_ambient_url_scoped_helper_is_invisible_to_the_preflight() {
    // A host-configured `gh`-style helper is not a credential this mechanism
    // may use. The remote still demands the application's own sign-in.
    let fixture = Fixture::worktree();
    let config = fixture.temp.path().join("ambient-gitconfig");
    fs::write(
        &config,
        "[credential \"https://github.com\"]\n\thelper = /usr/local/bin/gh auth git-credential\n",
    )
    .expect("an ambient config fixture");
    let policy = Policy::isolated(&config);
    let git = handle(&policy);

    let failure = git
        .preflight_remote(GITHUB_REMOTE, None)
        .expect_err("an ambient helper is not the application credential");

    assert_eq!(failure, RemoteAccessFailure::CredentialHelperMissing);
}

#[test]
fn each_remote_refusal_is_a_separate_reading() {
    assert_eq!(
        classify_remote_failure(
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
        ),
        RemoteAccessFailure::CredentialHelperMissing
    );
    assert_eq!(
        classify_remote_failure(
            "remote: Support for password authentication was removed.\nfatal: Authentication failed for 'https://github.com/example/content.git/'"
        ),
        RemoteAccessFailure::AuthenticationRequired
    );
    assert_eq!(
        classify_remote_failure(
            "ssh: Could not resolve hostname github.com: nodename nor servname provided"
        ),
        RemoteAccessFailure::NetworkUnavailable
    );
    assert_eq!(
        classify_remote_failure(
            "remote: Repository not found.\nfatal: repository 'https://github.com/example/missing.git/' not found"
        ),
        RemoteAccessFailure::RepositoryUnavailable
    );
    assert_eq!(
        classify_remote_failure("fatal: the remote end hung up unexpectedly"),
        RemoteAccessFailure::Unavailable
    );
}

#[test]
fn an_absent_local_remote_reads_as_an_unavailable_repository() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);
    let absent = fixture.temp.path().join("absent-remote.git");

    let failure = git
        .preflight_remote(absent.to_str().expect("a fixture path"), None)
        .expect_err("the remote does not exist");

    assert_eq!(failure, RemoteAccessFailure::RepositoryUnavailable);
}

#[test]
fn a_local_bare_remote_passes_the_preflight_with_no_credential() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    git.preflight_remote(fixture.remote.to_str().expect("a fixture path"), None)
        .expect("a local bare remote is readable");
    assert!(policy.credential_calls().is_empty());
}

#[test]
fn a_supplied_credential_reaches_the_preflight() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy().with_credential(
        rewrite_env(&fixture.remote, GITHUB_REMOTE),
        "controlled-secret",
    );
    let git = handle(&policy);
    let credential = GitCredential::new(
        rewrite_env(&fixture.remote, GITHUB_REMOTE),
        "controlled-secret",
    );
    assert!(credential_bridge_configured(Some(&credential)));

    git.preflight_remote(GITHUB_REMOTE, Some(&credential))
        .expect("the credential rewrites the remote to a local repository");
}

#[test]
fn the_write_capability_probe_classifies_a_present_and_an_absent_remote() {
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    git.push_capability(&fixture.root)
        .expect("the local bare remote accepts a dry-run push");

    let absent = fixture.temp.path().join("absent-remote.git");
    support::checked(
        &fixture.root,
        &fixture.config,
        &[
            "remote",
            "set-url",
            "origin",
            absent.to_str().expect("a path"),
        ],
    );
    let failure = git
        .push_capability(&fixture.root)
        .expect_err("the remote does not exist");
    assert_eq!(failure, RemoteAccessFailure::RepositoryUnavailable);
}

#[test]
fn an_unreachable_loopback_remote_reads_as_a_network_refusal() {
    // Loopback only: nothing leaves the host, and no listener exists on port 1.
    let fixture = Fixture::worktree();
    let policy = fixture.policy();
    let git = handle(&policy);

    let failure = git
        .preflight_remote("https://127.0.0.1:1/example/content.git", None)
        .expect_err("no server listens there");

    assert_eq!(failure, RemoteAccessFailure::NetworkUnavailable);
}
