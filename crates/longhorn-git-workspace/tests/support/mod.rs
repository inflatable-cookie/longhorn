//! Test support: real local Git fixtures and scripted policies.
#![allow(dead_code)]
//!
//! Every fixture lives in its own `tempfile` directory. Nothing here reads the
//! network, writes outside that directory, or depends on the machine's Git
//! configuration: each fixture carries an empty global config and disables the
//! system one.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
};

use longhorn_git_workspace::{Git, GitCredential, GitFailure, GitPolicy};
use longhorn_redact::{RedactionRules, Redactor};
use tempfile::TempDir;

/// The redactor tests hand the mechanism: no application secrets, common
/// rules.
pub fn redactor() -> Redactor {
    Redactor::new(Vec::new(), RedactionRules::common())
}

/// One handle over a scripted policy.
pub fn handle(policy: &Policy) -> Git<'_> {
    Git::new(policy, redactor())
}

/// A scripted policy: explicit isolation, an optional credential, an optional
/// credential failure, and a record of every remote it was asked about.
pub struct Policy {
    isolation: Vec<(String, String)>,
    credential: Option<(Vec<(String, String)>, String)>,
    credential_error: Option<GitFailure>,
    credential_calls: Mutex<Vec<String>>,
}

impl Policy {
    /// A policy that isolates Git from the machine's own configuration.
    #[must_use]
    pub fn isolated(config: &Path) -> Self {
        Self {
            isolation: vec![
                (
                    "GIT_CONFIG_GLOBAL".to_owned(),
                    config.to_string_lossy().into_owned(),
                ),
                ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
            ],
            credential: None,
            credential_error: None,
            credential_calls: Mutex::new(Vec::new()),
        }
    }

    /// A policy that supplies one credential for every credential-scoped
    /// remote it is asked about.
    #[must_use]
    pub fn with_credential(mut self, env: Vec<(String, String)>, secret: &str) -> Self {
        self.credential = Some((env, secret.to_owned()));
        self
    }

    /// A policy whose credential lookup fails, as a locked store would.
    #[must_use]
    pub fn with_credential_error(mut self, failure: GitFailure) -> Self {
        self.credential_error = Some(failure);
        self
    }

    /// Every remote the mechanism asked this policy for a credential about.
    #[must_use]
    pub fn credential_calls(&self) -> Vec<String> {
        self.credential_calls
            .lock()
            .expect("the credential call log is not poisoned")
            .clone()
    }
}

impl GitPolicy for Policy {
    fn isolate(&self, command: &mut Command) {
        for (key, value) in &self.isolation {
            command.env(key, value);
        }
    }

    fn credential(&self, remote: &str) -> Result<Option<GitCredential>, GitFailure> {
        self.credential_calls
            .lock()
            .expect("the credential call log is not poisoned")
            .push(remote.to_owned());
        if let Some(failure) = &self.credential_error {
            return Err(failure.clone());
        }
        Ok(self
            .credential
            .as_ref()
            .map(|(env, secret)| GitCredential::new(env.clone(), secret.clone())))
    }

    fn refresh_credential(&self, _rejected: &GitCredential) -> Option<GitCredential> {
        // Offline tests cannot provoke a real authentication refusal, so the
        // refresh posture is "no replacement".
        None
    }
}

/// The child environment that points one HTTPS remote at a local bare
/// repository. It is delivered only through a supplied credential, so a
/// successful operation proves the bridge environment reached the child.
pub fn rewrite_env(local: &Path, remote: &str) -> Vec<(String, String)> {
    vec![
        ("GIT_CONFIG_COUNT".to_owned(), "2".to_owned()),
        (
            "GIT_CONFIG_KEY_0".to_owned(),
            format!("url.file://{}.insteadOf", local.display()),
        ),
        ("GIT_CONFIG_VALUE_0".to_owned(), remote.to_owned()),
        (
            "GIT_CONFIG_KEY_1".to_owned(),
            "credential.helper".to_owned(),
        ),
        ("GIT_CONFIG_VALUE_1".to_owned(), String::new()),
    ]
}

/// One worktree tracking a bare local remote, with an empty global config.
pub struct Fixture {
    /// The fixture's own temporary directory; dropping it removes the whole
    /// fixture.
    pub temp: TempDir,
    /// The worktree on `main`.
    pub root: PathBuf,
    /// The bare remote the worktree pushes to.
    pub remote: PathBuf,
    /// The empty global Git config every command in this fixture uses.
    pub config: PathBuf,
}

impl Fixture {
    /// Builds a worktree on `main` that tracks a bare local remote, with one
    /// extra local branch and one remote-tracking branch.
    #[must_use]
    pub fn worktree() -> Self {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let config = temp.path().join("gitconfig");
        fs::write(&config, "# empty fixture config\n").expect("an empty config");
        let remote = temp.path().join("remote.git");
        checked(
            temp.path(),
            &config,
            &["init", "--bare", remote.to_str().expect("a remote path")],
        );
        let root = temp.path().join("worktree");
        fs::create_dir_all(root.join("content/notes")).expect("the framed content folder");
        fs::write(root.join("content/notes/intro.md"), "base\n").expect("framed content");
        fs::write(root.join("content.md"), "base\n").expect("tracked content");
        checked(&root, &config, &["init", "-b", "main"]);
        checked(&root, &config, &["config", "user.name", "Longhorn Test"]);
        checked(
            &root,
            &config,
            &["config", "user.email", "test@example.invalid"],
        );
        checked(&root, &config, &["add", "."]);
        checked(&root, &config, &["commit", "-m", "Initial"]);
        checked(
            &root,
            &config,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("a remote path"),
            ],
        );
        checked(&root, &config, &["push", "-u", "origin", "main"]);
        // `init --bare` leaves HEAD on `master`; a clone of the fixture must
        // land on `main`.
        checked(
            &remote,
            &config,
            &["symbolic-ref", "HEAD", "refs/heads/main"],
        );
        checked(&root, &config, &["branch", "client-a"]);
        checked(
            &root,
            &config,
            &["update-ref", "refs/remotes/origin/client-b", "HEAD"],
        );
        Self {
            temp,
            root,
            remote,
            config,
        }
    }

    /// A fixture policy with no credential.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy::isolated(&self.config)
    }

    /// A second checkout of the same remote, for peer commits.
    #[must_use]
    pub fn peer(&self) -> PathBuf {
        let peer = self.temp.path().join("peer");
        checked(
            self.temp.path(),
            &self.config,
            &[
                "clone",
                "--branch",
                "main",
                self.remote.to_str().expect("a remote path"),
                peer.to_str().expect("a peer path"),
            ],
        );
        checked(
            &peer,
            &self.config,
            &["config", "user.name", "Longhorn Peer"],
        );
        checked(
            &peer,
            &self.config,
            &["config", "user.email", "peer@example.invalid"],
        );
        peer
    }
}

/// Runs one Git command with the fixture's isolation, for test setup only.
pub fn run_in(root: &Path, config: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .expect("the test host runs git")
}

/// Runs one Git command and returns its trimmed standard output.
pub fn checked(root: &Path, config: &Path, arguments: &[&str]) -> String {
    let output = run_in(root, config, arguments);
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Whether one Git command succeeded.
pub fn succeeds(root: &Path, config: &Path, arguments: &[&str]) -> bool {
    run_in(root, config, arguments).status.success()
}

/// Writes one path and commits it.
pub fn commit(root: &Path, config: &Path, path: &str, content: &str, message: &str) {
    fs::write(root.join(path), content).expect("the committed path");
    checked(root, config, &["add", path]);
    checked(root, config, &["commit", "-m", message]);
}

/// Every `.creating-` staging leftover under one parent.
pub fn staging_leftovers(parent: &Path) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("the fixture parent")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".creating-"))
        })
        .collect()
}
