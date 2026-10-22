//! Generic workspace-attach reading over explicit expected facts.
//!
//! The reading compares a folder against the expected remote and the expected
//! branch the application supplies. Which remote and branch those are, and
//! every word of the resulting refusal, stay with the application; the reading
//! never classifies Git facts into product copy and never invents a second
//! cleanliness notion.

use std::path::Path;

use crate::{
    execution::Git, failure::GitFailure, model::GitWorkspaceState,
    workspace::has_switch_blocking_dirt,
};

/// One attach reading. Each variant carries the Git facts; the application
/// turns them into its own projection and copy.
#[derive(Debug)]
pub enum AttachReading {
    /// The checkout is on the expected branch.
    Ready {
        /// The canonical worktree root.
        root: String,
    },
    /// Another branch is current and may be switched onto the expected one.
    SwitchOffered {
        /// The canonical worktree root.
        root: String,
        /// The branch that is current now.
        current_branch: String,
        /// Whether the checkout is detached.
        detached: bool,
        /// True when the expected branch was not observed, so it has to be
        /// created from the application's integration base.
        create_from_integration_base: bool,
    },
    /// The folder is not a Git worktree at all.
    NotRepository {
        /// The folder as the caller named it.
        root: String,
        /// The classified failure that reading produced.
        error: GitFailure,
    },
    /// `origin` does not match the expected remote.
    WrongRemote {
        /// The canonical worktree root.
        root: String,
        /// The observed remote, safe for display and redacted with the
        /// caller's redactor, when there is one.
        observed: Option<String>,
    },
    /// Another branch holds tracked, staged or conflicted work.
    WrongBranchDirty {
        /// The canonical worktree root.
        root: String,
        /// The branch that is current now.
        current_branch: String,
        /// Whether the checkout is detached.
        detached: bool,
    },
}

impl Git<'_> {
    /// Reads one folder against the expected branch and remote.
    ///
    /// This never mutates the checkout: it switches nothing, creates nothing
    /// and stages nothing.
    #[must_use]
    pub fn read_attach(
        &self,
        root: &Path,
        expected_branch: &str,
        expected_remote: &str,
    ) -> AttachReading {
        let display_root = root.to_string_lossy().into_owned();
        let state = match self.inspect_worktree(root) {
            Ok(state) => state,
            Err(error) => {
                return AttachReading::NotRepository {
                    root: display_root,
                    error,
                };
            }
        };
        let root = state.root.clone();
        match self.origin_url_raw(Path::new(&root)) {
            Some(observed) if remotes_match(&observed, expected_remote) => {}
            observed => {
                return AttachReading::WrongRemote {
                    root,
                    observed: observed
                        .map(|remote| self.redacted_output(&remote_for_display(&remote), None)),
                };
            }
        }
        if !state.detached && state.current_branch == expected_branch {
            return AttachReading::Ready { root };
        }
        if has_switch_blocking_dirt(&state) {
            return AttachReading::WrongBranchDirty {
                root,
                current_branch: state.current_branch.clone(),
                detached: state.detached,
            };
        }
        AttachReading::SwitchOffered {
            root,
            current_branch: state.current_branch.clone(),
            detached: state.detached,
            create_from_integration_base: !has_expected_branch(&state, expected_branch),
        }
    }
}

fn has_expected_branch(state: &GitWorkspaceState, expected_branch: &str) -> bool {
    let remote = format!("origin/{expected_branch}");
    state.has_named_ref(expected_branch) || state.has_named_ref(&remote)
}

/// Whether two remote URLs name the same repository.
#[must_use]
pub fn remotes_match(observed: &str, expected: &str) -> bool {
    remote_identity(observed) == remote_identity(expected)
}

/// One remote's comparable identity: scheme, trailing slash and `.git` suffix
/// removed, SSH user removed, and the case folded.
#[must_use]
pub fn remote_identity(raw: &str) -> String {
    let mut value = raw.trim().to_owned();
    if let Some(stripped) = value.strip_suffix('/') {
        value = stripped.to_owned();
    }
    if let Some(stripped) = value
        .strip_suffix(".git")
        .or_else(|| value.strip_suffix(".GIT"))
    {
        value = stripped.to_owned();
    }
    for prefix in ["ssh://", "git://", "https://", "http://"] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.to_owned();
            break;
        }
    }
    if let Some(scheme) = value.find("://") {
        value = value[scheme + 3..].to_owned();
    }
    if let Some(at) = value.find('@')
        && !value[..at].contains('/')
    {
        value = value[at + 1..].to_owned();
    }
    value = value.replace(':', "/");
    value.to_ascii_lowercase()
}

/// A remote URL safe to show: any credential in its user-info component is
/// removed. The raw Git value is useful for comparison but never belongs in a
/// renderer projection or an onboarding diagnostic.
#[must_use]
pub fn remote_for_display(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(scheme) = trimmed.find("://") {
        let authority_start = scheme + 3;
        if let Some(at) = trimmed[authority_start..].find('@') {
            let at = authority_start + at;
            return format!("{}{}", &trimmed[..authority_start], &trimmed[at + 1..]);
        }
    }
    if let Some(at) = trimmed.find('@')
        && !trimmed[..at].contains('/')
    {
        return trimmed[at + 1..].to_owned();
    }
    trimmed.to_owned()
}
