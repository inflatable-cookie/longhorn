//! Generic Git subprocess execution, output capture bounds and redaction.
//!
//! Every entry point on [`Git`] runs the fixed program `git` with an argument
//! vector, in one explicit working folder, under the caller's
//! [`GitPolicy`]. No code path builds a shell command string.
//!
//! A child's environment is cleared of author and committer identity, prompts
//! are disabled, and the caller's isolation is applied before the invocation
//! credential, so the credential wins. Both streams are retained from their
//! head, within [`MAX_CAPTURED_OUTPUT_BYTES`] and
//! [`MAX_CAPTURED_STDERR_BYTES`]; every Git text the crate returns is then
//! redacted with the caller's [`longhorn_redact::Redactor`] and the invocation
//! secret, and a failure's detail is cut to [`FAILURE_DETAIL_BOUND`].

use std::{
    ffi::OsString,
    io::{self, Read},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use longhorn_redact::{RedactionRules, Redactor};
use longhorn_support_log::{TailBounds, bound_field, bounded_tail};
use secrecy::ExposeSecret as _;
use tempfile::TempDir;

use crate::{
    askpass::{ASKPASS_DELEGATE, guarded_delegate},
    failure::{GitFailure, GitFailureCode},
    policy::{GitCredential, GitPolicy},
    preflight::{is_https_github, is_rejected_credential},
};

/// Most bytes retained from Git's standard output by one invocation.
///
/// A successful call whose standard output exceeds this bound fails with
/// [`GitFailureCode::OutputTooLarge`] rather than returning a partial answer.
pub const MAX_CAPTURED_OUTPUT_BYTES: usize = 64 * 1024;

/// Most bytes retained from Git's standard error before it is redacted.
///
/// Standard error is retained from its head, not its tail. A tail cut can
/// remove the opening line of a private-key block, and redaction no longer
/// recognises the body that follows; a cut at the end leaves an unterminated
/// block, which the rules redact through the end of the text. Beyond this
/// bound, later output is not retained and the returned detail says so.
pub const MAX_CAPTURED_STDERR_BYTES: usize = 1024 * 1024;

/// The bound one failure's Git detail keeps after redaction.
pub const FAILURE_DETAIL_BOUND: TailBounds = TailBounds {
    lines: 40,
    bytes: 4096,
};

/// The mechanism handle: one caller policy and one caller redactor.
///
/// Build it once per application subsystem and keep it for the process's
/// lifetime. Every reading takes the folder, remote or credential it applies to
/// as an argument.
pub struct Git<'a> {
    policy: &'a dyn GitPolicy,
    redactor: Redactor,
}

impl<'a> Git<'a> {
    /// Builds the handle from the caller's policy and redactor.
    ///
    /// The redactor is the caller's own: it seeds its application secrets and
    /// chooses its rules. The mechanism applies it to every detail it returns.
    #[must_use]
    pub fn new(policy: &'a dyn GitPolicy, redactor: Redactor) -> Self {
        Self { policy, redactor }
    }

    /// Runs one Git argument vector in `root` with a hard wall clock.
    ///
    /// `code` classifies a refusal. When the bound is reached the child's whole
    /// process group is killed and the child is reaped, so the call returns at
    /// its bound even when Git's own children still hold its output pipes; the
    /// result is [`GitFailure::TimedOut`].
    ///
    /// The bound governs Git processes and their output, not caller code.
    /// [`GitPolicy::credential`] and [`GitPolicy::refresh_credential`] run
    /// synchronously on the calling thread and cannot be preempted: a hook that
    /// overruns delays the return until it answers, and the call then reports
    /// [`GitFailure::TimedOut`] without starting Git again. A policy that must
    /// keep the bound returns from its hooks promptly.
    ///
    /// Standard output is redacted with the caller's redactor and the
    /// invocation secret before it is returned, so a value Git printed never
    /// leaves the crate unprotected.
    ///
    /// # Errors
    ///
    /// Returns a classified [`GitFailure`] when Git refuses, cannot run, exceeds
    /// the bound, or produces more standard output than
    /// [`MAX_CAPTURED_OUTPUT_BYTES`].
    pub fn run(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
        timeout: Duration,
    ) -> Result<String, GitFailure> {
        let executed = self.run_captured(root, arguments, code, Some(timeout))?;
        Ok(self.redacted_output(&executed.stdout, executed.credential.as_ref()))
    }

    pub(crate) fn run_git(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
    ) -> Result<String, GitFailure> {
        Ok(self.run_captured(root, arguments, code, None)?.stdout)
    }

    /// Runs one Git argument vector in `root` and returns its standard output
    /// as exact bytes, dropping only Git's single trailing record terminator
    /// (one `\n`).
    ///
    /// Unlike [`Git::run`], the bytes are never decoded, trimmed, or redacted:
    /// callers use this only for machine identifiers (canonical paths) where
    /// trimming or a lossy decode could collapse two distinct locations into
    /// one. A refusal still reports through [`redact_git_detail`], like every
    /// other failure detail.
    pub(crate) fn run_git_bytes(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
    ) -> Result<Vec<u8>, GitFailure> {
        let process = self.run_process(root, arguments, code, None)?;
        if process.capture.success {
            if process.capture.stdout_truncated {
                return Err(too_large());
            }
            return Ok(strip_record_terminator(process.capture.stdout));
        }
        Err(GitFailure::Refused {
            code,
            detail: stderr_detail(
                &self.redactor,
                &process.capture.stderr,
                process.capture.stderr_overflowed,
                process
                    .credential
                    .as_ref()
                    .map(|credential| credential.secret().expose_secret()),
            ),
        })
    }

    /// The caller's redactor, for a module that formats its own detail.
    pub(crate) fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// One non-mutating probe. `Ok(None)` means Git ran and refused; `Err`
    /// means it could not run or its answer could not be trusted.
    pub(crate) fn run_git_probe(
        &self,
        root: &Path,
        arguments: &[&str],
    ) -> Result<Option<String>, GitFailure> {
        let process = self.run_process(root, arguments, GitFailureCode::StatusFailed, None)?;
        if !process.capture.success {
            return Ok(None);
        }
        if process.capture.stdout_truncated {
            return Err(too_large());
        }
        Ok(Some(stdout(&process.capture.stdout)))
    }

    /// Runs a bounded remote probe without selecting a repository.
    pub(crate) fn run_git_unbound(
        &self,
        arguments: &[&str],
        remote: &str,
        code: GitFailureCode,
        credential: Option<&GitCredential>,
    ) -> Result<String, GitFailure> {
        let clock = Clock::new(code, Some(crate::preflight::REMOTE_PROBE_TIMEOUT));
        let resolved = if credential.is_some() {
            self.read_command(None, &["ls-remote", "--get-url", "--", remote], &clock)?
        } else {
            None
        };
        let capture = self.spawn(None, arguments, credential.zip(resolved.as_deref()), &clock)?;
        if capture.success {
            if capture.stdout_truncated {
                return Err(too_large());
            }
            return Ok(stdout(&capture.stdout));
        }
        Err(GitFailure::Refused {
            code,
            detail: stderr_detail(
                &self.redactor,
                &capture.stderr,
                capture.stderr_overflowed,
                credential.map(|credential| credential.secret().expose_secret()),
            ),
        })
    }

    /// Runs one invocation and returns its raw standard output. Callers that
    /// return it unparsed redact it first; parsing callers need the exact
    /// bytes.
    fn run_captured(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
        timeout: Option<Duration>,
    ) -> Result<Executed, GitFailure> {
        let process = self.run_process(root, arguments, code, timeout)?;
        if process.capture.success {
            if process.capture.stdout_truncated {
                return Err(too_large());
            }
            return Ok(Executed {
                stdout: stdout(&process.capture.stdout),
                credential: process.credential,
            });
        }
        Err(GitFailure::Refused {
            code,
            detail: stderr_detail(
                &self.redactor,
                &process.capture.stderr,
                process.capture.stderr_overflowed,
                process
                    .credential
                    .as_ref()
                    .map(|credential| credential.secret().expose_secret()),
            ),
        })
    }

    /// Redacts Git text for a caller: the invocation secret first, then the
    /// caller's own redactor, bounded to one captured stream.
    pub(crate) fn redacted_output(&self, text: &str, credential: Option<&GitCredential>) -> String {
        let scrubbed = redact_text(
            &self.redactor,
            text,
            credential.map(|credential| credential.secret().expose_secret()),
        );
        bound_field(&self.redactor, &scrubbed, MAX_CAPTURED_OUTPUT_BYTES)
    }

    /// Runs one Git invocation with the app credential when the argument vector
    /// is a remote operation on an application-credential remote, then retries
    /// once with a refreshed credential when Git reports the credential was
    /// rejected.
    ///
    /// One deadline, fixed here, covers the origin lookup, the first attempt,
    /// the policy's refresh and the retry: a retry never restarts the clock,
    /// and a refresh that answers after the deadline, with or without a
    /// replacement, is a timeout.
    fn run_process(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
        timeout: Option<Duration>,
    ) -> Result<Processed, GitFailure> {
        let clock = Clock::new(code, timeout);
        let remote = self.credential_remote(root, arguments, &clock)?;
        let credential = remote
            .as_deref()
            .map(|remote| self.policy.credential(remote))
            .transpose();
        if clock.expired() {
            return Err(clock.timed_out());
        }
        let mut credential = credential?.flatten();
        let mut capture = self.spawn(
            Some(root),
            arguments,
            credential.as_ref().zip(remote.as_deref()),
            &clock,
        )?;
        if !capture.success
            && let Some(existing) = credential.as_ref()
            && is_rejected_credential(&String::from_utf8_lossy(&capture.stderr))
        {
            let refreshed = self.policy.refresh_credential(existing);
            // The refresh runs on the caller's clock whatever it returns: a
            // late answer, even a declined one, is a timeout.
            if clock.expired() {
                return Err(clock.timed_out());
            }
            if let Some(refreshed) = refreshed {
                credential = Some(refreshed);
                capture = self.spawn(
                    Some(root),
                    arguments,
                    credential.as_ref().zip(remote.as_deref()),
                    &clock,
                )?;
            }
        }
        Ok(Processed {
            capture,
            credential,
        })
    }

    // Resolve Git's destination before calling the policy. Bare fetch uses
    // Git's default fetch remote; bare push has its own precedence and URLs.
    fn credential_remote(
        &self,
        root: &Path,
        arguments: &[&str],
        clock: &Clock,
    ) -> Result<Option<String>, GitFailure> {
        let Some((operation, remote)) = credential_operation(arguments) else {
            return Ok(None);
        };
        let root = Some(root);
        let resolved = if operation == "push" {
            let name = match remote {
                Some(name) => name.to_owned(),
                None => self.default_push_remote(root, clock)?,
            };
            self.read_command(
                root,
                &["remote", "get-url", "--push", "--all", "--", &name],
                clock,
            )?
        } else {
            let mut args = vec!["ls-remote", "--get-url"];
            if let Some(remote) = remote {
                args.extend(["--", remote]);
            }
            self.read_command(root, &args, clock)?
        };
        // Multiple push URLs are used by the same Git child. Only hand it a
        // credential when every destination belongs to the credential host.
        Ok(resolved.and_then(|urls| {
            urls.lines()
                .all(is_https_github)
                .then(|| urls.lines().next().map(str::to_owned))
                .flatten()
        }))
    }

    fn default_push_remote(
        &self,
        root: Option<&Path>,
        clock: &Clock,
    ) -> Result<String, GitFailure> {
        let branch =
            self.read_command(root, &["symbolic-ref", "--quiet", "--short", "HEAD"], clock)?;
        let push_key = branch
            .as_ref()
            .map(|branch| format!("branch.{branch}.pushRemote"));
        let fetch_key = branch
            .as_ref()
            .map(|branch| format!("branch.{branch}.remote"));
        for key in [
            push_key.as_deref(),
            Some("remote.pushDefault"),
            fetch_key.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(remote) = self.read_command(root, &["config", "--get", key], clock)? {
                return Ok(remote);
            }
        }
        let remotes = self.read_command(root, &["remote"], clock)?;
        Ok(remotes
            .filter(|remotes| !remotes.contains('\n'))
            .unwrap_or_else(|| "origin".to_owned()))
    }

    fn read_command(
        &self,
        root: Option<&Path>,
        arguments: &[&str],
        clock: &Clock,
    ) -> Result<Option<String>, GitFailure> {
        match self.spawn(root, arguments, None, clock) {
            Ok(read) if read.success && !read.stdout_truncated => {
                Ok(Some(stdout(&read.stdout)).filter(|value| !value.is_empty()))
            }
            Err(failure @ GitFailure::TimedOut { .. }) => Err(failure),
            _ => Ok(None),
        }
    }

    fn spawn(
        &self,
        root: Option<&Path>,
        arguments: &[&str],
        credential: Option<(&GitCredential, &str)>,
        clock: &Clock,
    ) -> Result<Capture, GitFailure> {
        if clock.expired() {
            return Err(clock.timed_out());
        }
        let capture = (|| {
            let (command, _askpass) = self.command(root, arguments, credential)?;
            capture(command, clock.deadline)
        })();
        capture.map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                clock.timed_out()
            } else {
                GitFailure::Unavailable {
                    code: GitFailureCode::CommandUnavailable,
                    detail: redact_git_detail(&self.redactor, &error.to_string(), None),
                }
            }
        })
    }

    pub(crate) fn command(
        &self,
        root: Option<&Path>,
        arguments: &[&str],
        credential: Option<(&GitCredential, &str)>,
    ) -> io::Result<(Command, Option<TempDir>)> {
        let mut command = Command::new("git");
        if let Some(root) = root {
            command.arg("-C").arg(root);
        }
        command.args(arguments).env_remove("GIT_CONFIG_COUNT");
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy();
            if name.starts_with("GIT_CONFIG_KEY_") || name.starts_with("GIT_CONFIG_VALUE_") {
                command.env_remove(key);
            }
        }
        self.policy.isolate(&mut command);

        let mut askpass = None;
        let mut askpass_program = OsString::new();
        if let Some((credential, remote)) = credential
            && let Some(host) = https_host(remote)
            && (!credential.is_github() || is_https_github(remote))
        {
            let env = credential.env_vars();
            if let Some(helper) =
                credential_env(env, "GIT_ASKPASS").filter(|helper| !helper.is_empty())
            {
                if credential.is_github() {
                    askpass_program = helper.into();
                } else {
                    let guard = guarded_delegate(&host)?;
                    askpass_program = guard.path().join("git-askpass").into_os_string();
                    command.env(ASKPASS_DELEGATE, helper);
                    askpass = Some(guard);
                }
            }
            for (key, value) in env {
                if !key.starts_with("GIT_CONFIG_") && key != ASKPASS_DELEGATE {
                    command.env(key, value);
                }
            }
            append_credential_config(&mut command, env)?;
            append_config(&mut command, [("http.followRedirects", "false")])?;
        }
        append_config(
            &mut command,
            [
                ("credential.helper", ""),
                ("credential.useHttpPath", "false"),
            ],
        )?;

        command
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", askpass_program)
            .env("SSH_ASKPASS_REQUIRE", "never")
            .env("LC_ALL", "C");
        for name in [
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
            "GIT_NAMESPACE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_CONFIG_PARAMETERS",
            "SSH_ASKPASS",
        ] {
            command.env_remove(name);
        }
        isolate_process_group(&mut command);
        Ok((command, askpass))
    }

    pub(crate) fn clone_command(
        &self,
        arguments: &[&str],
        remote: &str,
        credential: Option<&GitCredential>,
    ) -> io::Result<(Command, Option<TempDir>)> {
        let resolved = if credential.is_some() {
            let clock = Clock::new(
                GitFailureCode::RemotePreflightFailed,
                Some(crate::preflight::REMOTE_PROBE_TIMEOUT),
            );
            self.read_command(None, &["ls-remote", "--get-url", "--", remote], &clock)
                .map_err(|error| io::Error::other(error.to_string()))?
        } else {
            None
        };
        self.command(None, arguments, credential.zip(resolved.as_deref()))
    }
}

// Only known valueless flags may precede a positional origin. Treat ambiguous
// argument lists as uncredentialled. This keeps refspecs and option values from
// being mistaken for the destination.
fn credential_operation<'a>(arguments: &'a [&str]) -> Option<(&'a str, Option<&'a str>)> {
    let (&operation, args) = arguments.split_first()?;
    if !matches!(operation, "fetch" | "push" | "ls-remote") {
        return None;
    }
    if args.is_empty() {
        return Some((operation, None));
    }
    let mut args = args.iter().copied();
    while let Some(arg) = args.next() {
        match arg {
            "--" => return (args.next() == Some("origin")).then_some((operation, Some("origin"))),
            "origin" => return Some((operation, Some("origin"))),
            "-q" | "--quiet" | "-v" | "--verbose" | "--progress" | "--no-progress" | "--prune"
            | "--no-prune" | "--tags" | "--no-tags" | "--dry-run" | "-u" | "--set-upstream"
            | "--heads" | "--branches" | "--refs" | "--symref" | "--exit-code" => {}
            _ => return None,
        }
    }
    None
}

/// True when the arguments identify a remote operation eligible for a GitHub
/// credential over HTTPS. For bare commands, pass Git's resolved destination.
#[must_use]
pub fn is_remote_credential_operation(arguments: &[&str], remote: &str) -> bool {
    credential_operation(arguments).is_some() && is_https_github(remote)
}

pub(crate) fn https_host(remote: &str) -> Option<String> {
    // Git and URL parsers can disagree about backslash and whitespace. Only
    // admit unambiguous HTTPS URLs to the credential path.
    if remote.contains('\\') || remote.chars().any(char::is_whitespace) {
        return None;
    }
    let (scheme, address) = remote.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") || address.starts_with('/') {
        return None;
    }
    let url = reqwest::Url::parse(remote).ok()?;
    let host = url.host_str()?;
    if !host
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b".-:[]".contains(&byte))
    {
        return None;
    }
    let authority = address.split(['/', '?', '#']).next()?.rsplit('@').next()?;
    if !authority.is_ascii() || authority.contains('%') {
        return None;
    }
    match url.port() {
        Some(port) => Some(format!("{host}:{port}")),
        None => Some(host.to_owned()),
    }
}

fn credential_env<'a>(env: &'a [(String, String)], name: &str) -> Option<&'a str> {
    env.iter()
        .rev()
        .find_map(|(key, value)| (key == name).then_some(value.as_str()))
}

fn invalid_config() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid Git environment configuration",
    )
}

fn append_credential_config(command: &mut Command, env: &[(String, String)]) -> io::Result<()> {
    let count = credential_env(env, "GIT_CONFIG_COUNT")
        .map_or(Ok(0), |count| count.parse::<usize>())
        .map_err(|_| invalid_config())?;
    let entries = (0..count)
        .map(|index| {
            let key = credential_env(env, &format!("GIT_CONFIG_KEY_{index}"))
                .ok_or_else(invalid_config)?;
            let value = credential_env(env, &format!("GIT_CONFIG_VALUE_{index}"))
                .ok_or_else(invalid_config)?;
            Ok((key, value))
        })
        .collect::<io::Result<Vec<_>>>()?;
    append_config(command, entries)
}

fn append_config<'a>(
    command: &mut Command,
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> io::Result<()> {
    let mut index = command
        .get_envs()
        .find_map(|(key, value)| (key == "GIT_CONFIG_COUNT").then_some(value).flatten())
        .map_or(Ok(0), |count| {
            count
                .to_str()
                .and_then(|count| count.parse::<usize>().ok())
                .ok_or_else(invalid_config)
        })?;
    for (key, value) in entries {
        command
            .env(format!("GIT_CONFIG_KEY_{index}"), key)
            .env(format!("GIT_CONFIG_VALUE_{index}"), value);
        index = index.checked_add(1).ok_or_else(invalid_config)?;
    }
    command.env("GIT_CONFIG_COUNT", index.to_string());
    Ok(())
}

/// Redacts one Git detail with the caller's redactor and the invocation
/// secret, then cuts it to [`FAILURE_DETAIL_BOUND`].
///
/// Applications use this for Git text they capture themselves, so their own
/// diagnostics keep the same redaction and bound as the mechanism's failures.
#[must_use]
pub fn redact_git_detail(redactor: &Redactor, detail: &str, secret: Option<&str>) -> String {
    bounded_tail(
        redactor,
        &redact_text(redactor, detail, secret),
        FAILURE_DETAIL_BOUND,
    )
}

/// The invocation secret and the caller's redactor, in that order.
fn redact_text(redactor: &Redactor, text: &str, secret: Option<&str>) -> String {
    let scrubbed = match secret {
        Some(secret) => {
            Redactor::new([secret.to_owned()], RedactionRules::common())
                .redact(text)
                .text
        }
        None => text.to_owned(),
    };
    redactor.redact(&scrubbed).text
}

/// Redacts one captured standard-error stream for a failure detail.
///
/// The stream is retained from its head, so an opening private-key line is
/// present whenever its block starts within the capture. A stream past
/// [`MAX_CAPTURED_STDERR_BYTES`] carries a note that later output was not
/// retained instead of returning a cut that could hide a block opening.
pub(crate) fn stderr_detail(
    redactor: &Redactor,
    bytes: &[u8],
    overflowed: bool,
    secret: Option<&str>,
) -> String {
    let mut text = redact_text(redactor, &String::from_utf8_lossy(bytes), secret);
    if overflowed {
        text.push_str(&format!(
            "\nGit standard error exceeded {MAX_CAPTURED_STDERR_BYTES} bytes; later output was not retained."
        ));
    }
    bounded_tail(redactor, &text, FAILURE_DETAIL_BOUND)
}

/// One call's wall clock: a deadline fixed when the call starts, the bound the
/// caller asked for, and the code a timeout reports.
struct Clock {
    deadline: Option<Instant>,
    bound: Duration,
    code: GitFailureCode,
}

impl Clock {
    fn new(code: GitFailureCode, timeout: Option<Duration>) -> Self {
        Self {
            deadline: timeout.map(|bound| Instant::now() + bound),
            bound: timeout.unwrap_or_default(),
            code,
        }
    }

    fn expired(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    fn timed_out(&self) -> GitFailure {
        GitFailure::TimedOut {
            code: self.code,
            after: self.bound,
        }
    }
}

struct Processed {
    capture: Capture,
    credential: Option<GitCredential>,
}

struct Executed {
    stdout: String,
    credential: Option<GitCredential>,
}

struct Capture {
    success: bool,
    stdout: Vec<u8>,
    stdout_truncated: bool,
    stderr: Vec<u8>,
    stderr_overflowed: bool,
}

fn capture(mut command: Command, deadline: Option<Instant>) -> io::Result<Capture> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let child_pid = child.id();
    let Some(stdout_pipe) = child.stdout.take() else {
        kill_child_tree(&mut child);
        return Err(io::Error::other("Git standard output was not captured"));
    };
    let Some(stderr_pipe) = child.stderr.take() else {
        kill_child_tree(&mut child);
        return Err(io::Error::other("Git standard error was not captured"));
    };
    let (stdout_sender, stdout_receiver) = mpsc::channel();
    let (stderr_sender, stderr_receiver) = mpsc::channel();
    let stdout_reader = thread::spawn(move || {
        let _ = stdout_sender.send(read_capped(stdout_pipe, MAX_CAPTURED_OUTPUT_BYTES));
    });
    let stderr_reader = thread::spawn(move || {
        let _ = stderr_sender.send(read_capped(stderr_pipe, MAX_CAPTURED_STDERR_BYTES));
    });

    let status = wait_bounded(&mut child, deadline);
    // The bound covers the drain too. Git can exit while a shell alias, a
    // remote helper or another descendant still holds its output pipes; a call
    // that is still waiting for that output has not finished inside its bound.
    let drained = receive(&stdout_receiver, deadline)
        .and_then(|stdout| receive(&stderr_receiver, deadline).map(|stderr| (stdout, stderr)));
    let drained = match drained {
        Ok(drained) => Ok(drained),
        Err(error) => {
            if error.kind() == io::ErrorKind::TimedOut {
                // The direct child may already be reaped, so end the group by
                // the id captured at spawn.
                kill_process_group_of(child_pid);
            }
            Err(error)
        }
    };
    // A reader that could not deliver before the deadline ends once the group
    // is gone; join both so no reader outlives the call.
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    let (stdout, stderr) = drained?;
    let status = status?;
    let (stdout, stdout_truncated) = stdout;
    let (stderr, stderr_overflowed) = stderr;
    Ok(Capture {
        success: status.success(),
        stdout,
        stdout_truncated,
        stderr,
        stderr_overflowed,
    })
}

/// Waits for one reader's captured stream, bounded by the call's deadline.
fn receive<T>(
    receiver: &mpsc::Receiver<io::Result<T>>,
    deadline: Option<Instant>,
) -> io::Result<T> {
    match deadline {
        None => receiver
            .recv()
            .map_err(|_| io::Error::other("the Git output reader stopped unexpectedly"))?,
        Some(deadline) => {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(received) => received,
                Err(RecvTimeoutError::Timeout) => Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Git output was still held at its wall clock",
                )),
                Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(
                    "the Git output reader stopped unexpectedly",
                )),
            }
        }
    }
}

/// Waits for the child, and kills and reaps it when the bound is reached or the
/// wait itself fails, so no reader is left waiting on a live child.
fn wait_bounded(child: &mut Child, deadline: Option<Instant>) -> io::Result<ExitStatus> {
    let Some(deadline) = deadline else {
        let status = child.wait();
        if status.is_err() {
            kill_child_tree(child);
        }
        return status;
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => {
                kill_child_tree(child);
                return Err(error);
            }
        }
        if Instant::now() >= deadline {
            kill_child_tree(child);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Git exceeded its wall clock",
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// Puts one Git child in its own process group, so a bound or a cancellation
/// can end every process it started instead of only the direct child.
#[cfg(unix)]
pub(crate) fn isolate_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

/// No process group is configured on this host.
#[cfg(not(unix))]
pub(crate) fn isolate_process_group(_command: &mut Command) {}

/// Ends one process group by the id its leader had at spawn.
pub(crate) fn kill_process_group_of(pid: u32) {
    #[cfg(unix)]
    {
        if let Ok(pid) = i32::try_from(pid)
            && let Some(pid) = rustix::process::Pid::from_raw(pid)
        {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pid;
}

/// Kills the child's whole process group where the host has one, then kills
/// and reaps the direct child. A descendant holding an output pipe therefore
/// cannot keep a bounded call alive.
pub(crate) fn kill_child_tree(child: &mut Child) {
    kill_process_group_of(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn read_capped(mut pipe: impl Read, cap: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::with_capacity(cap.min(64 * 1024));
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        let read = pipe.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = cap.saturating_sub(retained.len());
        let keep = read.min(remaining);
        retained.extend_from_slice(&buffer[..keep]);
        truncated |= keep < read;
    }
    Ok((retained, truncated))
}

fn stdout(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_owned()
}

/// Drops Git's single trailing record terminator (one `\n`) and nothing
/// else: leading whitespace and trailing whitespace the value itself carries
/// are preserved byte for byte.
fn strip_record_terminator(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    bytes
}

fn too_large() -> GitFailure {
    GitFailure::Unavailable {
        code: GitFailureCode::OutputTooLarge,
        detail: format!(
            "Git produced more than {MAX_CAPTURED_OUTPUT_BYTES} bytes of standard output."
        ),
    }
}

#[cfg(test)]
mod record_terminator_tests {
    use super::strip_record_terminator;

    #[test]
    fn drops_only_the_single_trailing_newline() {
        assert_eq!(
            strip_record_terminator(b"/repo/repository \n".to_vec()),
            b"/repo/repository ".to_vec(),
        );
        assert_eq!(
            strip_record_terminator(b"/repo/repository\n\n".to_vec()),
            b"/repo/repository\n".to_vec(),
        );
        assert_eq!(
            strip_record_terminator(b"  /repo/repository".to_vec()),
            b"  /repo/repository".to_vec(),
        );
        assert_eq!(strip_record_terminator(Vec::new()), Vec::<u8>::new());
        assert_eq!(
            strip_record_terminator(b"/repo/repo\xff".to_vec()),
            b"/repo/repo\xff".to_vec(),
        );
    }
}
