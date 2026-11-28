//! Generic Git workspace projections.
//!
//! These are the shapes a Git workspace reading produces. They carry no
//! product decision: the keys, the types, and the fixed Git vocabulary the
//! caller's own contract pins. An application may serialize them unchanged or
//! project its own wire shape from them.

use std::path::{Path, PathBuf};

use serde::{Serialize, Serializer};

/// One worktree's Git status as Git reports it.
///
/// `clean` is computed here from the four dirt counters, so no caller
/// reconstructs cleanliness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// Paths with staged changes.
    pub(crate) staged: usize,
    /// Paths with unstaged changes to tracked content.
    pub(crate) modified: usize,
    /// Paths Git does not track.
    pub(crate) untracked: usize,
    /// Paths in a conflicted merge state.
    pub(crate) conflicted: usize,
    /// Commits the current branch has and its upstream does not.
    pub ahead: usize,
    /// Commits the upstream has and the current branch does not.
    pub behind: usize,
    /// True when no staged, modified, untracked or conflicted path exists.
    pub(crate) clean: bool,
}

impl GitStatus {
    /// Number of paths with staged changes.
    #[must_use]
    pub fn staged(&self) -> usize {
        self.staged
    }
    /// Number of paths with unstaged changes.
    #[must_use]
    pub fn modified(&self) -> usize {
        self.modified
    }
    /// Number of untracked paths.
    #[must_use]
    pub fn untracked(&self) -> usize {
        self.untracked
    }
    /// Number of conflicted paths.
    #[must_use]
    pub fn conflicted(&self) -> usize {
        self.conflicted
    }
    /// Whether all four dirt counters are zero.
    #[must_use]
    pub fn clean(&self) -> bool {
        self.clean
    }
}

/// Where one branch reference lives.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitBranchKind {
    /// A local branch under `refs/heads`.
    Local,
    /// A remote-tracking branch under `refs/remotes`.
    Remote,
}

/// One observed branch reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    /// The stable reference id: `local:<name>` or `remote:<name>`.
    pub id: String,
    /// Git's short reference name.
    pub name: String,
    /// Whether the reference is local or remote-tracking.
    pub kind: GitBranchKind,
    /// Whether this reference is the current branch.
    pub current: bool,
    /// For a remote reference with a local branch tracking it, that local
    /// branch's name. Never serialized: it is an internal switch target.
    #[serde(skip_serializing)]
    pub local_target: Option<String>,
}

/// One worktree reading: where it is, what is current, and what Git reports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorkspaceState {
    /// The canonical worktree root, exact. Git's own bytes are kept, so a
    /// later read or switch addresses the same folder. Serialized as a lossy
    /// display string.
    #[serde(serialize_with = "serialize_lossy_root")]
    pub root: PathBuf,
    /// The current branch name, or the short revision when detached.
    pub current_branch: String,
    /// True when the checkout is detached.
    pub detached: bool,
    /// The upstream reference, when the branch tracks one.
    pub upstream: Option<String>,
    /// Every local and remote-tracking branch observed.
    pub branches: Vec<GitBranch>,
    /// The status counters.
    pub status: GitStatus,
}

impl GitWorkspaceState {
    /// Whether one observed reference has this name.
    #[must_use]
    pub fn has_named_ref(&self, name: &str) -> bool {
        self.branches.iter().any(|branch| branch.name == name)
    }
}

/// The wire form of a worktree root: the same lossy display string the
/// projection has always carried, while the Rust field keeps the exact path.
fn serialize_lossy_root<S: Serializer>(root: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&root.to_string_lossy())
}
