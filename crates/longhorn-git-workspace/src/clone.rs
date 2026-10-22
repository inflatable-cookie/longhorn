//! Generic clone into an invocation-owned staging folder.
//!
//! An invocation owns one unique staging directory under the destination's
//! parent; the requested target stays absent until a same-parent exclusive
//! rename. Nothing here decides which remote is allowed, what a workspace is,
//! or what a failure says: the application supplies the remote, its credential
//! and the copy.
//!
//! The composition rule is the application's to keep: create the staging
//! folder beside the target, clone into it, remove it on every failure, and
//! move it into place only with [`rename_exclusive`]. A refused clone then
//! leaves no partial target.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use uuid::Uuid;

use crate::{
    execution::{Git, isolate_process_group, kill_child_tree, read_tail, redact_git_detail},
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
pub fn create_owned_staging(parent: &Path, target_name: &OsStr) -> io::Result<PathBuf> {
    let mut last = None;
    for _ in 0..16 {
        let name = format!(
            "{}.creating-{}",
            Path::new(target_name).display(),
            Uuid::now_v7()
        );
        let staging = parent.join(name);
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
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

/// Removes only a staging path this invocation created.
pub fn remove_owned_staging(staging: &Path) {
    let _ = fs::remove_dir_all(staging);
}

impl Git<'_> {
    /// Clones `remote` into the invocation-owned empty staging folder `target`,
    /// bounded by the caller's cancellation flag, with the caller's credential
    /// environment applied after isolation so the credential wins.
    ///
    /// The caller renames the staging folder into place with
    /// [`rename_exclusive`] after its own branch and validation steps.
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
        let mut command = Command::new("git");
        command
            .args(["clone", "--progress", "--", remote, target_str])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        self.isolate(&mut command);
        isolate_process_group(&mut command);
        if let Some(credential) = credential {
            for (key, value) in credential.env_vars() {
                command.env(key, value);
            }
        }
        let mut child = command.spawn().map_err(|error| CloneFailure::NotStarted {
            detail: redact_git_detail(self.redactor(), &error.to_string(), None),
        })?;
        let mut stderr_handle = child
            .stderr
            .take()
            .map(|pipe| thread::spawn(move || read_tail(pipe)));
        loop {
            if cancel.load(Ordering::SeqCst) {
                stop(&mut child, stderr_handle.take());
                return Err(CloneFailure::Cancelled);
            }
            match child.try_wait() {
                Ok(Some(status)) if status.success() => {
                    let _ = stderr_handle.take().map(join);
                    return Ok(());
                }
                Ok(Some(_)) => {
                    let stderr = stderr_handle.take().map(join).unwrap_or_default();
                    return Err(CloneFailure::Refused {
                        detail: redact_git_detail(
                            self.redactor(),
                            &String::from_utf8_lossy(&stderr),
                            credential.map(GitCredential::secret),
                        ),
                    });
                }
                Ok(None) => thread::sleep(Duration::from_millis(40)),
                Err(error) => {
                    stop(&mut child, stderr_handle.take());
                    return Err(CloneFailure::WaitFailed {
                        detail: redact_git_detail(self.redactor(), &error.to_string(), None),
                    });
                }
            }
        }
    }
}

fn stop(child: &mut std::process::Child, stderr: Option<StderrReader>) {
    kill_child_tree(child);
    if let Some(reader) = stderr {
        let _ = join(reader);
    }
}

type StderrReader = thread::JoinHandle<io::Result<(Vec<u8>, bool)>>;

fn join(reader: StderrReader) -> Vec<u8> {
    reader
        .join()
        .ok()
        .and_then(Result::ok)
        .map(|(bytes, _truncated)| bytes)
        .unwrap_or_default()
}
