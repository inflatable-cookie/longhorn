use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, OnceLock, TryLockError, Weak},
    thread,
    time::{Duration, Instant},
};

use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use fs4::{FileExt, TryLockError as FileTryLockError};

const LOCK_DIRECTORY: &str = ".longhorn";
const LOCK_FILE: &str = "config.lock";
const RETRY_INTERVAL: Duration = Duration::from_millis(2);

type ProcessLock = Arc<Mutex<()>>;

static PROCESS_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();

/// Stable local authority used to coordinate all store mutations.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CoordinationAuthority {
    root: PathBuf,
}

impl CoordinationAuthority {
    /// Resolves an existing absolute coordination root to a stable identity.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, CoordinationAuthorityError> {
        let root = root.into();
        if !root.is_absolute() {
            return Err(CoordinationAuthorityError::RootNotAbsolute { root });
        }

        let resolved = std::fs::canonicalize(&root).map_err(|error| {
            CoordinationAuthorityError::ResolveFailed {
                root,
                detail: error.to_string(),
            }
        })?;

        Ok(Self { root: resolved })
    }

    /// Returns the resolved coordination root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the stable lock-file path for diagnostics.
    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LOCK_DIRECTORY).join(LOCK_FILE)
    }
}

/// Invalid coordination authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoordinationAuthorityError {
    /// The injected root was not absolute.
    RootNotAbsolute {
        /// Rejected root.
        root: PathBuf,
    },
    /// The existing root could not be resolved.
    ResolveFailed {
        /// Rejected root.
        root: PathBuf,
        /// Filesystem detail.
        detail: String,
    },
}

impl fmt::Display for CoordinationAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotAbsolute { root } => {
                write!(
                    formatter,
                    "coordination root must be absolute: {}",
                    root.display()
                )
            }
            Self::ResolveFailed { root, detail } => {
                write!(
                    formatter,
                    "cannot resolve coordination root {}: {detail}",
                    root.display()
                )
            }
        }
    }
}

impl Error for CoordinationAuthorityError {}

/// Stable coordination failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinationFailureKind {
    /// A zero-wait acquisition found another writer.
    Busy,
    /// The finite acquisition deadline elapsed.
    Timeout,
    /// Lock infrastructure returned an I/O failure.
    Io,
    /// The requested authority cannot provide local coordination.
    Unsupported,
}

/// Typed failure to acquire the mutation coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinationFailure {
    /// Stable failure category.
    pub kind: CoordinationFailureKind,
    /// Stable lock path.
    pub lock_path: PathBuf,
    /// Human-readable detail.
    pub detail: String,
}

impl fmt::Display for CoordinationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} coordination failure at {}: {}",
            self.kind,
            self.lock_path.display(),
            self.detail
        )
    }
}

impl Error for CoordinationFailure {}

#[derive(Debug)]
pub(crate) struct Coordinator {
    authority: CoordinationAuthority,
    process_lock: ProcessLock,
}

impl Coordinator {
    pub(crate) fn new(authority: CoordinationAuthority) -> Self {
        let process_lock = process_lock_for(authority.root());
        Self {
            authority,
            process_lock,
        }
    }

    pub(crate) fn acquire(
        &self,
        timeout: Duration,
    ) -> Result<CoordinationGuard<'_>, CoordinationFailure> {
        let started = Instant::now();
        let deadline = started.checked_add(timeout).ok_or_else(|| {
            self.failure(
                CoordinationFailureKind::Unsupported,
                "lock timeout is too large for the platform clock",
            )
        })?;
        let process_guard = self.acquire_process_lock(timeout, deadline)?;
        let file = self.open_lock_file()?;
        self.acquire_file_lock(&file, timeout, deadline)?;

        Ok(CoordinationGuard {
            _file: file,
            _process_guard: process_guard,
        })
    }

    pub(crate) fn authority_root(&self) -> &Path {
        self.authority.root()
    }

    fn acquire_process_lock(
        &self,
        timeout: Duration,
        deadline: Instant,
    ) -> Result<MutexGuard<'_, ()>, CoordinationFailure> {
        loop {
            match self.process_lock.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
                Err(TryLockError::WouldBlock) => self.wait_for_retry(timeout, deadline)?,
            }
        }
    }

    fn open_lock_file(&self) -> Result<File, CoordinationFailure> {
        let root = Dir::open_ambient_dir(self.authority.root(), ambient_authority())
            .map_err(|error| self.io_failure("cannot open coordination root", error))?;
        root.create_dir_all(LOCK_DIRECTORY)
            .map_err(|error| self.io_failure("cannot create lock directory", error))?;
        let directory = root
            .open_dir(LOCK_DIRECTORY)
            .map_err(|error| self.io_failure("cannot open lock directory", error))?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        set_private_mode(&mut options);
        let file = directory
            .open_with(LOCK_FILE, &options)
            .map_err(|error| self.io_failure("cannot open lock file", error))?;

        Ok(file.into_std())
    }

    fn acquire_file_lock(
        &self,
        file: &File,
        timeout: Duration,
        deadline: Instant,
    ) -> Result<(), CoordinationFailure> {
        loop {
            match FileExt::try_lock(file) {
                Ok(()) => return Ok(()),
                Err(FileTryLockError::WouldBlock) => self.wait_for_retry(timeout, deadline)?,
                Err(FileTryLockError::Error(error)) => {
                    return Err(self.io_failure("cannot acquire lock file", error));
                }
            }
        }
    }

    fn wait_for_retry(
        &self,
        timeout: Duration,
        deadline: Instant,
    ) -> Result<(), CoordinationFailure> {
        if timeout.is_zero() {
            return Err(self.failure(
                CoordinationFailureKind::Busy,
                "another writer holds the coordinator",
            ));
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(self.failure(
                CoordinationFailureKind::Timeout,
                "coordination deadline elapsed",
            ));
        }

        thread::sleep(RETRY_INTERVAL.min(deadline.saturating_duration_since(now)));
        Ok(())
    }

    fn io_failure(&self, context: &str, error: std::io::Error) -> CoordinationFailure {
        self.failure(CoordinationFailureKind::Io, format!("{context}: {error}"))
    }

    fn failure(
        &self,
        kind: CoordinationFailureKind,
        detail: impl Into<String>,
    ) -> CoordinationFailure {
        CoordinationFailure {
            kind,
            lock_path: self.authority.lock_path(),
            detail: detail.into(),
        }
    }
}

pub(crate) struct CoordinationGuard<'a> {
    _file: File,
    _process_guard: MutexGuard<'a, ()>,
}

fn process_lock_for(identity: &Path) -> ProcessLock {
    let registry = PROCESS_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = registry.lock().unwrap_or_else(|error| error.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);

    if let Some(lock) = locks.get(identity).and_then(Weak::upgrade) {
        return lock;
    }

    let lock = Arc::new(Mutex::new(()));
    locks.insert(identity.to_path_buf(), Arc::downgrade(&lock));
    lock
}

#[cfg(unix)]
fn set_private_mode(options: &mut OpenOptions) {
    use cap_std::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut OpenOptions) {}
