//! Shared atomic file publication helpers for plain `std::fs` paths.
//!
//! The unique-temporary-name model: a temporary named
//! `.{file_name}.{pid}.{sequence}.tmp` is created with `create_new`, so no
//! writer can collide with another writer or with a stale temporary left by
//! a crashed one, and there is no remove-then-create window where a fixed
//! name is missing. A failed write may leave one inert temporary file.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) const TEMP_ATTEMPTS: u32 = 32;

/// Process-local sequence for unique temporary names.
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Returns a fresh unique temporary name below `parent` for `file_name`.
pub(crate) fn temporary_name(parent: &Path, file_name: &str) -> PathBuf {
    parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

/// A uniquely named temporary file created with `create_new`.
pub(crate) struct UniqueTemporary {
    file: Option<File>,
    path: PathBuf,
}

impl UniqueTemporary {
    /// Creates the temporary file below `parent`, retrying on name
    /// collisions.
    pub(crate) fn create(parent: &Path, file_name: &str) -> io::Result<Self> {
        for _ in 0..TEMP_ATTEMPTS {
            let path = temporary_name(parent, file_name);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            set_private_mode(&mut options);
            match options.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file: Some(file),
                        path,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::other(
            "temporary name collision retry limit reached",
        ))
    }

    /// Returns the temporary file's path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `bytes` and syncs the temporary file.
    pub(crate) fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let file = self.file.as_mut().expect("temporary file is closed");
        file.write_all(bytes)?;
        file.sync_all()
    }

    /// Releases the file handle without removing the temporary.
    pub(crate) fn close(&mut self) {
        self.file = None;
    }

    /// Removes the temporary file.
    pub(crate) fn discard(self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }

    /// Renames the temporary onto `target` and syncs the parent directory.
    ///
    /// On rename failure the temporary is removed best-effort and the rename
    /// error is returned. A directory-sync failure is reported after the
    /// rename: the file is on disk under its final name either way.
    pub(crate) fn commit(mut self, target: &Path) -> io::Result<()> {
        self.close();
        if let Err(error) = fs::rename(&self.path, target) {
            let _ = fs::remove_file(&self.path);
            return Err(error);
        }
        sync_directory(target.parent().unwrap_or_else(|| Path::new("/")))
    }
}

/// Syncs a directory so a rename below it is durable.
pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

/// Creates a private file with `create_new` and writes `bytes`.
pub(crate) fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    set_private_mode(&mut options);
    let mut output = options.open(path)?;
    output.write_all(bytes)?;
    output.sync_all()
}

#[cfg(unix)]
fn set_private_mode(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_mode(_options: &mut OpenOptions) {}
