//! Generic worktree reading: worktree identity, branch discovery, status,
//! remote reading, the write-capability probe and the explicit branch switch.
//!
//! Root admission, branch naming and every sentence a person reads stay with
//! the application. The readings here carry facts and wire codes only.

use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use crate::{
    execution::{Git, redact_git_detail},
    failure::{GitFailure, GitFailureCode},
    model::{GitBranch, GitBranchKind, GitWorkspaceState},
    preflight::{REMOTE_PROBE_TIMEOUT, RemoteAccessFailure, probe_failure},
    status::has_tracked_dirt,
};

/// Unix filesystem identity (device and inode) for an exact directory.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FilesystemIdentity {
    /// Device identifier (`st_dev`).
    pub device: u64,
    /// Inode number (`st_ino`).
    pub inode: u64,
}

/// Exact, unredacted identity of a checked-out Git worktree and its repository.
///
/// The carried values are structured machine identifiers, not prose, and are
/// exposed only through explicit accessors. [`fmt::Debug`] stays opaque so
/// ordinary diagnostic formatting prints no path, branch name, or object ID.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct WorktreeIdentity {
    root: PathBuf,
    common_dir: PathBuf,
    root_filesystem_id: Option<FilesystemIdentity>,
    common_dir_filesystem_id: Option<FilesystemIdentity>,
    branch: Option<String>,
    head_oid: String,
}

impl WorktreeIdentity {
    /// Canonical worktree root path, exact and unredacted.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical Git common directory (`--git-common-dir`), exact and
    /// unredacted.
    #[must_use]
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Unix filesystem identity (device and inode) for [`Self::root`], or
    /// `None` on non-Unix hosts.
    #[must_use]
    pub fn root_filesystem_id(&self) -> Option<FilesystemIdentity> {
        self.root_filesystem_id
    }

    /// Unix filesystem identity (device and inode) for [`Self::common_dir`],
    /// or `None` on non-Unix hosts.
    #[must_use]
    pub fn common_dir_filesystem_id(&self) -> Option<FilesystemIdentity> {
        self.common_dir_filesystem_id
    }

    /// Exact current branch short name, or `None` when `HEAD` is detached.
    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Full `HEAD` object ID.
    #[must_use]
    pub fn head_oid(&self) -> &str {
        &self.head_oid
    }

    /// Whether `HEAD` is detached.
    #[must_use]
    pub fn is_detached(&self) -> bool {
        self.branch.is_none()
    }

    /// Whether `self` and `other` belong to the same Git repository, compared on
    /// exact canonical `common_dir` paths and filesystem identity.
    #[must_use]
    pub fn same_repository(&self, other: &Self) -> bool {
        same_location(
            &self.common_dir,
            self.common_dir_filesystem_id,
            &other.common_dir,
            other.common_dir_filesystem_id,
        )
    }

    /// Whether `self` and `other` identify the same checked-out worktree in the
    /// same repository, compared on exact canonical paths and filesystem
    /// identity.
    #[must_use]
    pub fn same_worktree(&self, other: &Self) -> bool {
        self.same_repository(other)
            && same_location(
                &self.root,
                self.root_filesystem_id,
                &other.root,
                other.root_filesystem_id,
            )
    }
}

impl fmt::Debug for WorktreeIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("WorktreeIdentity");
        if let Some(id) = self.root_filesystem_id {
            debug.field("root_filesystem_id", &id);
        }
        if let Some(id) = self.common_dir_filesystem_id {
            debug.field("common_dir_filesystem_id", &id);
        }
        debug
            .field("detached", &self.is_detached())
            .finish_non_exhaustive()
    }
}

/// Builds a path from Git's exact output bytes, with no trim and no lossy
/// decode, so two byte-distinct locations can never collapse into one string.
#[cfg(unix)]
fn path_from_exact_bytes(bytes: Vec<u8>) -> Result<PathBuf, GitFailure> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;
    Ok(PathBuf::from(OsStr::from_bytes(&bytes).to_owned()))
}

/// Builds a path from Git's exact output bytes on a host without byte-level
/// path access. Git paths that are not valid UTF-8 are refused instead of
/// decoded lossily, so two byte-distinct locations can never collapse into
/// one string.
#[cfg(not(unix))]
fn path_from_exact_bytes(bytes: Vec<u8>) -> Result<PathBuf, GitFailure> {
    String::from_utf8(bytes)
        .map(PathBuf::from)
        .map_err(|_| GitFailure::Unavailable {
            code: GitFailureCode::RepositoryUnavailable,
            detail: "Git returned a path that is not valid UTF-8.".to_owned(),
        })
}

fn same_location(
    left_path: &Path,
    left_fs: Option<FilesystemIdentity>,
    right_path: &Path,
    right_fs: Option<FilesystemIdentity>,
) -> bool {
    if left_path != right_path {
        return false;
    }
    match (left_fs, right_fs) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

/// Tracked, staged or conflicted work blocks a branch switch. Untracked
/// application content is in-progress work, not dirt.
#[must_use]
pub fn has_switch_blocking_dirt(state: &GitWorkspaceState) -> bool {
    has_tracked_dirt(&state.status)
}

/// What the explicit switch did. The application turns this into its own
/// projection and copy.
#[derive(Debug)]
pub enum SwitchReading {
    /// The requested branch is already current, so nothing ran.
    AlreadyCurrent {
        /// The branch name the reading reports.
        branch: String,
        /// The unchanged worktree reading.
        state: GitWorkspaceState,
    },
    /// Tracked, staged or conflicted work blocks the switch.
    Blocked {
        /// The unchanged worktree reading.
        state: GitWorkspaceState,
    },
    /// Git switched and the workspace was re-read through the caller's path.
    Switched {
        /// The re-read worktree.
        state: GitWorkspaceState,
    },
}

impl Git<'_> {
    /// Reads one folder's exact, unredacted worktree and repository identity.
    ///
    /// Resolves the canonical worktree root and canonical Git common directory
    /// (`--git-common-dir`) from Git's exact output bytes — only Git's single
    /// trailing record terminator is removed, so a path ending in whitespace
    /// or holding non-UTF-8 bytes keeps its exact bytes — records Unix
    /// device/inode filesystem identity for both, and reads the exact branch
    /// short name (`None` when detached) and full `HEAD` object ID.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::RepositoryUnavailable`] when the folder cannot
    /// be resolved or is not inside a Git worktree, and
    /// [`GitFailureCode::BranchUnavailable`] when `HEAD` cannot be resolved
    /// (for example on an unborn branch).
    pub fn worktree_identity(&self, root: &Path) -> Result<WorktreeIdentity, GitFailure> {
        let requested = self.canonicalize_directory(root)?;
        self.ensure_git_repository(&requested)?;
        let toplevel = path_from_exact_bytes(self.run_git_bytes(
            &requested,
            &["rev-parse", "--path-format=absolute", "--show-toplevel"],
            GitFailureCode::RepositoryUnavailable,
        )?)?;
        let worktree_root = self.canonicalize_directory(&toplevel)?;
        let common = path_from_exact_bytes(self.run_git_bytes(
            &worktree_root,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            GitFailureCode::RepositoryUnavailable,
        )?)?;
        let common_dir = self.canonicalize_directory(&common)?;
        let root_filesystem_id = self.filesystem_identity(&worktree_root)?;
        let common_dir_filesystem_id = self.filesystem_identity(&common_dir)?;
        let branch = self
            .run_git_probe(
                &worktree_root,
                &["symbolic-ref", "--quiet", "--short", "HEAD"],
            )?
            .filter(|value| !value.is_empty());
        let head_oid = self.run_git(
            &worktree_root,
            &["rev-parse", "--verify", "HEAD"],
            GitFailureCode::BranchUnavailable,
        )?;
        if head_oid.is_empty() {
            return Err(GitFailure::Unavailable {
                code: GitFailureCode::BranchUnavailable,
                detail: "Git returned an empty HEAD object ID.".to_owned(),
            });
        }
        Ok(WorktreeIdentity {
            root: worktree_root,
            common_dir,
            root_filesystem_id,
            common_dir_filesystem_id,
            branch,
            head_oid,
        })
    }

    /// Reads one folder as a Git worktree: its canonical root, current branch,
    /// detached state, upstream, every observed branch and its status.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::RepositoryUnavailable`] when the folder cannot
    /// be resolved or is not a worktree, and the branch or status codes when a
    /// later read refuses.
    pub fn inspect_worktree(&self, root: &Path) -> Result<GitWorkspaceState, GitFailure> {
        let root = self.canonicalize_directory(root)?;
        self.ensure_git_repository(&root)?;
        let (current_branch, detached) = self.current_branch(&root)?;
        let upstream = if detached {
            None
        } else {
            self.upstream(&root)?
        };
        let branches = self.branches(&root, &current_branch, detached)?;
        let status = self.status(&root)?;
        Ok(GitWorkspaceState {
            root,
            current_branch,
            detached,
            upstream,
            branches,
            status,
        })
    }

    /// The `origin` remote URL of one worktree, redacted with the caller's
    /// redactor, or `None` when Git refuses.
    ///
    /// Git reports the configured URL verbatim, so it can carry a credential in
    /// its user-info component. The redaction keeps that value out of anything
    /// a caller may display or log. [`crate::remote_identity`] still compares
    /// two such values, because it removes the user-info component before
    /// comparing.
    #[must_use]
    pub fn origin_url(&self, root: &Path) -> Option<String> {
        let raw = self.origin_url_raw(root)?;
        Some(self.redacted_output(&raw, None))
    }

    /// The exact `origin` value. Internal: the credential decision and the
    /// attach comparison need the value Git reported, not a redacted form.
    pub(crate) fn origin_url_raw(&self, root: &Path) -> Option<String> {
        self.run_git(
            root,
            &["remote", "get-url", "origin"],
            GitFailureCode::OriginUnavailable,
        )
        .ok()
        .filter(|url| !url.is_empty())
    }

    /// A non-mutating write-capability probe with a fifteen-second Git budget.
    ///
    /// `--dry-run` contacts the remote and resolves the ref without creating,
    /// updating or deleting anything on either side. A failure is classified
    /// with the same vocabulary as the clone preflight, so a readiness check
    /// never invents a cause.
    ///
    /// # Errors
    ///
    /// Returns the classified [`RemoteAccessFailure`].
    pub fn push_capability(&self, root: &Path) -> Result<(), RemoteAccessFailure> {
        match self.run(
            root,
            &["push", "--dry-run", "origin", "HEAD"],
            GitFailureCode::PushCapability,
            REMOTE_PROBE_TIMEOUT,
        ) {
            Ok(_) => Ok(()),
            Err(error) => Err(probe_failure(&error)),
        }
    }

    /// Switches onto one branch already observed in `before`.
    ///
    /// `reload` re-reads the same workspace through the caller's own admission
    /// path, so an application that admits its roots keeps owning that
    /// admission for every read inside one switch.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::BranchNotFound`] when the id is not among the
    /// observed branches, and [`GitFailureCode::SwitchFailed`] when Git
    /// refuses the switch.
    pub fn switch_branch(
        &self,
        before: GitWorkspaceState,
        branch_id: &str,
        reload: impl Fn(&Path) -> Result<GitWorkspaceState, GitFailure>,
    ) -> Result<SwitchReading, GitFailure> {
        let Some(branch) = before.branches.iter().find(|branch| branch.id == branch_id) else {
            return Err(GitFailure::BranchMissing {
                code: GitFailureCode::BranchNotFound,
            });
        };
        let current = branch.current;
        let name = branch.name.clone();
        let kind = branch.kind;
        let local_target = branch.local_target.clone();
        let root = before.root.clone();

        if current {
            return Ok(SwitchReading::AlreadyCurrent {
                branch: name,
                state: before,
            });
        }
        if has_switch_blocking_dirt(&before) {
            return Ok(SwitchReading::Blocked { state: before });
        }

        let arguments = match (kind, local_target.as_deref()) {
            (GitBranchKind::Remote, Some(local_target)) => vec!["switch", "--", local_target],
            (GitBranchKind::Local, _) => vec!["switch", "--", name.as_str()],
            (GitBranchKind::Remote, None) => vec!["switch", "--track", "--", name.as_str()],
        };
        self.run_git(&root, &arguments, GitFailureCode::SwitchFailed)?;
        let state = reload(&root)?;
        Ok(SwitchReading::Switched { state })
    }

    fn canonicalize_directory(&self, path: &Path) -> Result<PathBuf, GitFailure> {
        path.canonicalize()
            .map_err(|error| GitFailure::FolderUnavailable {
                code: GitFailureCode::RepositoryUnavailable,
                detail: redact_git_detail(self.redactor(), &error.to_string(), None),
            })
    }

    fn filesystem_identity(&self, path: &Path) -> Result<Option<FilesystemIdentity>, GitFailure> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            let metadata = fs::metadata(path).map_err(|error| GitFailure::FolderUnavailable {
                code: GitFailureCode::RepositoryUnavailable,
                detail: redact_git_detail(self.redactor(), &error.to_string(), None),
            })?;
            Ok(Some(FilesystemIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            }))
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Ok(None)
        }
    }

    fn ensure_git_repository(&self, root: &Path) -> Result<(), GitFailure> {
        let value = self.run_git(
            root,
            &["rev-parse", "--is-inside-work-tree"],
            GitFailureCode::RepositoryUnavailable,
        )?;
        if value.trim() == "true" {
            Ok(())
        } else {
            Err(GitFailure::NotWorktree {
                code: GitFailureCode::RepositoryUnavailable,
            })
        }
    }

    fn current_branch(&self, root: &Path) -> Result<(String, bool), GitFailure> {
        if let Some(symbolic) =
            self.run_git_probe(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?
        {
            return Ok((symbolic, false));
        }
        let revision = self.run_git(
            root,
            &["rev-parse", "--short", "HEAD"],
            GitFailureCode::BranchUnavailable,
        )?;
        // A detached checkout reads as its short revision. How that is
        // labelled for an operator is application copy.
        Ok((revision.trim().to_owned(), true))
    }

    fn upstream(&self, root: &Path) -> Result<Option<String>, GitFailure> {
        Ok(self
            .run_git_probe(
                root,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ],
            )?
            .filter(|value| !value.is_empty()))
    }

    fn branches(
        &self,
        root: &Path,
        current_branch: &str,
        detached: bool,
    ) -> Result<Vec<GitBranch>, GitFailure> {
        let mut branches = Vec::new();
        let mut tracking_locals = HashMap::new();
        for row in lines(self.run_git(
            root,
            &[
                "for-each-ref",
                "--format=%(refname:short)%09%(upstream:short)",
                "refs/heads",
            ],
            GitFailureCode::BranchDiscoveryFailed,
        )?) {
            let (name, upstream) = row.split_once('\t').unwrap_or((&row, ""));
            if !upstream.is_empty() {
                tracking_locals.insert(upstream.to_owned(), name.to_owned());
            }
            branches.push(GitBranch {
                id: format!("local:{name}"),
                current: !detached && name == current_branch,
                name: name.to_owned(),
                kind: GitBranchKind::Local,
                local_target: None,
            });
        }
        for row in lines(self.run_git(
            root,
            &[
                "for-each-ref",
                "--format=%(refname:short)%09%(symref)",
                "refs/remotes",
            ],
            GitFailureCode::BranchDiscoveryFailed,
        )?) {
            let (name, symbolic_target) = row.split_once('\t').unwrap_or((&row, ""));
            if !symbolic_target.is_empty() || name.ends_with("/HEAD") {
                continue;
            }
            branches.push(GitBranch {
                id: format!("remote:{name}"),
                name: name.to_owned(),
                kind: GitBranchKind::Remote,
                current: false,
                local_target: tracking_locals.get(name).cloned(),
            });
        }
        Ok(branches)
    }
}

fn lines(source: String) -> impl Iterator<Item = String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .into_iter()
}
