use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos",
))]
use crate::unix;
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos",
)))]
use crate::unsupported;

/// Maximum bytes in one output event.
pub const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

/// Number of output events buffered for a session before the child is slowed.
pub const OUTPUT_QUEUE_CAPACITY: usize = 8;

/// The maximum output payload retained by a session, including the reader's
/// current chunk while it waits for queue capacity.
pub const MAX_BUFFERED_OUTPUT_BYTES: usize = OUTPUT_CHUNK_BYTES * (OUTPUT_QUEUE_CAPACITY + 1);

/// Grace between sending `SIGHUP` and escalating to `SIGKILL` on close.
pub const CLOSE_GRACE_PERIOD: Duration = Duration::from_millis(300);

/// Default pseudo-terminal size.
pub const DEFAULT_SIZE: PtySize = PtySize {
    rows: 24,
    columns: 80,
};

/// Initial or requested terminal size. Zero rows or columns are clamped to 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtySize {
    /// Number of rows.
    pub rows: u16,
    /// Number of columns.
    pub columns: u16,
}

/// Complete caller-owned process configuration.
///
/// The environment replaces the inherited environment entirely. Its values
/// are omitted from `Debug` output because they may contain credentials.
pub struct CommandSpec {
    /// Program path to execute.
    pub program: PathBuf,
    /// Arguments passed directly to the program.
    pub arguments: Vec<OsString>,
    /// Complete child environment, replacing the inherited environment.
    pub environment: BTreeMap<OsString, OsString>,
    /// Working directory for the child.
    pub working_directory: PathBuf,
    /// Initial pseudo-terminal size.
    pub size: PtySize,
}

impl fmt::Debug for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandSpec")
            .field("program", &self.program)
            .field("argument_count", &self.arguments.len())
            .field("environment_entry_count", &self.environment.len())
            .field("working_directory", &self.working_directory)
            .field("size", &self.size)
            .finish()
    }
}

/// Process termination reported once after currently readable output drains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitResult {
    /// The child returned this exit code.
    Code(i32),
    /// The child terminated from this signal number.
    Signal(i32),
}

impl ExitResult {
    /// Maps the typed result to the donor's conventional shell-style code.
    pub const fn compatibility_code(self) -> i32 {
        match self {
            Self::Code(code) => code,
            Self::Signal(signal) => 128 + signal,
        }
    }
}

/// A bounded stream event from a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PtyEvent {
    /// One output byte chunk, at most [`OUTPUT_CHUNK_BYTES`] bytes.
    Output(Vec<u8>),
    /// A classified output-reader or child-wait failure.
    Failure(Failure),
    /// The child's one terminal result, sent after currently readable output drains.
    Exit(ExitResult),
}

/// Classified failure from opening or controlling a session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// Opening the PTY master failed.
    OpenPty,
    /// Granting access to the PTY slave failed.
    GrantPty,
    /// Unlocking the PTY slave failed.
    UnlockPty,
    /// Resolving the PTY slave name failed.
    PtyName,
    /// Opening the PTY slave failed.
    OpenSlave,
    /// Duplicating a PTY descriptor failed.
    DuplicateDescriptor,
    /// Setting close-on-exec on a PTY descriptor failed.
    DescriptorFlags,
    /// Configuring the child command failed to start it.
    Spawn,
    /// Creating a session worker thread failed.
    WorkerSpawn,
    /// Writing bytes to the child failed.
    WriteInput,
    /// Resizing the PTY failed.
    Resize,
    /// Sending a process-group signal failed.
    Signal,
    /// Reading child output failed.
    ReadOutput,
    /// Waiting for or classifying the child status failed.
    Wait,
    /// The child has exited or the session has been closed.
    Ended,
    /// PTY sessions are not supported on this target.
    Unsupported,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OpenPty => "opening the pseudo-terminal failed",
            Self::GrantPty => "granting the pseudo-terminal slave failed",
            Self::UnlockPty => "unlocking the pseudo-terminal slave failed",
            Self::PtyName => "resolving the pseudo-terminal slave name failed",
            Self::OpenSlave => "opening the pseudo-terminal slave failed",
            Self::DuplicateDescriptor => "duplicating a pseudo-terminal descriptor failed",
            Self::DescriptorFlags => "setting pseudo-terminal descriptor flags failed",
            Self::Spawn => "starting the child process failed",
            Self::WorkerSpawn => "starting a session worker failed",
            Self::WriteInput => "writing to the child failed",
            Self::Resize => "resizing the pseudo-terminal failed",
            Self::Signal => "signalling the child process group failed",
            Self::ReadOutput => "reading child output failed",
            Self::Wait => "waiting for the child process failed",
            Self::Ended => "the session has ended",
            Self::Unsupported => "pseudo-terminal sessions are unsupported on this target",
        })
    }
}

impl std::error::Error for Failure {}

/// One running child process attached to a pseudo-terminal.
pub struct PtySession {
    #[cfg(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "illumos",
    ))]
    pub(crate) inner: unix::Inner,
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "illumos",
    )))]
    _private: (),
}

impl fmt::Debug for PtySession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PtySession { .. }")
    }
}

impl PtySession {
    /// Starts the configured process and returns its bounded event receiver.
    pub fn open(spec: CommandSpec) -> Result<(Self, Receiver<PtyEvent>), Failure> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        ))]
        {
            unix::open(spec)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        )))]
        {
            unsupported::open(spec)
        }
    }

    /// Writes all supplied bytes to the child's terminal input.
    pub fn write(&self, bytes: &[u8]) -> Result<(), Failure> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        ))]
        {
            unix::write(self, bytes)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        )))]
        {
            unsupported::write(self, bytes)
        }
    }

    /// Sets the terminal size and sends `SIGWINCH` to the child process group.
    pub fn resize(&self, size: PtySize) -> Result<(), Failure> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        ))]
        {
            unix::resize(self, size)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        )))]
        {
            unsupported::resize(self, size)
        }
    }

    /// Hangs up the process group, then kills it after the bounded grace period
    /// if any process in the group remains.
    pub fn close(&self) -> Result<(), Failure> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        ))]
        {
            unix::close(self)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "illumos",
        )))]
        {
            unsupported::close(self)
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
