//! Generic Git subprocess execution, output capture bounds and redaction.
//!
//! Every entry point on [`Git`] runs the fixed program `git` with an argument
//! vector, in one explicit working folder, under the caller's
//! [`GitPolicy`]. No code path builds a shell command string.
//!
//! A child's environment is cleared of author and committer identity, prompts
//! are disabled, and the caller's isolation is applied before the invocation
//! credential, so the credential wins. Standard output is retained from its
//! head and standard error from its tail, each within
//! [`MAX_CAPTURED_OUTPUT_BYTES`]; a failure's detail is then redacted with the
//! caller's [`longhorn_redact::Redactor`] and the invocation secret, and cut to
//! [`FAILURE_DETAIL_BOUND`].

use std::{
    io::{self, Read},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use longhorn_redact::{RedactionRules, Redactor};
use longhorn_support_log::{TailBounds, bounded_tail};

use crate::{
    failure::{GitFailure, GitFailureCode},
    policy::{GitCredential, GitPolicy},
    preflight::{RemoteAccessFailure, classify_remote_failure, is_https_github},
};

/// Most bytes retained from either Git output stream by one invocation.
///
/// Standard output keeps its head, because callers parse it; standard error
/// keeps its tail, because Git reports the cause there. A successful call whose
/// standard output exceeds this bound fails with
/// [`GitFailureCode::OutputTooLarge`] rather than returning a partial answer.
pub const MAX_CAPTURED_OUTPUT_BYTES: usize = 64 * 1024;

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
    /// `code` classifies a refusal. When the bound is reached the child is
    /// killed and awaited, and the result is [`GitFailure::TimedOut`].
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
        self.run_with(root, arguments, code, Some(timeout))
    }

    pub(crate) fn run_git(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
    ) -> Result<String, GitFailure> {
        self.run_with(root, arguments, code, None)
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
            detail: redact_git_detail(
                &self.redactor,
                &String::from_utf8_lossy(&capture.stderr),
                credential.map(GitCredential::secret),
            ),
        })
    }

    fn run_with(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
        timeout: Option<Duration>,
    ) -> Result<String, GitFailure> {
        let process = self.run_process(root, arguments, code, timeout)?;
        if process.capture.success {
            if process.capture.stdout_truncated {
                return Err(too_large());
            }
            return Ok(stdout(&process.capture.stdout));
        }
        Err(GitFailure::Refused {
            code,
            detail: redact_git_detail(
                &self.redactor,
                &String::from_utf8_lossy(&process.capture.stderr),
                process.credential.as_ref().map(GitCredential::secret),
            ),
        })
    }

    /// Runs one Git invocation with the app credential when the argument vector
    /// is a remote operation on an application-credential remote, then retries
    /// once with a refreshed credential when Git reports the credential was
    /// rejected.
    fn run_process(
        &self,
        root: &Path,
        arguments: &[&str],
        code: GitFailureCode,
        timeout: Option<Duration>,
    ) -> Result<Processed, GitFailure> {
        let mut credential = self.credential_for(root, arguments)?;
        let mut capture = self.spawn(root, arguments, credential.as_ref(), code, timeout)?;
        if !capture.success
            && let Some(existing) = credential.as_ref()
            && matches!(
                classify_remote_failure(&String::from_utf8_lossy(&capture.stderr)),
                RemoteAccessFailure::AuthenticationRequired
            )
            && let Some(refreshed) = self.policy.refresh_credential(existing)
        {
            credential = Some(refreshed);
            capture = self.spawn(root, arguments, credential.as_ref(), code, timeout)?;
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
    ) -> Result<Option<GitCredential>, GitFailure> {
        if !matches!(
            arguments.first(),
            Some(&"fetch") | Some(&"push") | Some(&"ls-remote")
        ) {
            return Ok(None);
        }
        let Some(remote) = self.origin_url(root) else {
            return Ok(None);
        };
        if !is_remote_credential_operation(arguments, &remote) {
            return Ok(None);
        }
        self.policy.credential(&remote)
    }

    fn spawn(
        &self,
        root: &Path,
        arguments: &[&str],
        credential: Option<&GitCredential>,
        code: GitFailureCode,
        timeout: Option<Duration>,
    ) -> Result<Capture, GitFailure> {
        let mut command = self.command(Some(root), arguments);
        if let Some(credential) = credential {
            apply_credential(&mut command, credential);
        }
        capture(command, timeout).map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                GitFailure::TimedOut {
                    code,
                    after: timeout.unwrap_or_default(),
                }
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
    let scrubbed = match secret {
        Some(secret) => {
            Redactor::new([secret.to_owned()], RedactionRules::common())
                .redact(detail)
                .text
        }
        None => detail.to_owned(),
    };
    bounded_tail(redactor, &scrubbed, FAILURE_DETAIL_BOUND)
}

fn apply_credential(command: &mut Command, credential: &GitCredential) {
    for (key, value) in credential.env_vars() {
        command.env(key, value);
    }
}

struct Processed {
    capture: Capture,
    credential: Option<GitCredential>,
}

struct Capture {
    success: bool,
    stdout: Vec<u8>,
    stdout_truncated: bool,
    stderr: Vec<u8>,
}

fn capture(mut command: Command, timeout: Option<Duration>) -> io::Result<Capture> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let Some(stdout_pipe) = child.stdout.take() else {
        terminate(&mut child);
        return Err(io::Error::other("Git standard output was not captured"));
    };
    let Some(stderr_pipe) = child.stderr.take() else {
        terminate(&mut child);
        return Err(io::Error::other("Git standard error was not captured"));
    };
    let stdout_reader = thread::spawn(move || read_head(stdout_pipe));
    let stderr_reader = thread::spawn(move || read_tail(stderr_pipe));
    let status = wait_bounded(&mut child, timeout);
    let stdout = join(stdout_reader);
    let stderr = join(stderr_reader);
    let status = status?;
    let (stdout, stdout_truncated) = stdout?;
    let (stderr, _stderr_truncated) = stderr?;
    Ok(Capture {
        success: status.success(),
        stdout,
        stdout_truncated,
        stderr,
    })
}

/// Waits for the child, and kills and reaps it when the bound is reached or the
/// wait itself fails, so no reader is left waiting on a live child.
fn wait_bounded(child: &mut Child, timeout: Option<Duration>) -> io::Result<ExitStatus> {
    let Some(timeout) = timeout else {
        let status = child.wait();
        if status.is_err() {
            terminate(child);
        }
        return status;
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => {
                terminate(child);
                return Err(error);
            }
        }
        if Instant::now() >= deadline {
            terminate(child);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Git exceeded its wall clock",
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn join(reader: thread::JoinHandle<io::Result<(Vec<u8>, bool)>>) -> io::Result<(Vec<u8>, bool)> {
    reader
        .join()
        .map_err(|_| io::Error::other("the Git output reader stopped unexpectedly"))?
}

fn read_head(mut pipe: impl Read) -> io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::with_capacity(MAX_CAPTURED_OUTPUT_BYTES);
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        let read = pipe.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = MAX_CAPTURED_OUTPUT_BYTES.saturating_sub(retained.len());
        let keep = read.min(remaining);
        retained.extend_from_slice(&buffer[..keep]);
        truncated |= keep < read;
    }
    Ok((retained, truncated))
}

pub(crate) fn read_tail(mut pipe: impl Read) -> io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::new();
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        let read = pipe.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        retained.extend_from_slice(&buffer[..read]);
        if retained.len() > MAX_CAPTURED_OUTPUT_BYTES {
            truncated = true;
            let excess = retained.len() - MAX_CAPTURED_OUTPUT_BYTES;
            retained.drain(..excess);
        }
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
