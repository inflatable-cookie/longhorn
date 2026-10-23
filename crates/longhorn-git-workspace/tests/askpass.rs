//! The invocation-scoped askpass credential bridge, over real Git (contract
//! 029, part 5b).
//!
//! The helper is a real program that Git runs: `git credential fill` proves it
//! answers both prompts, and a fetch proves Git reaches a remote with it. The
//! token is proved absent from arguments, output, errors and `Debug` output,
//! and the helper folder is proved gone afterwards.
//!
//! Nothing here reaches the network: the fixtures run local `git` against
//! temporary repositories and bare remotes.

mod support;

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime},
};

use longhorn_core::{CredentialSlot, MemoryCredentialStore};
use longhorn_git_workspace::{
    AccessTokenResponse, AskpassConfig, CredentialScope, Git, GitCredential, GitCredentialBridge,
    GitFailure, GitFailureCode, GitPolicy, GithubAuthFailure, TokenClient, TokenGrant,
    force_refresh_token, fresh_access_token, write_grant,
};
use secrecy::{ExposeSecret as _, SecretString};
use support::{Fixture, handle};

const GITHUB_REMOTE: &str = "https://github.com/example/content";
const TOKEN_ENV: &str = "LONGHORN_TEST_GITHUB_TOKEN";
const TOKEN: &str = "controlled-secret-token";
const CLIENT_ID: &str = "test-client-id";

fn askpass() -> AskpassConfig {
    AskpassConfig::new(TOKEN_ENV).expect("a valid askpass variable name")
}

fn bridge() -> GitCredentialBridge {
    GitCredentialBridge::from_token(&askpass(), SecretString::from(TOKEN))
        .expect("a private bridge")
}

/// The askpass program's path, read from a bridge credential's environment.
fn askpass_path(credential: &GitCredential) -> PathBuf {
    let (_, value) = credential
        .env_vars()
        .iter()
        .find(|(key, _)| key == "GIT_ASKPASS")
        .expect("the credential names the askpass program");
    PathBuf::from(value)
}

/// One bridge credential's environment, with the local substitute for the
/// remote added to its Git environment-config block.
///
/// The substitute rides the credential because the mechanism applies the
/// credential's environment after the application's isolation, and because
/// `git remote get-url` would otherwise report the substitute rather than the
/// GitHub remote the mechanism decides the credential on.
fn with_substitute(credential: &GitCredential, rules: &[(String, String)]) -> GitCredential {
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

/// The rule that points one remote at a local bare repository.
fn file_rule(local: &Path, remote: &str) -> Vec<(String, String)> {
    vec![(
        format!("url.file://{}.insteadOf", local.display()),
        remote.to_owned(),
    )]
}

/// The rule that points one remote at a local program through Git's `ext::`
/// transport.
fn ext_rule(program: &Path, remote: &str) -> Vec<(String, String)> {
    // The remote rides the command, so the scripted program records the
    // address Git was told to reach and the test can prove the token is in no
    // argument of the child Git started.
    vec![
        (
            format!("url.ext::{} {remote}.insteadOf", program.display()),
            remote.to_owned(),
        ),
        ("protocol.ext.allow".to_owned(), "always".to_owned()),
    ]
}

/// Runs `git credential fill` with one bridge credential, as Git would.
fn credential_fill(fixture: &Fixture, credential: &GitCredential, input: &str) -> String {
    let mut command = Command::new("git");
    command
        .args(["credential", "fill"])
        .env("GIT_CONFIG_GLOBAL", &fixture.config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in credential.env_vars() {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("the test host runs git");
    child
        .stdin
        .take()
        .expect("a writable standard input")
        .write_all(input.as_bytes())
        .expect("the credential input");
    let output = child.wait_with_output().expect("Git answers");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "git credential fill failed: {stderr}"
    );
    assert!(!stderr.contains(TOKEN), "{stderr}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn the_credential_carries_the_askpass_environment_and_no_token_in_debug() {
    let bridge = bridge();
    let credential = bridge.credential();
    let env = credential.env_vars();
    assert!(
        env.iter()
            .any(|(key, value)| key == "GIT_ASKPASS" && !value.is_empty())
    );
    assert!(
        env.iter()
            .any(|(key, value)| key == TOKEN_ENV && value == TOKEN)
    );
    assert!(
        env.iter()
            .any(|(key, value)| key == "GIT_TERMINAL_PROMPT" && value == "0")
    );
    // A stale machine-wide helper never runs before the bridge.
    assert!(
        env.iter()
            .any(|(key, value)| key == "GIT_CONFIG_KEY_0" && value == "credential.helper")
    );
    assert!(
        env.iter()
            .any(|(key, value)| key == "GIT_CONFIG_VALUE_0" && value.is_empty())
    );
    let debug = format!("{bridge:?} {credential:?}");
    assert!(!debug.contains(TOKEN), "{debug}");
    assert!(debug.contains("GitCredential"));
}

#[test]
fn the_helper_is_owner_only_and_its_folder_is_removed_with_the_bridge() {
    let path;
    {
        let bridge = bridge();
        let credential = bridge.credential();
        path = askpass_path(&credential);
        assert!(path.exists(), "the helper exists for the invocation");
        let program = fs::read_to_string(&path).expect("a readable helper");
        assert!(
            program.contains(&format!("${{{TOKEN_ENV}:-}}")),
            "{program}"
        );
        assert!(program.contains("x-access-token"));
        assert!(!program.contains(TOKEN), "{program}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let program_mode = fs::metadata(&path)
                .expect("the helper's metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(program_mode, 0o700, "the helper is owner-only");
            let folder = path.parent().expect("the private folder");
            let folder_mode = fs::metadata(folder)
                .expect("the folder's metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(folder_mode, 0o700, "the folder is owner-only");
        }
    }
    assert!(
        !path.exists(),
        "the helper folder is removed with its bridge"
    );
}

#[test]
fn the_helper_answers_both_of_gits_credential_prompts() {
    let fixture = Fixture::worktree();
    let bridge = bridge();
    let credential = bridge.credential();

    let without_user =
        credential_fill(&fixture, &credential, "protocol=https\nhost=github.com\n\n");
    assert!(
        without_user.contains("username=x-access-token"),
        "{without_user}"
    );
    assert!(
        without_user.contains(&format!("password={TOKEN}")),
        "{without_user}"
    );

    let with_user = credential_fill(
        &fixture,
        &credential,
        "protocol=https\nhost=github.com\nusername=octocat\n\n",
    );
    assert!(with_user.contains("username=octocat"), "{with_user}");
    assert!(
        with_user.contains(&format!("password={TOKEN}")),
        "{with_user}"
    );
}

#[test]
fn a_real_fetch_reaches_a_local_remote_through_the_bridge_credential() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    // One new commit on the remote, so a fetch has something to bring.
    let peer = fixture.peer();
    support::commit(
        &peer,
        &fixture.config,
        "content.md",
        "from the peer\n",
        "Peer",
    );
    support::checked(&peer, &fixture.config, &["push", "origin", "main"]);
    let peer_head = support::checked(&peer, &fixture.config, &["rev-parse", "HEAD"]);

    let bridge = bridge();
    let credential = with_substitute(
        &bridge.credential(),
        &file_rule(&fixture.remote, GITHUB_REMOTE),
    );
    let policy = fixture.policy().with_supplied_credential(credential);
    let git = handle(&policy);

    let output = git
        .run(
            &fixture.root,
            &["fetch", "origin"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(60),
        )
        .expect("the local substitute for the remote answers");

    assert!(!output.contains(TOKEN), "{output}");
    let fetched = support::checked(
        &fixture.root,
        &fixture.config,
        &["rev-parse", "origin/main"],
    );
    assert_eq!(fetched, peer_head, "the fetch brought the peer's commit");
}

#[cfg(unix)]
#[test]
fn the_git_child_receives_the_bridge_environment_and_no_token_in_its_arguments() {
    let fixture = Fixture::worktree();
    support::checked(
        &fixture.root,
        &fixture.config,
        &["remote", "set-url", "origin", GITHUB_REMOTE],
    );
    let argv_record = fixture.temp.path().join("argv.txt");
    let env_record = fixture.temp.path().join("env.txt");
    let recorder = support::script(
        fixture.temp.path(),
        "record-and-refuse.sh",
        &format!(
            "printf '%s\\n' \"$@\" > \"{}\"\nprintf '%s\\n' \"GIT_ASKPASS=${{GIT_ASKPASS:-}}\" \"TOKEN=${{{TOKEN_ENV}:-}}\" > \"{}\"\necho 'fatal: Authentication failed for the remote' >&2\nexit 1",
            argv_record.display(),
            env_record.display()
        ),
    );
    let bridge = bridge();
    let credential = bridge.credential();
    let helper = askpass_path(&credential);
    let credential = with_substitute(&credential, &ext_rule(&recorder, GITHUB_REMOTE));
    let policy = fixture.policy().with_supplied_credential(credential);
    let git = handle(&policy);

    let failure = git
        .run(
            &fixture.root,
            &["fetch", "origin"],
            GitFailureCode::FetchFailed,
            Duration::from_secs(60),
        )
        .expect_err("the scripted remote refuses");

    // The child saw the askpass environment and the token value.
    let environment = fs::read_to_string(&env_record).expect("the recorded environment");
    assert!(
        environment.contains(&format!("TOKEN={TOKEN}")),
        "{environment}"
    );
    assert!(environment.contains("GIT_ASKPASS="), "{environment}");
    assert!(
        environment.contains(&helper.to_string_lossy().to_string()),
        "{environment}"
    );
    assert!(helper.exists(), "the helper was present for the invocation");

    // The token is in no argument, and neither the failure nor its `Debug`
    // output carries it.
    let arguments = fs::read_to_string(&argv_record).expect("the recorded arguments");
    assert!(!arguments.contains(TOKEN), "{arguments}");
    assert!(arguments.contains(GITHUB_REMOTE), "{arguments}");
    let debug = format!("{failure:?}");
    assert!(!failure.detail().contains(TOKEN), "{}", failure.detail());
    assert!(!debug.contains(TOKEN), "{debug}");
}

/// A policy whose credential is the stored grant's, and whose refresh is the
/// part 5b refresh path building a new bridge.
struct RefreshingPolicy {
    config: PathBuf,
    refused_rule: Vec<(String, String)>,
    accepted_rule: Vec<(String, String)>,
    store: MemoryCredentialStore,
    slot: CredentialSlot,
    client: RotatingClient,
    askpass: AskpassConfig,
    refresh_calls: AtomicUsize,
    kept: Mutex<Vec<GitCredentialBridge>>,
}

impl RefreshingPolicy {
    fn scope(&self) -> CredentialScope<'_> {
        CredentialScope {
            store: &self.store,
            slot: &self.slot,
        }
    }

    fn bridge_for(
        &self,
        token: SecretString,
        rules: &[(String, String)],
    ) -> Result<GitCredential, GitFailure> {
        let bridge = GitCredentialBridge::from_token(&self.askpass, token).map_err(|_| {
            GitFailure::Unavailable {
                code: GitFailureCode::FetchFailed,
                detail: "the askpass bridge could not be written".to_owned(),
            }
        })?;
        let credential = with_substitute(&bridge.credential(), rules);
        self.kept
            .lock()
            .expect("the kept bridges are not poisoned")
            .push(bridge);
        Ok(credential)
    }
}

impl GitPolicy for RefreshingPolicy {
    fn isolate(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }

    fn credential(&self, _remote: &str) -> Result<Option<GitCredential>, GitFailure> {
        let token = fresh_access_token(
            &self.scope(),
            CLIENT_ID,
            &self.client,
            SystemTime::now(),
            false,
            None,
        )
        .map_err(|_| GitFailure::Unavailable {
            code: GitFailureCode::FetchFailed,
            detail: "the stored GitHub sign-in is unavailable".to_owned(),
        })?
        .ok_or(GitFailure::Unavailable {
            code: GitFailureCode::FetchFailed,
            detail: "no GitHub sign-in is stored".to_owned(),
        })?;
        self.bridge_for(token, &self.refused_rule).map(Some)
    }

    fn refresh_credential(&self, rejected: &GitCredential) -> Option<GitCredential> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        let token = force_refresh_token(&self.scope(), CLIENT_ID, &self.client, rejected.secret())
            .ok()??;
        self.bridge_for(token, &self.accepted_rule).ok()
    }
}

/// A token client that rotates the stored grant once.
struct RotatingClient {
    calls: AtomicUsize,
}

impl TokenClient for RotatingClient {
    fn exchange(&self, fields: &[(&str, &str)]) -> Result<AccessTokenResponse, GithubAuthFailure> {
        assert!(
            fields
                .iter()
                .any(|(key, value)| *key == "grant_type" && *value == "refresh_token"),
            "{fields:?}"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(AccessTokenResponse {
            access_token: Some("access-refreshed".to_owned()),
            expires_in: Some(28_800),
            refresh_token: Some("refresh-rotated".to_owned()),
            refresh_token_expires_in: Some(15_897_600),
            error: None,
            interval: None,
        })
    }
}

#[cfg(unix)]
#[test]
fn a_rejected_bridge_credential_is_refreshed_once_and_the_fetch_retried() {
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
    let peer = fixture.peer();
    support::commit(&peer, &fixture.config, "content.md", "refreshed\n", "Peer");
    support::checked(&peer, &fixture.config, &["push", "origin", "main"]);
    let peer_head = support::checked(&peer, &fixture.config, &["rev-parse", "HEAD"]);

    let slot = CredentialSlot::consumer_scoped("longhorn-tests", "github-com", "device-token")
        .expect("a consumer-scoped test slot");
    let store = MemoryCredentialStore::new();
    let grant = TokenGrant {
        access_token: SecretString::from("access-rejected"),
        access_expires_at: Some(SystemTime::now() + Duration::from_secs(86_400)),
        refresh_token: Some(SecretString::from("refresh-valid")),
        refresh_expires_at: Some(SystemTime::now() + Duration::from_secs(15_897_600)),
    };
    write_grant(
        &CredentialScope {
            store: &store,
            slot: &slot,
        },
        &grant,
    )
    .expect("the seeded grant");

    let policy = RefreshingPolicy {
        config: fixture.config.clone(),
        refused_rule: ext_rule(&refusal, GITHUB_REMOTE),
        accepted_rule: file_rule(&fixture.remote, GITHUB_REMOTE),
        store,
        slot: slot.clone(),
        client: RotatingClient {
            calls: AtomicUsize::new(0),
        },
        askpass: askpass(),
        refresh_calls: AtomicUsize::new(0),
        kept: Mutex::new(Vec::new()),
    };
    let git = Git::new(&policy, support::redactor());

    git.run(
        &fixture.root,
        &["fetch", "origin"],
        GitFailureCode::FetchFailed,
        Duration::from_secs(60),
    )
    .expect("the refreshed bridge reaches the local substitute for the remote");

    assert_eq!(policy.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(policy.client.calls.load(Ordering::SeqCst), 1);
    let refreshed = longhorn_git_workspace::read_grant(&CredentialScope {
        store: &policy.store,
        slot: &policy.slot,
    })
    .expect("the store is readable")
    .expect("a grant is stored");
    assert_eq!(refreshed.access_token.expose_secret(), "access-refreshed");
    let fetched = support::checked(
        &fixture.root,
        &fixture.config,
        &["rev-parse", "origin/main"],
    );
    assert_eq!(fetched, peer_head, "the retry brought the peer's commit");

    // Both bridges outlived their invocations, and both helpers were present
    // while Git ran.
    let kept = policy
        .kept
        .lock()
        .expect("the kept bridges are not poisoned");
    assert_eq!(kept.len(), 2);
    for bridge in kept.iter() {
        let credential = bridge.credential();
        assert!(askpass_path(&credential).exists());
    }
}

#[test]
fn an_empty_token_is_not_bridged_and_the_variable_name_is_validated() {
    assert_eq!(
        GitCredentialBridge::from_token(&askpass(), SecretString::from("  ")).err(),
        Some(GithubAuthFailure::CredentialStoreUnavailable)
    );
    assert!(AskpassConfig::new("EXAMPLE_GITHUB_TOKEN").is_ok());
    assert!(AskpassConfig::new("_x9").is_ok());
    for rejected in [
        "",
        "1TOKEN",
        "with space",
        "with-dash",
        "with.dot",
        "TOKEN\n",
    ] {
        assert_eq!(
            AskpassConfig::new(rejected).err(),
            Some(GithubAuthFailure::NotConfigured),
            "{rejected:?}"
        );
    }
    assert_eq!(askpass().token_env(), TOKEN_ENV);
}
