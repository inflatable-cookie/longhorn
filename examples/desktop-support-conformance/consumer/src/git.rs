//! Contract 029 (`longhorn-git-workspace`): the workspace mechanism over real
//! local Git, and GitHub sign-in over the crate's loopback fixture server.
//!
//! Every fixture here is constructed from values this consumer owns: a
//! temporary bare remote, a temporary worktree, a remote URL, a branch name, an
//! expected remote, an OAuth client id, a token variable name and a
//! credential slot. No product name, path or policy appears.
//!
//! Nothing reaches the network. The clone, the status and branch reads, the
//! preflight, the sync, the branch switch and the attach comparison run real
//! `git` against temporary local repositories. The device flow, the token
//! exchange and the askpass fetch run against the crate's loopback
//! `fixture-server`, with a local bare repository behind the GitHub remote the
//! mechanism resolves.

use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};

use longhorn_core::{CredentialError, CredentialSlot, CredentialStore};
use longhorn_git_workspace::{
    AskpassConfig, AttachReading, CloneFailure, CredentialScope, CredentialStoreReading,
    DeviceFlowEndpoints, FixtureRoute, FixtureServer, GITHUB_DEVICE_VERIFICATION_URL, Git,
    GitCredential, GitCredentialBridge, GitFailure, GitFailureCode, GitPolicy, GithubAuthProgress,
    GithubTokenClient, RemoteAccessFailure, SwitchReading, SyncReading, SyncStop,
    classify_remote_failure, create_owned_staging, fresh_access_token, has_switch_blocking_dirt,
    read_credential_presence, remote_for_display, remote_identity, remotes_match, remove_owned_staging,
    rename_exclusive, run_device_flow, staging_parent, target_already_present, write_grant,
};
use longhorn_redact::{RedactionRules, Redactor};
use secrecy::{ExposeSecret as _, SecretString};

/// The GitHub remote the sign-in half resolves. Nothing dials it: the
/// credential's Git environment points it at a local bare repository.
const REMOTE: &str = "https://github.com/example-owner/example-repository";

/// The consumer's OAuth registration and the token variable its askpass
/// configuration names.
const CLIENT_ID: &str = "conformance-client-id";
const TOKEN_ENV: &str = "CONFORMANCE_GIT_TOKEN";
const USER_CODE: &str = "CONF-0001";

/// The two values the loopback token endpoint issues. The token is the value
/// every assertion here searches for.
const TOKEN: &str = "conformance-access-token-9d2b";
const REFRESH_TOKEN: &str = "conformance-refresh-token-4c71";

/// The loopback routes the device flow and the token exchange read.
const DEVICE_CODE_PATH: &str = "/login/device/code";
const ACCESS_TOKEN_PATH: &str = "/login/oauth/access_token";

/// The wall clock one Git invocation the consumer owns is allowed.
const TIMEOUT: Duration = Duration::from_secs(60);

/// The caller's redactor for every reading: no seeded secrets, the common
/// rules.
fn redactor() -> Redactor {
    Redactor::new(Vec::<String>::new(), RedactionRules::common())
}

/// The caller's process isolation for the local half: an empty global Git
/// configuration and no system one, so the machine's Git configuration cannot
/// change a reading. No credential is supplied.
struct LocalPolicy {
    config: PathBuf,
}

impl GitPolicy for LocalPolicy {
    fn isolate(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }

    fn credential(&self, _remote: &str) -> Result<Option<GitCredential>, GitFailure> {
        Ok(None)
    }

    fn refresh_credential(&self, _rejected: &GitCredential) -> Option<GitCredential> {
        None
    }
}

/// The caller's policy for the signed-in half: the same isolation, and a
/// credential built from the stored grant's token through the crate's askpass
/// bridge.
///
/// The bridge is kept for the policy's lifetime, so its private folder exists
/// exactly while an invocation can use it.
struct BridgePolicy {
    config: PathBuf,
    askpass: AskpassConfig,
    token: SecretString,
    rules: Vec<(String, String)>,
    asked: Mutex<Vec<String>>,
    kept: Mutex<Vec<GitCredentialBridge>>,
}

impl BridgePolicy {
    fn new(config: &Path, token: SecretString, rules: Vec<(String, String)>) -> Self {
        Self {
            config: config.to_path_buf(),
            askpass: AskpassConfig::new(TOKEN_ENV).expect("a shell-safe token variable name"),
            token,
            rules,
            asked: Mutex::new(Vec::new()),
            kept: Mutex::new(Vec::new()),
        }
    }

    /// Every remote the mechanism asked this policy for a credential about.
    fn asked(&self) -> Vec<String> {
        self.asked
            .lock()
            .expect("the credential log is not poisoned")
            .clone()
    }

    fn build(&self) -> Result<GitCredential, GitFailure> {
        let bridge =
            GitCredentialBridge::from_token(&self.askpass, self.token.clone()).map_err(|_| {
                GitFailure::Unavailable {
                    code: GitFailureCode::FetchFailed,
                    detail: "the askpass bridge could not be written".to_owned(),
                }
            })?;
        let credential = with_rules(&bridge.credential(), &self.rules);
        self.kept
            .lock()
            .expect("the kept bridges are not poisoned")
            .push(bridge);
        Ok(credential)
    }
}

impl GitPolicy for BridgePolicy {
    fn isolate(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }

    fn credential(&self, remote: &str) -> Result<Option<GitCredential>, GitFailure> {
        self.asked
            .lock()
            .expect("the credential log is not poisoned")
            .push(remote.to_owned());
        self.build().map(Some)
    }

    fn refresh_credential(&self, _rejected: &GitCredential) -> Option<GitCredential> {
        None
    }
}

/// A bridge credential's environment with the caller's Git configuration rules
/// added, so a local bare repository stands in for the GitHub remote.
///
/// The credential rides the mechanism's child environment, which is applied
/// after the policy's isolation, so this is the only place the substitute can
/// live.
fn with_rules(credential: &GitCredential, rules: &[(String, String)]) -> GitCredential {
    let mut env: Vec<(String, String)> = credential
        .env_vars()
        .iter()
        .filter(|(key, _)| !key.starts_with("GIT_CONFIG_"))
        .cloned()
        .collect();
    env.push(("GIT_CONFIG_COUNT".to_owned(), (rules.len() + 1).to_string()));
    env.push((
        "GIT_CONFIG_KEY_0".to_owned(),
        "credential.helper".to_owned(),
    ));
    env.push(("GIT_CONFIG_VALUE_0".to_owned(), String::new()));
    for (index, (key, value)) in rules.iter().enumerate() {
        env.push((format!("GIT_CONFIG_KEY_{}", index + 1), key.clone()));
        env.push((format!("GIT_CONFIG_VALUE_{}", index + 1), value.clone()));
    }
    GitCredential::new(env, credential.secret().expose_secret().to_owned())
}

/// The rule that points the GitHub remote at a local bare repository.
fn file_rule(local: &Path) -> Vec<(String, String)> {
    vec![(
        format!("url.file://{}.insteadOf", local.display()),
        REMOTE.to_owned(),
    )]
}

/// The rule that points the GitHub remote at a local recording program through
/// Git's `ext::` transport, so a refusal can be scripted offline and the
/// arguments Git was given can be read back.
fn recording_rule(program: &Path) -> Vec<(String, String)> {
    vec![
        (
            format!("url.ext::{} {REMOTE}.insteadOf", program.display()),
            REMOTE.to_owned(),
        ),
        ("protocol.ext.allow".to_owned(), "always".to_owned()),
    ]
}

/// A credential store held in memory for this consumer's own scope.
///
/// The mechanism never supplies a store: contract 029 pins that the store is
/// injected, so this consumer keeps its own.
#[derive(Default)]
struct InMemoryStore {
    secrets: Mutex<BTreeMap<CredentialSlot, String>>,
}

impl CredentialStore for InMemoryStore {
    fn store(&self, slot: &CredentialSlot, secret: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .expect("the store is not poisoned")
            .insert(slot.clone(), secret.to_owned());
        Ok(())
    }

    fn retrieve(&self, slot: &CredentialSlot) -> Result<Option<String>, CredentialError> {
        Ok(self
            .secrets
            .lock()
            .expect("the store is not poisoned")
            .get(slot)
            .cloned())
    }

    fn remove(&self, slot: &CredentialSlot) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .expect("the store is not poisoned")
            .remove(slot);
        Ok(())
    }
}

/// Real local Git fixtures: one bare remote, one worktree that seeds it, and an
/// empty global configuration every child Git process is isolated to.
struct Fixture {
    temp: tempfile::TempDir,
    config: PathBuf,
    remote: PathBuf,
    seed: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let config = temp.path().join("gitconfig");
        fs::write(&config, "# empty conformance config\n").expect("an empty Git configuration");
        let remote = temp.path().join("remote.git");
        let remote_text = remote.to_string_lossy().into_owned();
        checked(temp.path(), &config, &["init", "--bare", &remote_text]);
        let seed = temp.path().join("seed");
        fs::create_dir_all(&seed).expect("the seed worktree");
        fs::write(seed.join("notes.md"), "base\n").expect("the seeded content");
        checked(&seed, &config, &["init", "-b", "main"]);
        identity(&seed, &config);
        checked(&seed, &config, &["add", "."]);
        checked(&seed, &config, &["commit", "-m", "Base"]);
        checked(&seed, &config, &["remote", "add", "origin", &remote_text]);
        checked(&seed, &config, &["push", "-u", "origin", "main"]);
        // A bare repository keeps its own HEAD; a clone of it must land on the
        // branch the seed published.
        checked(&remote, &config, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        Self {
            temp,
            config,
            remote,
            seed,
        }
    }

    /// The bare remote's path, as a Git remote URL for the caller's values.
    fn remote_url(&self) -> String {
        self.remote.to_string_lossy().into_owned()
    }

    /// A second checkout of the same remote, for a commit this workspace has
    /// not seen.
    fn peer(&self, name: &str) -> PathBuf {
        let peer = self.temp.path().join(name);
        let remote = self.remote_url();
        let peer_text = peer.to_string_lossy().into_owned();
        checked(
            self.temp.path(),
            &self.config,
            &["clone", "--branch", "main", &remote, &peer_text],
        );
        identity(&peer, &self.config);
        peer
    }
}

/// Sets the author and committer identity fixture commits use.
fn identity(root: &Path, config: &Path) {
    checked(root, config, &["config", "user.name", "Conformance Fixture"]);
    checked(
        root,
        config,
        &["config", "user.email", "fixture@example.invalid"],
    );
}

/// Runs one Git command with the fixture's isolation. Fixture construction and
/// commits use this, never the mechanism.
fn run_git(root: &Path, config: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .expect("the conformance host runs git")
}

/// Runs one Git command and returns its trimmed standard output.
fn checked(root: &Path, config: &Path, arguments: &[&str]) -> String {
    let output = run_git(root, config, arguments);
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Writes one executable shell program inside a fixture directory.
fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the scripted program");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("an executable script");
    path
}

/// Every staging leftover one parent holds.
fn staging_leftovers(parent: &Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("the fixture parent")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.contains(".creating-"))
        })
        .collect()
}

pub(crate) fn run() -> String {
    let fixture = Fixture::new();
    let local = LocalPolicy {
        config: fixture.config.clone(),
    };
    let git = Git::new(&local, redactor());

    // The contract's composition for a new workspace: an absent target, one
    // staging folder this invocation owns, a clone into it, and a same-parent
    // exclusive rename into place.
    let target = fixture.temp.path().join("workspace");
    assert_eq!(staging_parent(&target), Some(fixture.temp.path()));
    assert!(
        !target_already_present(&target),
        "the target starts absent"
    );
    let staging = create_owned_staging(fixture.temp.path(), OsStr::new("workspace"))
        .expect("one owned staging folder");
    let cancel = AtomicBool::new(false);
    git.clone_into(&fixture.remote_url(), &staging, &cancel, None)
        .expect("the local remote clones into the staging folder");
    rename_exclusive(&staging, &target).expect("the exclusive rename places the clone");
    assert!(!staging.exists(), "the staging folder became the target");

    // A worktree reading: the canonical root, the current branch, the upstream,
    // both branch kinds, and one computed cleanliness.
    let state = git
        .inspect_worktree(&target)
        .expect("the clone is a worktree");
    assert_eq!(state.current_branch, "main");
    assert!(!state.detached, "the clone is on a branch");
    assert_eq!(state.upstream.as_deref(), Some("origin/main"));
    assert!(state.status.clean, "a fresh clone is clean");
    let local_main = state
        .branches
        .iter()
        .find(|branch| branch.id == "local:main")
        .expect("the clone observes its local branch");
    assert!(local_main.current, "local:main is current");
    let remote_main = state
        .branches
        .iter()
        .find(|branch| branch.id == "remote:origin/main")
        .expect("the clone observes the remote-tracking branch");
    assert!(!remote_main.current);
    assert_eq!(
        remote_main.local_target.as_deref(),
        Some("main"),
        "a remote reference records the local branch that tracks it"
    );
    assert_eq!(
        git.origin_url(&target).as_deref(),
        Some(fixture.remote_url().as_str())
    );

    // A second clone onto the same target is refused, and the target it would
    // have replaced is unchanged. `target_already_present` reads the existing
    // target first; a clone that still filled its own staging folder is then
    // refused by the same-parent exclusive rename, which never replaces a
    // destination that appeared.
    assert!(target_already_present(&target), "the target exists now");
    assert!(
        staging_leftovers(fixture.temp.path()).is_empty(),
        "the first clone left no staging folder behind"
    );
    let head_before = checked(&target, &fixture.config, &["rev-parse", "HEAD"]);
    let second = create_owned_staging(fixture.temp.path(), OsStr::new("workspace"))
        .expect("one owned staging folder for the second attempt");
    git.clone_into(&fixture.remote_url(), &second, &cancel, None)
        .expect("the second clone fills its own staging folder");
    let refused = rename_exclusive(&second, &target)
        .expect_err("the exclusive rename never replaces an existing target");
    assert_eq!(refused.kind(), std::io::ErrorKind::AlreadyExists);
    remove_owned_staging(&second);
    assert!(!second.exists(), "the refused staging folder is removed");
    assert_eq!(
        checked(&target, &fixture.config, &["rev-parse", "HEAD"]),
        head_before,
        "the existing workspace is unchanged"
    );

    // A failed clone leaves no partial target: the caller removes only its own
    // staging folder, and the target never appeared.
    let absent_remote = fixture.temp.path().join("absent.git");
    let unreachable = fixture.temp.path().join("unreachable");
    let abandoned = create_owned_staging(fixture.temp.path(), OsStr::new("unreachable"))
        .expect("one owned staging folder for the refused clone");
    let failure = git
        .clone_into(
            &absent_remote.to_string_lossy(),
            &abandoned,
            &cancel,
            None,
        )
        .expect_err("a remote that does not exist refuses the clone");
    assert!(
        matches!(failure, CloneFailure::Refused { .. }),
        "the missing remote is a refusal: {failure:?}"
    );
    remove_owned_staging(&abandoned);
    assert!(!unreachable.exists(), "a refused clone leaves no target");
    assert!(staging_leftovers(fixture.temp.path()).is_empty());

    // A preflight refusal is classified, and read-only: a GitHub remote without
    // the caller's credential is refused before Git runs, and a missing
    // repository is read from Git's own words.
    assert_eq!(
        git.preflight_remote(
            "https://github.com/example-owner/absent-repository",
            None
        ),
        Err(RemoteAccessFailure::CredentialHelperMissing)
    );
    assert_eq!(
        git.preflight_remote(&absent_remote.to_string_lossy(), None),
        Err(RemoteAccessFailure::RepositoryUnavailable)
    );

    // The remote helpers are the caller's comparison and display values: an
    // SSH and an HTTPS spelling of one repository compare equal, and a remote
    // carrying user information is displayable.
    assert!(remotes_match(
        "git@github.com:example-owner/example-repository.git",
        REMOTE
    ));
    assert_eq!(
        remote_identity(REMOTE),
        "github.com/example-owner/example-repository"
    );
    assert_eq!(
        remote_for_display("https://user:credential@example.invalid/repository"),
        "https://example.invalid/repository"
    );

    // Legacy Sync: a commit this workspace has not seen is fetched and
    // fast-forwarded, then a local commit is pushed. The seed worktree
    // publishes the first one; the sign-in half uses a fresh checkout, because
    // the local commit below moves the remote past the seed.
    fs::write(fixture.seed.join("notes.md"), "base\nfrom the peer\n")
        .expect("the peer's change");
    checked(&fixture.seed, &fixture.config, &["commit", "-am", "Peer"]);
    checked(&fixture.seed, &fixture.config, &["push", "origin", "main"]);
    let peer_head = checked(&fixture.seed, &fixture.config, &["rev-parse", "HEAD"]);
    let before = git
        .inspect_worktree(&target)
        .expect("the worktree before the sync");
    let reading = git
        .sync_workspace(before, |root| git.inspect_worktree(Path::new(root)))
        .expect("the local remote answers the sync");
    let SyncReading::Completed {
        fetched,
        fast_forwarded,
        pushed,
        ..
    } = reading
    else {
        panic!("a clean, upstream-tracking branch completes the sync: {reading:?}");
    };
    assert!(fetched, "the sync fetched");
    assert!(fast_forwarded, "a behind branch fast-forwarded");
    assert!(!pushed, "an up-to-date branch was not pushed");
    assert_eq!(
        checked(&target, &fixture.config, &["rev-parse", "HEAD"]),
        peer_head,
        "the fast-forward brought the peer's commit"
    );

    fs::write(target.join("notes.md"), "base\nfrom the peer\nlocal\n").expect("the local change");
    checked(&target, &fixture.config, &["commit", "-am", "Local"]);
    let local_head = checked(&target, &fixture.config, &["rev-parse", "HEAD"]);
    let before = git
        .inspect_worktree(&target)
        .expect("the worktree before the push");
    let reading = git
        .sync_workspace(before, |root| git.inspect_worktree(Path::new(root)))
        .expect("the local remote answers the push");
    let SyncReading::Completed {
        fetched,
        fast_forwarded,
        pushed,
        ..
    } = reading
    else {
        panic!("an ahead branch completes the sync: {reading:?}");
    };
    assert!(fetched);
    assert!(!fast_forwarded);
    assert!(pushed, "an ahead branch was pushed");
    assert_eq!(
        checked(&fixture.remote, &fixture.config, &["rev-parse", "main"]),
        local_head,
        "the push reached the remote"
    );

    // The explicit branch switch: a branch observed in the prior reading, a
    // branch that is already current, an id the reading never observed, an
    // untracked-only worktree, and tracked work that blocks it.
    checked(&target, &fixture.config, &["branch", "topic"]);
    let before = git
        .inspect_worktree(&target)
        .expect("the worktree before the switch");
    assert!(
        before.has_named_ref("topic"),
        "the switch reads only observed branches"
    );
    let reading = git
        .switch_branch(before, "local:topic", |root| {
            git.inspect_worktree(Path::new(root))
        })
        .expect("an observed branch switches");
    let SwitchReading::Switched { state } = reading else {
        panic!("a local branch switches: {reading:?}");
    };
    assert_eq!(state.current_branch, "topic");
    let reading = git
        .switch_branch(state, "local:topic", |root| {
            git.inspect_worktree(Path::new(root))
        })
        .expect("the current branch is already current");
    assert!(
        matches!(reading, SwitchReading::AlreadyCurrent { .. }),
        "an already-current branch runs nothing: {reading:?}"
    );
    let missing = git
        .switch_branch(
            git.inspect_worktree(&target).expect("the worktree"),
            "local:absent",
            |root| git.inspect_worktree(Path::new(root)),
        )
        .expect_err("an unobserved branch id switches nothing");
    assert_eq!(missing.code(), GitFailureCode::BranchNotFound);

    // Untracked content is in-progress work, not dirt: it counts in the status
    // and does not block a switch.
    fs::write(target.join("in-progress.txt"), "unsaved\n").expect("untracked content");
    let untracked = git.status(&target).expect("the status of the worktree");
    assert_eq!(untracked.untracked, 1);
    assert!(!untracked.clean, "untracked content is not clean");
    assert!(
        !has_switch_blocking_dirt(
            &git.inspect_worktree(&target)
                .expect("the worktree with untracked content")
        ),
        "untracked content does not block a switch"
    );
    let reading = git
        .switch_branch(
            git.inspect_worktree(&target).expect("the worktree"),
            "remote:origin/main",
            |root| git.inspect_worktree(Path::new(root)),
        )
        .expect("a remote branch with a tracking local branch switches to it");
    let SwitchReading::Switched { state } = reading else {
        panic!("a tracked remote branch switches: {reading:?}");
    };
    assert_eq!(
        state.current_branch, "main",
        "the remote branch switched through its local target"
    );

    // Tracked work blocks a switch and stops a sync at the dirty step.
    fs::write(target.join("notes.md"), "base\nfrom the peer\nlocal\nedited\n")
        .expect("tracked work");
    let dirty = git.status(&target).expect("the status of the dirty worktree");
    assert_eq!(dirty.modified, 1);
    assert!(has_switch_blocking_dirt(
        &git.inspect_worktree(&target).expect("the dirty worktree")
    ));
    let reading = git
        .switch_branch(
            git.inspect_worktree(&target).expect("the dirty worktree"),
            "local:topic",
            |root| git.inspect_worktree(Path::new(root)),
        )
        .expect("a dirty worktree is read, not switched");
    assert!(
        matches!(reading, SwitchReading::Blocked { .. }),
        "tracked work blocks the switch: {reading:?}"
    );
    let before = git
        .inspect_worktree(&target)
        .expect("the dirty worktree before the sync");
    let reading = git
        .sync_workspace(before, |root| git.inspect_worktree(Path::new(root)))
        .expect("a dirty worktree stops the sync without running Git");
    let SyncReading::Stopped {
        stop,
        fetched,
        fast_forwarded,
        ..
    } = reading
    else {
        panic!("a dirty worktree stops the sync: {reading:?}");
    };
    assert_eq!(stop, SyncStop::Dirty);
    assert!(!fetched);
    assert!(!fast_forwarded);
    checked(&target, &fixture.config, &["checkout", "--", "."]);

    // Attach over explicit expected facts, in every reading.
    let ready = git.read_attach(&target, "main", &fixture.remote_url());
    assert!(
        matches!(ready, AttachReading::Ready { .. }),
        "the expected branch on the expected remote is ready: {ready:?}"
    );
    let wrong_remote = git.read_attach(
        &target,
        "main",
        "https://github.com/example-owner/other-repository",
    );
    let AttachReading::WrongRemote { observed, .. } = wrong_remote else {
        panic!("another expected remote is a wrong remote: {wrong_remote:?}");
    };
    assert_eq!(
        observed.as_deref(),
        Some(fixture.remote_url().as_str()),
        "the observed remote is reported for display"
    );
    let before = git.inspect_worktree(&target).expect("the clean worktree");
    git.switch_branch(before, "local:topic", |root| {
        git.inspect_worktree(Path::new(root))
    })
    .expect("the clean worktree switches");
    let offered = git.read_attach(&target, "release", &fixture.remote_url());
    let AttachReading::SwitchOffered {
        current_branch,
        detached,
        create_from_integration_base,
        ..
    } = offered
    else {
        panic!("another branch with untracked-only content offers a switch: {offered:?}");
    };
    assert_eq!(current_branch, "topic");
    assert!(!detached);
    assert!(
        create_from_integration_base,
        "an expected branch nobody observed has to be created"
    );
    fs::write(target.join("notes.md"), "base\nfrom the peer\nlocal\nedited\n")
        .expect("tracked work on the wrong branch");
    let blocked = git.read_attach(&target, "main", &fixture.remote_url());
    let AttachReading::WrongBranchDirty {
        current_branch, ..
    } = blocked
    else {
        panic!("tracked work on another branch is a dirty wrong branch: {blocked:?}");
    };
    assert_eq!(current_branch, "topic");
    checked(&target, &fixture.config, &["checkout", "--", "."]);
    fs::remove_file(target.join("in-progress.txt")).expect("the untracked file");
    let plain = fixture.temp.path().join("not-a-repository");
    fs::create_dir_all(&plain).expect("a folder that is not a worktree");
    let not_a_repository = git.read_attach(&plain, "main", &fixture.remote_url());
    let AttachReading::NotRepository { error, .. } = not_a_repository else {
        panic!("a plain folder is not a repository: {not_a_repository:?}");
    };
    assert_eq!(error.code(), GitFailureCode::RepositoryUnavailable);

    let signed_in = sign_in(&fixture, &target);

    format!(
        "clone through staging and the exclusive rename (a second clone refused, a failed clone leaving no target), worktree status and branch reading, two preflight refusals, sync fast-forward and push, branch switch in five readings, attach in five readings, {signed_in}"
    )
}

/// Part 5b: a device-flow sign-in against the loopback server, the grant in a
/// consumer-scoped credential slot, and a fetch whose token rides only the
/// child's environment.
fn sign_in(fixture: &Fixture, target: &Path) -> String {
    let server = auth_server();
    let device_url = server.url(DEVICE_CODE_PATH);
    let token_url = server.url(ACCESS_TOKEN_PATH);
    let endpoints = DeviceFlowEndpoints::loopback_for_tests(&device_url, &token_url)
        .expect("loopback endpoint addresses");
    let cancel = AtomicBool::new(false);
    let mut progress = Vec::new();
    let grant = run_device_flow(CLIENT_ID, &endpoints, &cancel, &mut |event| {
        progress.push(event)
    })
    .expect("the loopback device flow signs in");
    assert_eq!(grant.access_token.expose_secret(), TOKEN);
    assert_eq!(
        grant
            .refresh_token
            .as_ref()
            .map(|token| token.expose_secret().to_owned()),
        Some(REFRESH_TOKEN.to_owned())
    );
    assert!(
        grant.access_expires_at.is_some(),
        "the token endpoint declared an expiry"
    );
    assert_eq!(progress.len(), 1, "one poll answered the flow: {progress:?}");
    let GithubAuthProgress::AwaitingVerification {
        user_code,
        verification_uri,
    } = &progress[0]
    else {
        panic!("the first reading carries the code: {progress:?}");
    };
    assert_eq!(user_code, USER_CODE);
    assert_eq!(
        verification_uri,
        GITHUB_DEVICE_VERIFICATION_URL,
        "the reported page is GitHub's own"
    );

    // The one-time code and the client id were posted; no token value was.
    let device_bodies = server.bodies_for(DEVICE_CODE_PATH);
    assert!(
        device_bodies
            .iter()
            .all(|body| body.contains(CLIENT_ID) && !body.contains(TOKEN)),
        "the device request carries the client id and no token: {device_bodies:?}"
    );

    let slot = CredentialSlot::consumer_scoped("conformance", "git-remote", "access-grant")
        .expect("a consumer-scoped credential slot");
    let store = InMemoryStore::default();
    let scope = CredentialScope {
        store: &store,
        slot: &slot,
    };
    assert_eq!(
        read_credential_presence(&scope),
        CredentialStoreReading::Missing,
        "the slot starts empty"
    );
    write_grant(&scope, &grant).expect("the grant is stored in the caller's slot");
    assert_eq!(
        read_credential_presence(&scope),
        CredentialStoreReading::Present
    );

    // An access token outside the refresh skew is read from the store without
    // an exchange.
    let client = GithubTokenClient::loopback_for_tests(&token_url).expect("a loopback token client");
    let token = fresh_access_token(&scope, CLIENT_ID, &client, std::time::SystemTime::now(), false, None)
        .expect("the stored grant is readable")
        .expect("the operator is signed in");
    assert_eq!(token.expose_secret(), TOKEN);
    let polls = server.bodies_for(ACCESS_TOKEN_PATH);
    assert_eq!(
        polls.len(),
        1,
        "the device poll exchanged the token and the stored grant did not"
    );
    assert!(
        polls[0].contains("urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "the poll asked for the device grant: {polls:?}"
    );
    assert!(
        polls.iter().all(|body| !body.contains(TOKEN)),
        "a token request never carries a token: {polls:?}"
    );

    // The clone's remote becomes the GitHub address the credential applies to.
    checked(
        target,
        &fixture.config,
        &["remote", "set-url", "origin", REMOTE],
    );

    let argv_record = fixture.temp.path().join("arguments.txt");
    let env_record = fixture.temp.path().join("environment.txt");
    let recorder = script(
        fixture.temp.path(),
        "record-and-refuse.sh",
        &format!(
            "printf '%s\\n' \"$@\" > \"{argv}\"\nprintf '%s\\n' \"GIT_ASKPASS=${{GIT_ASKPASS:-}}\" \"TOKEN=${{{TOKEN_ENV}:-}}\" > \"{environment}\"\necho 'fatal: Authentication failed for the remote' >&2\nexit 1",
            argv = argv_record.display(),
            environment = env_record.display()
        ),
    );
    let helper = {
        let policy = BridgePolicy::new(
            &fixture.config,
            token.clone(),
            recording_rule(&recorder),
        );
        let git = Git::new(&policy, redactor());

        // A status read never asks the policy for a credential.
        git.status(target).expect("the status of the workspace");
        assert!(
            policy.asked().is_empty(),
            "only a credential-scoped remote call asks for one: {:?}",
            policy.asked()
        );

        let refusal = git
            .run(
                target,
                &["fetch", "origin"],
                GitFailureCode::FetchFailed,
                TIMEOUT,
            )
            .expect_err("the scripted remote refuses the fetch");
        assert_eq!(
            policy.asked(),
            vec![REMOTE.to_owned()],
            "the fetch asked the policy once for the scoped remote"
        );

        let environment = fs::read_to_string(&env_record).expect("the recorded environment");
        assert!(
            environment.contains(&format!("TOKEN={TOKEN}")),
            "the token reached the child as an environment value: {environment}"
        );
        let arguments = fs::read_to_string(&argv_record).expect("the recorded arguments");
        assert!(
            arguments.contains(REMOTE),
            "the child was given the remote: {arguments}"
        );
        assert!(
            !arguments.contains(TOKEN),
            "the token is in no argument: {arguments}"
        );
        assert!(
            !refusal.detail().contains(TOKEN),
            "the token is in no failure detail: {}",
            refusal.detail()
        );
        assert!(
            !format!("{refusal:?}").contains(TOKEN),
            "the token is in no Debug output"
        );
        assert_eq!(
            classify_remote_failure(refusal.detail()),
            RemoteAccessFailure::AuthenticationRequired,
            "the remote's refusal is classified: {}",
            refusal.detail()
        );

        let askpass = environment
            .lines()
            .find_map(|line| line.strip_prefix("GIT_ASKPASS="))
            .expect("the child saw the askpass program");
        let askpass = PathBuf::from(askpass);
        assert!(
            askpass.exists(),
            "the askpass program exists for the invocation"
        );
        askpass
    };
    assert!(
        !helper.exists(),
        "the askpass folder is removed with its bridge"
    );

    // A real fetch through the same bridge: the local bare repository stands in
    // for the GitHub remote, and the token is nowhere in the returned text.
    let peer = fixture.peer("remote-peer");
    fs::write(peer.join("notes.md"), "base\nfrom the peer\nlocal\nremote\n")
        .expect("the peer's change");
    checked(&peer, &fixture.config, &["commit", "-am", "Remote change"]);
    checked(&peer, &fixture.config, &["push", "origin", "main"]);
    let peer_head = checked(&peer, &fixture.config, &["rev-parse", "HEAD"]);
    let policy = BridgePolicy::new(&fixture.config, token, file_rule(&fixture.remote));
    let git = Git::new(&policy, redactor());
    let output = git
        .run(
            target,
            &["fetch", "origin"],
            GitFailureCode::FetchFailed,
            TIMEOUT,
        )
        .expect("the local substitute for the remote answers");
    assert!(
        !output.contains(TOKEN),
        "the token is in no returned output: {output}"
    );
    let fetched = git
        .run(
            target,
            &["rev-parse", "origin/main"],
            GitFailureCode::BranchUnavailable,
            TIMEOUT,
        )
        .expect("the fetched reference is readable");
    assert_eq!(
        fetched.trim(),
        peer_head,
        "the fetch through the bridge brought the remote's commit"
    );

    "a loopback device-flow sign-in stored in a consumer-scoped slot, and a fetch whose token rode only the child environment".to_owned()
}

/// The loopback server the sign-in half reads: a device-code answer and one
/// successful token exchange. The profile route is not part of this row set.
fn auth_server() -> FixtureServer {
    FixtureServer::spawn(vec![
        FixtureRoute::new(DEVICE_CODE_PATH, 200, device_body()),
        FixtureRoute::new(ACCESS_TOKEN_PATH, 200, token_body()),
    ])
    .expect("a loopback fixture server")
}

/// The device-code answer: a code, GitHub's own verification page, and a wait
/// this proof only needs one poll of.
fn device_body() -> Vec<u8> {
    serde_json::json!({
        "device_code": "conformance-device-code",
        "user_code": USER_CODE,
        "verification_uri": GITHUB_DEVICE_VERIFICATION_URL,
        "expires_in": 60,
        "interval": 5,
    })
    .to_string()
    .into_bytes()
}

/// The token answer: one access token, its expiry, and a refresh token this
/// row set stores but never spends.
fn token_body() -> Vec<u8> {
    serde_json::json!({
        "access_token": TOKEN,
        "token_type": "bearer",
        "expires_in": 3600,
        "refresh_token": REFRESH_TOKEN,
        "refresh_token_expires_in": 7200,
    })
    .to_string()
    .into_bytes()
}
