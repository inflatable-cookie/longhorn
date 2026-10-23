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

use crate::{
    failure::{GitFailure, GitFailureCode},
    policy::{GitCredential, GitPolicy},
    preflight::{RemoteAccessFailure, classify_remote_failure, is_https_github},
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

    /// The caller's redactor, for a module that formats its own detail.
    pub(crate) fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// Applies the caller's isolation to one command this crate builds itself.
    pub(crate) fn isolate(&self, command: &mut Command) {
        self.policy.isolate(command);
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

    /// Runs Git with no repository and no credential, for a remote probe.
    pub(crate) fn run_git_unbound(
        &self,
        arguments: &[&str],
        code: GitFailureCode,
        credential: Option<&GitCredential>,
    ) -> Result<String, GitFailure> {
        let mut command = self.command(None, arguments);
        if let Some(credential) = credential {
            apply_credential(&mut command, credential);
        }
        let capture = capture(command, None).map_err(|error| GitFailure::Unavailable {
            code: GitFailureCode::CommandUnavailable,
            detail: redact_git_detail(&self.redactor, &error.to_string(), None),
        })?;
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
        let clock = Clock {
            deadline: timeout.map(|bound| Instant::now() + bound),
            bound: timeout.unwrap_or_default(),
            code,
        };
        let mut credential = self.credential_for(root, arguments, &clock)?;
        let mut capture = self.spawn(root, arguments, credential.as_ref(), &clock)?;
        if !capture.success
            && let Some(existing) = credential.as_ref()
            && matches!(
                classify_remote_failure(&String::from_utf8_lossy(&capture.stderr)),
                RemoteAccessFailure::AuthenticationRequired
            )
        {
            let refreshed = self.policy.refresh_credential(existing);
            // The refresh runs on the caller's clock whatever it returns: a
            // late answer, even a declined one, is a timeout.
            if clock.expired() {
                return Err(clock.timed_out());
            }
            if let Some(refreshed) = refreshed {
                credential = Some(refreshed);
                capture = self.spawn(root, arguments, credential.as_ref(), &clock)?;
            }
        }
        Ok(Processed {
            capture,
            credential,
        })
    }

    /// Decides the credential posture for a remote Git operation. A missing
    /// credential and a credential error are both the caller's error: the
    /// operation stops rather than running Git with an ambient helper.
    fn credential_for(
        &self,
        root: &Path,
        arguments: &[&str],
        clock: &Clock,
    ) -> Result<Option<GitCredential>, GitFailure> {
        if !matches!(
            arguments.first(),
            Some(&"fetch") | Some(&"push") | Some(&"ls-remote")
        ) {
            return Ok(None);
        }
        // The origin lookup runs under the caller's clock; an unreadable
        // origin means no credential, as an absent one does.
        let remote = match self.spawn(root, &["remote", "get-url", "origin"], None, clock) {
            Ok(origin) if origin.success && !origin.stdout_truncated => stdout(&origin.stdout),
            Err(failure @ GitFailure::TimedOut { .. }) => return Err(failure),
            Ok(_) | Err(_) => return Ok(None),
        };
        if remote.is_empty() || !is_remote_credential_operation(arguments, &remote) {
            return Ok(None);
        }
        let credential = self.policy.credential(&remote);
        // The lookup runs on the caller's clock whatever it returns: a late
        // answer, a credential or an error, is a timeout.
        if clock.expired() {
            return Err(clock.timed_out());
        }
        credential
    }

    fn spawn(
        &self,
        root: &Path,
        arguments: &[&str],
        credential: Option<&GitCredential>,
        clock: &Clock,
    ) -> Result<Capture, GitFailure> {
        let mut command = self.command(Some(root), arguments);
        if let Some(credential) = credential {
            apply_credential(&mut command, credential);
        }
        capture(command, clock.deadline).map_err(|error| {
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

    fn command(&self, root: Option<&Path>, arguments: &[&str]) -> Command {
        let mut command = Command::new("git");
        if let Some(root) = root {
            command.arg("-C").arg(root);
        }
        command
            .args(arguments)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env_remove("GIT_AUTHOR_NAME")
            .env_remove("GIT_AUTHOR_EMAIL")
            .env_remove("GIT_COMMITTER_NAME")
            .env_remove("GIT_COMMITTER_EMAIL");
        // Isolation first: the credential and any caller environment are
        // applied afterwards and win.
        self.policy.isolate(&mut command);
        isolate_process_group(&mut command);
        command
    }
}

/// True when this argument vector is a remote operation that must use the
/// caller's credential: `fetch`, `push` or `ls-remote` naming `origin` on an
/// GitHub remote over HTTP or HTTPS.
#[must_use]
pub fn is_remote_credential_operation(arguments: &[&str], remote: &str) -> bool {
    matches!(
        arguments.first(),
        Some(&"fetch") | Some(&"push") | Some(&"ls-remote")
    ) && (arguments.len() == 1
        || arguments
            .iter()
            .skip(1)
            .any(|argument| *argument == "origin"))
        && is_https_github(remote)
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

fn apply_credential(command: &mut Command, credential: &GitCredential) {
    for (key, value) in credential.env_vars() {
        command.env(key, value);
    }
}

/// One call's wall clock: a deadline fixed when the call starts, the bound the
/// caller asked for, and the code a timeout reports.
struct Clock {
    deadline: Option<Instant>,
    bound: Duration,
    code: GitFailureCode,
}

impl Clock {
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

fn too_large() -> GitFailure {
    GitFailure::Unavailable {
        code: GitFailureCode::OutputTooLarge,
        detail: format!(
            "Git produced more than {MAX_CAPTURED_OUTPUT_BYTES} bytes of standard output."
        ),
    }
}
