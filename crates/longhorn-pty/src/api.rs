use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::imp;

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
///
/// OS-backed failures carry the numeric OS error code (`raw_os_error`) rather
/// than localized or path-bearing diagnostic strings, keeping errors typed and
/// redaction-safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// Opening the PTY master failed.
    OpenPty {
        /// Raw OS error code from `openpt`.
        raw_os_error: i32,
    },
    /// Granting access to the PTY slave failed.
    GrantPty {
        /// Raw OS error code from `grantpt`.
        raw_os_error: i32,
    },
    /// Unlocking the PTY slave failed.
    UnlockPty {
        /// Raw OS error code from `unlockpt`.
        raw_os_error: i32,
    },
    /// Resolving the PTY slave name failed.
    PtyName {
        /// Raw OS error code from `ptsname`.
        raw_os_error: i32,
    },
    /// Opening the PTY slave failed.
    OpenSlave {
        /// Raw OS error code from opening the PTY slave device.
        raw_os_error: i32,
    },
    /// Duplicating a PTY descriptor failed.
    DuplicateDescriptor {
        /// Raw OS error code from duplicating the descriptor.
        raw_os_error: i32,
    },
    /// Setting close-on-exec or non-blocking flags on a PTY descriptor failed.
    DescriptorFlags {
        /// Raw OS error code from updating descriptor flags.
        raw_os_error: i32,
    },
    /// Configuring the child command failed to start it.
    Spawn {
        /// Raw OS error code from spawning the child, when reported.
        raw_os_error: Option<i32>,
    },
    /// Creating a session worker thread failed.
    WorkerSpawn {
        /// Raw OS error code from spawning the worker thread, when reported.
        raw_os_error: Option<i32>,
    },
    /// Writing bytes to the child failed.
    WriteInput {
        /// Raw OS error code from writing to the PTY master, when reported.
        raw_os_error: Option<i32>,
    },
    /// Resizing the PTY failed.
    Resize {
        /// Raw OS error code from setting the terminal window size.
        raw_os_error: i32,
    },
    /// Sending a process-group signal failed.
    Signal {
        /// Raw OS error code from signalling the child process group.
        raw_os_error: i32,
    },
    /// Reading child output failed.
    ReadOutput {
        /// Raw OS error code from reading or polling the PTY master, when reported.
        raw_os_error: Option<i32>,
    },
    /// Waiting for or classifying the child status failed.
    Wait {
        /// Raw OS error code from waiting on the child process.
        raw_os_error: i32,
    },
    /// The child has exited or the session has been closed.
    Ended,
    /// PTY sessions are not supported on this target.
    Unsupported,
}

impl Failure {
    /// Returns the raw OS error code associated with this failure, if any.
    pub const fn raw_os_error(self) -> Option<i32> {
        match self {
            Self::OpenPty { raw_os_error }
            | Self::GrantPty { raw_os_error }
            | Self::UnlockPty { raw_os_error }
            | Self::PtyName { raw_os_error }
            | Self::OpenSlave { raw_os_error }
            | Self::DuplicateDescriptor { raw_os_error }
            | Self::DescriptorFlags { raw_os_error }
            | Self::Resize { raw_os_error }
            | Self::Signal { raw_os_error }
            | Self::Wait { raw_os_error } => Some(raw_os_error),
            Self::Spawn { raw_os_error }
            | Self::WorkerSpawn { raw_os_error }
            | Self::WriteInput { raw_os_error }
            | Self::ReadOutput { raw_os_error } => raw_os_error,
            Self::Ended | Self::Unsupported => None,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::OpenPty { .. } => "opening the pseudo-terminal failed",
            Self::GrantPty { .. } => "granting the pseudo-terminal slave failed",
            Self::UnlockPty { .. } => "unlocking the pseudo-terminal slave failed",
            Self::PtyName { .. } => "resolving the pseudo-terminal slave name failed",
            Self::OpenSlave { .. } => "opening the pseudo-terminal slave failed",
            Self::DuplicateDescriptor { .. } => "duplicating a pseudo-terminal descriptor failed",
            Self::DescriptorFlags { .. } => "setting pseudo-terminal descriptor flags failed",
            Self::Spawn { .. } => "starting the child process failed",
            Self::WorkerSpawn { .. } => "starting a session worker failed",
            Self::WriteInput { .. } => "writing to the child failed",
            Self::Resize { .. } => "resizing the pseudo-terminal failed",
            Self::Signal { .. } => "signalling the child process group failed",
            Self::ReadOutput { .. } => "reading child output failed",
            Self::Wait { .. } => "waiting for the child process failed",
            Self::Ended => "the session has ended",
            Self::Unsupported => "pseudo-terminal sessions are unsupported on this target",
        };
        match self.raw_os_error() {
            Some(code) => write!(f, "{message} (os error {code})"),
            None => f.write_str(message),
        }
    }
}

impl std::error::Error for Failure {}

/// One running child process attached to a pseudo-terminal.
pub struct PtySession {
    pub(crate) inner: imp::Inner,
}

impl fmt::Debug for PtySession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PtySession { .. }")
    }
}

impl PtySession {
    /// Starts the configured process and returns its bounded event receiver.
    pub fn open(spec: CommandSpec) -> Result<(Self, Receiver<PtyEvent>), Failure> {
        imp::open(spec)
    }

    /// Writes all supplied bytes to the child's terminal input.
    pub fn write(&self, bytes: &[u8]) -> Result<(), Failure> {
        imp::write(self, bytes)
    }

    /// Sets the terminal size and sends `SIGWINCH` to the child process group.
    pub fn resize(&self, size: PtySize) -> Result<(), Failure> {
        imp::resize(self, size)
    }

    /// Hangs up the process group, then kills it after the bounded grace period
    /// if any process in the group remains.
    pub fn close(&self) -> Result<(), Failure> {
        imp::close(self)
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
