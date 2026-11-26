//! Generic clone into an invocation-owned staging folder.
//!
//! An invocation owns one unique staging directory under the destination's
//! parent; the requested target stays absent until a same-parent exclusive
//! rename. Nothing here decides which remote is allowed, what a workspace is,
//! or what a failure says: the application supplies the remote, its credential
//! and the copy.
//!
//! The composition rule is the application's to keep: create the staging
//! folder beside the target, clone into it, remove it on every failure with
//! [`OwnedStaging::remove`], and move it into place only with
//! [`rename_exclusive`]. A refused clone then leaves no partial target.
//!
//! [`create_owned_staging`] returns [`OwnedStaging`], a handle for the one
//! folder it made. Cleanup is a consuming method on that handle, so it can name
//! no other path, and it returns the operating system's error when the removal
//! fails instead of dropping it.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

use secrecy::ExposeSecret as _;
use uuid::Uuid;

use crate::{
    execution::{
        Git, MAX_CAPTURED_STDERR_BYTES, exited_uncollected, kill_child_tree, read_capped,
        redact_git_detail, stderr_detail,
    },
    policy::GitCredential,
};

/// Why a clone attempt did not produce a workspace. The application owns the
/// copy and the remote-refusal projection.
#[derive(Debug)]
pub enum CloneFailure {
    /// The operator cancelled; the caller removes only its own staging.
    Cancelled,
    /// The destination cannot be named to Git.
    PathNotUsable,
    /// The Git child could not be started. Detail is the operating system's.
    NotStarted {
        /// The operating system's own words, redacted and bounded.
        detail: String,
    },
    /// The started Git child could not be awaited.
    WaitFailed {
        /// The operating system's own words, redacted and bounded.
        detail: String,
    },
    /// Git ran and refused.
    Refused {
        /// Git's own redacted, bounded words. The caller classifies these for
        /// the remote refusal they represent.
        detail: String,
    },
}

/// True when anything already occupies `path`, including a symlink.
#[must_use]
pub fn target_already_present(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// The destination's parent, or `None` when the path cannot be used.
#[must_use]
pub fn staging_parent(target: &Path) -> Option<&Path> {
    match target.parent() {
        None => None,
        Some(parent) if parent.as_os_str().is_empty() => Some(Path::new(".")),
        Some(parent) => Some(parent),
    }
}

/// One staging directory [`create_owned_staging`] made for this invocation.
///
/// The handle exists so cleanup names only a folder this invocation created:
/// [`OwnedStaging::remove`] takes the handle, not a path the caller assembles,
/// and a removal that fails is returned rather than dropped. The caller hands
/// [`OwnedStaging::path`] to [`Git::clone_into`] and to [`rename_exclusive`].
#[derive(Debug)]
pub struct OwnedStaging {
    path: PathBuf,
}

impl OwnedStaging {
    /// The staging folder this invocation owns.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Removes the staging folder this invocation created, consuming the
    /// handle.
    ///
    /// Consuming the handle means one cleanup per staging folder, and it means
    /// a caller whose staging was moved into place can no longer remove it.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error, naming the folder, when the
    /// recursive removal fails: a permission the removal needs is absent, or
    /// something inside still holds a file the removal cannot unlink. A folder
    /// [`rename_exclusive`] already moved into place is gone, so its removal
    /// reports [`io::ErrorKind::NotFound`]: the composition removes staging on
    /// failure, not after a move.
    pub fn remove(self) -> io::Result<()> {
        let path = self.path;
        fs::remove_dir_all(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot remove staging {}: {error}", path.display()),
            )
        })
    }
}

/// Creates one unique staging directory this invocation owns.
///
/// The name keeps the target's own name and appends `.creating-` and a v7
/// UUID, so a leftover is recognisable beside the target it belongs to.
///
/// # Errors
///
/// Returns the operating system's error when the directory cannot be created.
/// A bounded retry keeps a collision with a concurrent creator from reading as
/// a failure.
pub fn create_owned_staging(parent: &Path, target_name: &OsStr) -> io::Result<OwnedStaging> {
    let mut last = None;
    for _ in 0..16 {
        let name = format!(
            "{}.creating-{}",
            Path::new(target_name).display(),
            Uuid::now_v7()
        );
        let staging = parent.join(name);
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(OwnedStaging { path: staging }),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("no unique staging path")))
}

/// Same-parent exclusive rename: fails instead of replacing a destination that
/// appeared after the preflight.
///
/// # Errors
///
/// Returns [`io::ErrorKind::AlreadyExists`] when `to` exists, and
/// [`io::ErrorKind::Unsupported`] on a target where no exclusive rename exists.
pub fn rename_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            from,
            rustix::fs::CWD,
            to,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(io::Error::from)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (from, to);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "same-parent exclusive rename is not available on this host",
        ))
    }
}

impl Git<'_> {
    /// Clones `remote` into the invocation-owned empty staging folder `target`,
    /// bounded by the caller's cancellation flag, with the caller's credential
    /// environment applied after isolation so the credential wins.
    ///
    /// The caller passes [`OwnedStaging::path`], and renames the staging folder
    /// into place with [`rename_exclusive`] after its own branch and validation
    /// steps.
    ///
    /// # Errors
    ///
    /// Returns a [`CloneFailure`] when the operator cancels, the path cannot be
    /// used, Git cannot start or be awaited, or Git refuses the clone.
    pub fn clone_into(
        &self,
        remote: &str,
        target: &Path,
        cancel: &AtomicBool,
        credential: Option<&GitCredential>,
    ) -> Result<(), CloneFailure> {
        if cancel.load(Ordering::SeqCst) {
            return Err(CloneFailure::Cancelled);
        }
        let Some(target_str) = target.to_str() else {
            return Err(CloneFailure::PathNotUsable);
        };
        let (mut command, _askpass) = self
            .clone_command(
                &["clone", "--progress", "--", remote, target_str],
                remote,
                credential,
            )
            .map_err(|error| CloneFailure::NotStarted {
                detail: redact_git_detail(self.redactor(), &error.to_string(), None),
            })?;
        if cancel.load(Ordering::SeqCst) {
            return Err(CloneFailure::Cancelled);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| CloneFailure::NotStarted {
            detail: redact_git_detail(self.redactor(), &error.to_string(), None),
        })?;
        let (sender, receiver) = mpsc::channel();
        let mut stderr_handle = child.stderr.take().map(|pipe| {
            thread::spawn(move || {
                let _ = sender.send(read_capped(pipe, MAX_CAPTURED_STDERR_BYTES));
            })
        });
        // The exit is observed without collecting the child, so its group id
        // stays reserved for the cancellation signal below.
        loop {
            if cancel.load(Ordering::SeqCst) {
                kill_child_tree(&mut child);
                let _ = stderr_handle.take().map(thread::JoinHandle::join);
                return Err(CloneFailure::Cancelled);
            }
            match exited_uncollected(&mut child, false) {
                Ok(true) => break,
                Ok(false) => thread::sleep(POLL),
                Err(error) => {
                    kill_child_tree(&mut child);
                    let _ = stderr_handle.take().map(thread::JoinHandle::join);
                    return Err(CloneFailure::WaitFailed {
                        detail: redact_git_detail(self.redactor(), &error.to_string(), None),
                    });
                }
            }
        }
        // Git has exited, but a remote helper or another descendant can still
        // hold standard error. The drain stays bounded by the cancellation
        // flag; a cancel ends the whole group and reaps the child.
        let (stderr, overflowed) = match stderr_handle.take() {
            None => (Vec::new(), false),
            Some(reader) => loop {
                match receiver.recv_timeout(POLL) {
                    Ok(captured) => {
                        let _ = reader.join();
                        break captured.unwrap_or_default();
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        let _ = reader.join();
                        break (Vec::new(), false);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if cancel.load(Ordering::SeqCst) {
                            // Ends the group while the child is still waitable,
                            // then reaps it; the drain is over either way.
                            kill_child_tree(&mut child);
                            let _ = reader.join();
                            return Err(CloneFailure::Cancelled);
                        }
                    }
                }
            },
        };
        // Reaped last, after the drain: the group id was reserved until now.
        let status = child.wait().map_err(|error| CloneFailure::WaitFailed {
            detail: redact_git_detail(self.redactor(), &error.to_string(), None),
        })?;
        if status.success() {
            return Ok(());
        }
        Err(CloneFailure::Refused {
            detail: stderr_detail(
                self.redactor(),
                &stderr,
                overflowed,
                credential.map(|credential| credential.secret().expose_secret()),
            ),
        })
    }
}

/// How often a clone checks its child and the cancellation flag.
const POLL: Duration = Duration::from_millis(40);
