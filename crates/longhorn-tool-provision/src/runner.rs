//! Bounded process execution ports used by install probes and login-shell discovery.

use std::{
    ffi::OsString,
    fmt,
    io::{self, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

/// Maximum bytes retained from either child output stream by [`CommandRunner`].
pub const MAX_PROCESS_OUTPUT_BYTES: usize = 64 * 1024;
/// The longest a process started by [`CommandRunner::default`] may run.
pub const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
/// How often the runner checks whether a child has exited.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// How long output may keep arriving after the child has exited. A grandchild
/// that inherits the pipes can hold them open; the runner stops waiting then.
const OUTPUT_GRACE: Duration = Duration::from_secs(5);

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

/// Production runner: starts programs directly with no standard input, retains
/// bounded output, and kills a process that runs past its time limit.
#[derive(Clone, Copy, Debug)]
pub struct CommandRunner {
    timeout: Duration,
}

impl Default for CommandRunner {
    fn default() -> Self {
        Self::with_timeout(DEFAULT_PROCESS_TIMEOUT)
    }
}

impl CommandRunner {
    /// A runner that kills any process still running after `timeout`.
    #[must_use]
    pub const fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// The run-time limit this runner enforces.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// One stream's read result, delivered by its reader thread.
type StreamResult = io::Result<(Vec<u8>, bool)>;

impl ProcessRunner for CommandRunner {
    fn run(&self, program: &Path, arguments: &[OsString]) -> Result<ProcessOutput, RunnerError> {
        let deadline = Instant::now().checked_add(self.timeout).ok_or_else(|| {
            RunnerError::new("the process timeout exceeds the supported deadline range")
        })?;
        let mut child = Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| RunnerError::new(format!("the process could not start: {error}")))?;
        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            stop(&mut child);
            return Err(RunnerError::new("the process output was unavailable"));
        };
        let (stdout_rx, stderr_rx) = match (spawn_reader(stdout), spawn_reader(stderr)) {
            (Ok(stdout_rx), Ok(stderr_rx)) => (stdout_rx, stderr_rx),
            (Err(error), _) | (_, Err(error)) => {
                stop(&mut child);
                return Err(RunnerError::new(format!(
                    "the output reader could not start: {error}"
                )));
            }
        };

        let status = match wait_until(&mut child, deadline) {
            Ok(Some(status)) => status,
            Ok(None) => {
                stop(&mut child);
                return Err(RunnerError::new(format!(
                    "the process did not finish within {} ms and was stopped",
                    self.timeout.as_millis()
                )));
            }
            Err(error) => {
                stop(&mut child);
                return Err(RunnerError::new(format!(
                    "the process wait failed: {error}"
                )));
            }
        };
        let output_deadline = Instant::now() + OUTPUT_GRACE;
        let (stdout, stdout_truncated) = receive(&stdout_rx, output_deadline)?;
        let (stderr, stderr_truncated) = receive(&stderr_rx, output_deadline)?;
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

/// Polls for the child's exit until `deadline`; `None` means it is still running.
fn wait_until(
    child: &mut Child,
    deadline: Instant,
) -> io::Result<Option<std::process::ExitStatus>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Kills a child that will not be waited for, and reaps it.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Reads one stream on its own thread. The reader is detached, so a stream
/// that never closes cannot hold the caller past its own deadline.
fn spawn_reader(reader: impl Read + Send + 'static) -> io::Result<Receiver<StreamResult>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new().spawn(move || {
        let _ = sender.send(read_bounded(reader));
    })?;
    Ok(receiver)
}

fn receive(
    receiver: &Receiver<StreamResult>,
    deadline: Instant,
) -> Result<(Vec<u8>, bool), RunnerError> {
    match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(RunnerError::new(format!(
            "the process output could not be read: {error}"
        ))),
        Err(RecvTimeoutError::Timeout) => Err(RunnerError::new(
            "the process output did not close within the time limit",
        )),
        Err(RecvTimeoutError::Disconnected) => Err(RunnerError::new(
            "the process output reader stopped unexpectedly",
        )),
    }
}

fn read_bounded(mut reader: impl Read) -> StreamResult {
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
