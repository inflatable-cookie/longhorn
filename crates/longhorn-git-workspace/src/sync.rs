//! Generic Legacy Sync mechanism.
//!
//! Clean, upstream-only, fetch-then-compare, `--ff-only` forward, ahead-only
//! push. No rebase, force, stash, reset, clean or branch picker exists here.
//! Which workspace root is admitted, and what the application calls this
//! action, stay with the application.

use std::path::Path;

use crate::{
    execution::Git,
    failure::{GitFailure, GitFailureCode},
    model::GitWorkspaceState,
    status::has_tracked_dirt,
};

/// Why a sync stopped before completing. Facts only; the application owns the
/// sentence and the outcome projection for each one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncStop {
    /// The checkout is detached.
    Detached,
    /// The worktree holds tracked, staged or conflicted work.
    Dirty,
    /// The current branch tracks no upstream.
    NoUpstream,
    /// The fetch brought in changes that now sit beside local work.
    DirtyAfterFetch,
    /// Local and remote history diverged after the fetch.
    DivergedAfterFetch,
    /// The worktree became dirty before the push.
    DirtyBeforePush,
    /// Local and remote history diverged before the push.
    DivergedBeforePush,
}

/// One sync reading. Each variant carries the Git facts; the application turns
/// them into its own outcome and copy.
#[derive(Debug)]
pub enum SyncReading {
    /// The sync stopped at the named step.
    Stopped {
        /// Why it stopped.
        stop: SyncStop,
        /// The worktree as last read.
        state: GitWorkspaceState,
        /// Whether the fetch ran.
        fetched: bool,
        /// Whether the fast-forward ran.
        fast_forwarded: bool,
    },
    /// Every step ran.
    Completed {
        /// The re-read worktree.
        state: GitWorkspaceState,
        /// Whether the fetch ran.
        fetched: bool,
        /// Whether the workspace fast-forwarded.
        fast_forwarded: bool,
        /// Whether the branch was pushed.
        pushed: bool,
    },
}

impl Git<'_> {
    /// Syncs one upstream-tracking worktree without tracked, staged or
    /// conflicted work: fetch, fast-forward when behind, push when ahead.
    /// Untracked content does not stop it.
    ///
    /// `reload` re-reads the same workspace through the caller's own admission
    /// path after each mutation, so an application that admits its roots keeps
    /// owning that admission.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::FetchFailed`], [`GitFailureCode::FastForwardFailed`]
    /// or [`GitFailureCode::PushFailed`] when Git refuses that step.
    pub fn sync_workspace(
        &self,
        before: GitWorkspaceState,
        reload: impl Fn(&Path) -> Result<GitWorkspaceState, GitFailure>,
    ) -> Result<SyncReading, GitFailure> {
        if before.detached {
            return Ok(stopped(SyncStop::Detached, before, false, false));
        }
        if has_tracked_dirt(&before.status) {
            return Ok(stopped(SyncStop::Dirty, before, false, false));
        }
        if before.upstream.is_none() {
            return Ok(stopped(SyncStop::NoUpstream, before, false, false));
        }

        let root = before.root.as_path();
        self.run_git(root, &["fetch"], GitFailureCode::FetchFailed)?;
        let after_fetch = reload(root)?;
        if has_tracked_dirt(&after_fetch.status) {
            return Ok(stopped(SyncStop::DirtyAfterFetch, after_fetch, true, false));
        }
        if after_fetch.status.ahead > 0 && after_fetch.status.behind > 0 {
            return Ok(stopped(
                SyncStop::DivergedAfterFetch,
                after_fetch,
                true,
                false,
            ));
        }

        let mut fast_forwarded = false;
        if after_fetch.status.behind > 0 {
            self.run_git(
                root,
                &["merge", "--ff-only", "@{upstream}"],
                GitFailureCode::FastForwardFailed,
            )?;
            fast_forwarded = true;
        }

        let before_push = reload(root)?;
        if has_tracked_dirt(&before_push.status) {
            return Ok(stopped(
                SyncStop::DirtyBeforePush,
                before_push,
                true,
                fast_forwarded,
            ));
        }
        if before_push.status.ahead > 0 && before_push.status.behind > 0 {
            return Ok(stopped(
                SyncStop::DivergedBeforePush,
                before_push,
                true,
                fast_forwarded,
            ));
        }

        let mut pushed = false;
        if before_push.status.ahead > 0 {
            self.run_git(root, &["push"], GitFailureCode::PushFailed)?;
            pushed = true;
        }
        let state = reload(root)?;

        Ok(SyncReading::Completed {
            state,
            fetched: true,
            fast_forwarded,
            pushed,
        })
    }
}

fn stopped(
    stop: SyncStop,
    state: GitWorkspaceState,
    fetched: bool,
    fast_forwarded: bool,
) -> SyncReading {
    SyncReading::Stopped {
        stop,
        state,
        fetched,
        fast_forwarded,
    }
}
