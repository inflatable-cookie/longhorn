//! Git mechanism failures: machine facts, Git's own words and wire codes.
//!
//! The mechanism authors no operator-facing sentence. An application maps
//! these facts to its own copy at its own boundary.

use std::{fmt, time::Duration};

/// The wire code one mechanism failure carries.
///
/// The strings are the donor's wire vocabulary, so an application that already
/// classifies those codes keeps its classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitFailureCode {
    /// A worktree status read failed.
    StatusFailed,
    /// A `fetch` failed.
    FetchFailed,
    /// A `push` failed.
    PushFailed,
    /// The fast-forward merge in Legacy Sync failed.
    FastForwardFailed,
    /// An explicit branch switch failed.
    SwitchFailed,
    /// The non-mutating write-capability probe failed.
    PushCapability,
    /// The requested branch is not among the observed refs.
    BranchNotFound,
    /// The folder is not a readable Git repository.
    RepositoryUnavailable,
    /// The current branch or revision could not be read.
    BranchUnavailable,
    /// `origin` could not be read.
    OriginUnavailable,
    /// Local or remote branch discovery failed.
    BranchDiscoveryFailed,
    /// The remote access preflight failed.
    RemotePreflightFailed,
    /// The Git process could not be started or awaited.
    CommandUnavailable,
    /// Git's standard output exceeded the crate's capture bound.
    OutputTooLarge,
}

impl GitFailureCode {
    /// The stable wire string for this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StatusFailed => "git.status_failed",
            Self::FetchFailed => "git.fetch_failed",
            Self::PushFailed => "git.push_failed",
            Self::FastForwardFailed => "git.fast_forward_failed",
            Self::SwitchFailed => "git.switch_failed",
            Self::PushCapability => "git.push_capability",
            Self::BranchNotFound => "git.branch_not_found",
            Self::RepositoryUnavailable => "git.repository_unavailable",
            Self::BranchUnavailable => "git.branch_unavailable",
            Self::OriginUnavailable => "git.origin_unavailable",
            Self::BranchDiscoveryFailed => "git.branch_discovery_failed",
            Self::RemotePreflightFailed => "git.remote_preflight_failed",
            Self::CommandUnavailable => "git.command_unavailable",
            Self::OutputTooLarge => "git.output_too_large",
        }
    }
}

impl fmt::Display for GitFailureCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One classified Git mechanism failure.
///
/// Every variant carries the facts a caller needs to decide its own outcome.
/// Details are Git's own words, already redacted and bounded by the time a
/// reading returns them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitFailure {
    /// Git ran and refused. `detail` is Git's own redacted output, possibly
    /// empty when Git reported nothing.
    Refused {
        /// The caller-supplied classification code.
        code: GitFailureCode,
        /// Git's own redacted, bounded words.
        detail: String,
    },
    /// The bounded call reached its wall clock and the child was killed.
    TimedOut {
        /// The caller-supplied classification code.
        code: GitFailureCode,
        /// The bound the call reached.
        after: Duration,
    },
    /// Git could not be started, awaited, or its answer could not be trusted.
    Unavailable {
        /// The classification code.
        code: GitFailureCode,
        /// The mechanism's own bounded detail.
        detail: String,
    },
    /// The folder is not a Git worktree.
    NotWorktree {
        /// The classification code.
        code: GitFailureCode,
    },
    /// The folder could not be resolved.
    FolderUnavailable {
        /// The classification code.
        code: GitFailureCode,
        /// The operating system's words.
        detail: String,
    },
    /// The requested branch is not among the observed refs.
    BranchMissing {
        /// The classification code.
        code: GitFailureCode,
    },
}

impl GitFailure {
    /// The failure's wire code.
    #[must_use]
    pub const fn code(&self) -> GitFailureCode {
        match self {
            Self::Refused { code, .. }
            | Self::TimedOut { code, .. }
            | Self::Unavailable { code, .. }
            | Self::NotWorktree { code }
            | Self::FolderUnavailable { code, .. }
            | Self::BranchMissing { code } => *code,
        }
    }

    /// Git's own redacted words when the failure carries any. Classification
    /// input only: a person never reads this directly from the mechanism.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Refused { detail, .. }
            | Self::Unavailable { detail, .. }
            | Self::FolderUnavailable { detail, .. } => detail,
            _ => "",
        }
    }
}

impl fmt::Display for GitFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.code())?;
        match self {
            Self::TimedOut { after, .. } => write!(formatter, ": timed out after {after:?}"),
            Self::NotWorktree { .. } | Self::BranchMissing { .. } => Ok(()),
            _ => {
                if self.detail().is_empty() {
                    Ok(())
                } else {
                    write!(formatter, ": {}", self.detail())
                }
            }
        }
    }
}

impl std::error::Error for GitFailure {}
