//! Generic worktree reading: branch discovery, status, remote reading, the
//! write-capability probe and the explicit branch switch.
//!
//! Root admission, branch naming and every sentence a person reads stay with
//! the application. The readings here carry facts and wire codes only.

use std::{collections::HashMap, path::Path};

use crate::{
    execution::Git,
    failure::{GitFailure, GitFailureCode},
    model::{GitBranch, GitBranchKind, GitWorkspaceState},
    preflight::{RemoteAccessFailure, classify_remote_failure},
};

/// Tracked, staged or conflicted work blocks a branch switch. Untracked
/// application content is in-progress work, not dirt.
#[must_use]
pub fn has_switch_blocking_dirt(state: &GitWorkspaceState) -> bool {
    state.status.staged + state.status.modified + state.status.conflicted > 0
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
    /// Reads one folder as a Git worktree: its canonical root, current branch,
    /// detached state, upstream, every observed branch and its status.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::RepositoryUnavailable`] when the folder cannot
    /// be resolved or is not a worktree, and the branch or status codes when a
    /// later read refuses.
    pub fn inspect_worktree(&self, root: &Path) -> Result<GitWorkspaceState, GitFailure> {
        let root = root
            .canonicalize()
            .map_err(|error| GitFailure::FolderUnavailable {
                code: GitFailureCode::RepositoryUnavailable,
                detail: error.to_string(),
            })?;
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
            root: root.to_string_lossy().into_owned(),
            current_branch,
            detached,
            upstream,
            branches,
            status,
        })
    }

    /// The `origin` remote URL of one worktree, or `None` when Git refuses.
    #[must_use]
    pub fn origin_url(&self, root: &Path) -> Option<String> {
        self.run_git(
            root,
            &["remote", "get-url", "origin"],
            GitFailureCode::OriginUnavailable,
        )
        .ok()
        .filter(|url| !url.is_empty())
    }

    /// A bounded, non-mutating write-capability probe.
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
        match self.run_git(
            root,
            &["push", "--dry-run", "origin", "HEAD"],
            GitFailureCode::PushCapability,
        ) {
            Ok(_) => Ok(()),
            Err(error) => Err(classify_remote_failure(error.detail())),
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
        reload: impl Fn(&str) -> Result<GitWorkspaceState, GitFailure>,
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
        self.run_git(Path::new(&root), &arguments, GitFailureCode::SwitchFailed)?;
        let state = reload(&root)?;
        Ok(SwitchReading::Switched { state })
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
