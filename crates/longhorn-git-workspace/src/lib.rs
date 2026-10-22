//! Generic Git workspace mechanism for desktop applications.
//!
//! This crate implements Longhorn contract 029, part 5a: the Git subprocess
//! executor with its caller-supplied [`GitPolicy`] hook, worktree status and
//! branch reading, remote-refusal preflight classification, Legacy Sync, the
//! explicit branch switch, clone into an invocation-owned staging folder with
//! a same-parent exclusive rename, and attach reading over explicit expected
//! facts.
//!
//! Every reading is a method on [`Git`], which holds the caller's policy and
//! the caller's [`longhorn_redact::Redactor`]. Callers supply, as explicit
//! values, the remote, the expected branch, a [`GitCredential`] where one
//! applies, the staging parent and target, and a cancellation flag. Nothing
//! here reads a product constant, calls a product module, or writes a word an
//! operator reads: failures carry machine facts, Git's own redacted words and
//! wire codes.
//!
//! Product decisions stay outside this crate. Isolation environment and Git
//! configuration arrive through [`GitPolicy::isolate`]; a credential arrives
//! through [`GitPolicy::credential`] or as an explicit argument; the allowed
//! remote, branch naming, the integration base, contract-readable root
//! admission, and every user-facing sentence belong to the application.
//!
//! Part 5b adds the GitHub sign-in mechanism to this crate: device flow,
//! token exchange and refresh, askpass, commit identity, and credential
//! storage. It is not part of this part's API.
//!
//! The integration tests run real `git` against temporary local repositories
//! and bare remotes. No test reaches the network.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod attach;
mod clone;
mod execution;
mod failure;
mod model;
mod policy;
mod preflight;
mod status;
mod sync;
mod workspace;

pub use attach::{AttachReading, remote_for_display, remote_identity, remotes_match};
pub use clone::{
    CloneFailure, create_owned_staging, remove_owned_staging, rename_exclusive, staging_parent,
    target_already_present,
};
pub use execution::{
    FAILURE_DETAIL_BOUND, Git, MAX_CAPTURED_OUTPUT_BYTES, is_remote_credential_operation,
    redact_git_detail,
};
pub use failure::{GitFailure, GitFailureCode};
pub use model::{GitBranch, GitBranchKind, GitStatus, GitWorkspaceState};
pub use policy::{GitCredential, GitPolicy};
pub use preflight::{
    RemoteAccessFailure, classify_remote_failure, credential_bridge_configured, is_https_github,
};
pub use sync::{SyncReading, SyncStop};
pub use workspace::{SwitchReading, has_switch_blocking_dirt};
