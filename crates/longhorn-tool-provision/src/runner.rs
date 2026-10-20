//! Bounded process execution ports used by install probes and login-shell discovery.

use std::{
    ffi::OsString,
    fmt,
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
    thread,
};

/// Maximum bytes retained from either child output stream by [`CommandRunner`].
pub const MAX_PROCESS_OUTPUT_BYTES: usize = 64 * 1024;

/// A host process runner. Implementations must execute the fixed program and
/// argument vector as supplied, without interpreting them through a shell.
pub trait ProcessRunner: Send + Sync {
    /// Runs one program with an argument vector and bounded captured output.
    ///
    /// # Errors
    ///
    /// Returns a runner fault when the process cannot start, its output cannot
    /// be read, or the process wait fails.
    fn run(&self, program: &Path, arguments: &[OsString]) -> Result<ProcessOutput, RunnerError>;
}

/// Captured process status and bounded output streams.
#[derive(Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    success: bool,
    exit_status: Option<i32>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

impl fmt::Debug for ProcessOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessOutput")
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("success", &self.success)
            .field("exit_status", &self.exit_status)
            .field("stdout_truncated", &self.stdout_truncated)
            .field("stderr_truncated", &self.stderr_truncated)
            .finish()
    }
}

impl ProcessOutput {
    /// Makes a captured result, primarily for host runner implementations.
    /// The install and PATH APIs apply their own smaller output bounds.
    #[must_use]
    pub fn new(
        stdout: impl Into<Vec<u8>>,
        stderr: impl Into<Vec<u8>>,
        success: bool,
        exit_status: Option<i32>,
    ) -> Self {
        let (stdout, stdout_truncated) = cap_output(stdout.into());
        let (stderr, stderr_truncated) = cap_output(stderr.into());
        Self {
            stdout,
            stderr,
            success,
            exit_status,
            stdout_truncated,
            stderr_truncated,
        }
    }

    /// Standard output, capped by the runner and again by the consumer.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Standard error, capped by the runner and again by the consumer.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// Whether the process exited successfully.
    pub const fn success(&self) -> bool {
        self.success
    }

    /// The numeric exit code, or `None` when the process ended without one.
    pub const fn exit_status(&self) -> Option<i32> {
        self.exit_status
    }

    /// Whether standard output exceeded the runner's capture bound.
    pub const fn stdout_truncated(&self) -> bool {
        self.stdout_truncated
    }

    /// Whether standard error exceeded the runner's capture bound.
    pub const fn stderr_truncated(&self) -> bool {
        self.stderr_truncated
    }
}

fn cap_output(mut output: Vec<u8>) -> (Vec<u8>, bool) {
    let truncated = output.len() > MAX_PROCESS_OUTPUT_BYTES;
    output.truncate(MAX_PROCESS_OUTPUT_BYTES);
    (output, truncated)
}

/// Why the host could not capture a process result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnerError {
    detail: String,
}

impl RunnerError {
    /// A runner error with a host-supplied detail.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// The host-supplied description.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for RunnerError {}

/// Production runner that starts programs directly and retains bounded output.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandRunner;

impl ProcessRunner for CommandRunner {
    fn run(&self, program: &Path, arguments: &[OsString]) -> Result<ProcessOutput, RunnerError> {
        let mut child = Command::new(program)
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| RunnerError::new(format!("the process could not start: {error}")))?;
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunnerError::new(
                "the process standard output was unavailable",
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunnerError::new(
                "the process standard error was unavailable",
            ));
        };
        let stdout_reader = match thread::Builder::new().spawn(move || read_bounded(stdout)) {
            Ok(reader) => reader,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunnerError::new(format!(
                    "the output reader could not start: {error}"
                )));
            }
        };
        let stderr_reader = match thread::Builder::new().spawn(move || read_bounded(stderr)) {
            Ok(reader) => reader,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                return Err(RunnerError::new(format!(
                    "the output reader could not start: {error}"
                )));
            }
        };

        let status = match child.wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(RunnerError::new(format!(
                    "the process wait failed: {error}"
                )));
            }
        };
        let stdout = join_reader(stdout_reader);
        let stderr = join_reader(stderr_reader);
        let (stdout, stdout_truncated) = stdout?;
        let (stderr, stderr_truncated) = stderr?;
        Ok(ProcessOutput {
            stdout,
            stderr,
            success: status.success(),
            exit_status: status.code(),
            stdout_truncated,
            stderr_truncated,
        })
    }
}

fn read_bounded(mut reader: impl Read) -> io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(MAX_PROCESS_OUTPUT_BYTES);
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = MAX_PROCESS_OUTPUT_BYTES.saturating_sub(output.len());
        let retained = read.min(remaining);
        output.extend_from_slice(&buffer[..retained]);
        truncated |= retained < read;
    }
    Ok((output, truncated))
}

fn join_reader(
    reader: thread::JoinHandle<io::Result<(Vec<u8>, bool)>>,
) -> Result<(Vec<u8>, bool), RunnerError> {
    reader
        .join()
        .map_err(|_| RunnerError::new("the process output reader stopped unexpectedly"))?
        .map_err(|error| RunnerError::new(format!("the process output could not be read: {error}")))
}
