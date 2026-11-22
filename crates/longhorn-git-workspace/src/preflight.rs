//! Generic remote access classification and preflight.
//!
//! One classification of remote refusals, shared by the preflight and every
//! remote call that reports one. It reads remote availability only. The
//! application credential is supplied explicitly as a [`GitCredential`]; the
//! mechanism never writes, prompts, stores or discovers one, and ambient
//! credential helpers are never consulted — not as a first choice and not as a
//! fallback.

use std::time::Duration;

use crate::{
    execution::{Git, https_host},
    failure::{GitFailure, GitFailureCode},
    policy::GitCredential,
};

pub(crate) const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) fn probe_failure(failure: &GitFailure) -> RemoteAccessFailure {
    match failure {
        GitFailure::TimedOut { .. } => RemoteAccessFailure::NetworkUnavailable,
        _ => classify_remote_failure(failure.detail()),
    }
}

/// What one remote access reading found.
///
/// The caller supplies the product copy for each reading; this owns the
/// classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteAccessFailure {
    /// No credential was supplied, so a GitHub remote can only fail at
    /// the prompt the application never shows.
    CredentialHelperMissing,
    /// A credential was supplied and the remote refused it.
    AuthenticationRequired,
    /// The remote host could not be reached or resolved.
    NetworkUnavailable,
    /// The host answered and the repository is absent or not readable.
    RepositoryUnavailable,
    /// Git refused for a reason none of the above explains.
    Unavailable,
}

/// True for an HTTPS URL on GitHub's standard credential endpoint.
#[must_use]
pub fn is_https_github(remote: &str) -> bool {
    https_host(remote).as_deref() == Some("github.com")
}

// Only HTTP credential rejections authorize a refresh. SSH and local
// filesystem failures can contain the same generic permission language.
pub(crate) fn is_rejected_credential(stderr: &str) -> bool {
    stderr.lines().any(|line| {
        let line = line.to_ascii_lowercase();
        line.starts_with("fatal: authentication failed for ")
            || line.starts_with("remote: invalid username or password")
            || (line.starts_with("fatal: unable to access ")
                && line.ends_with("the requested url returned error: 401"))
    })
}

/// Separates a missing credential, refused credentials, network and DNS
/// problems, and repository availability from Git's own words.
#[must_use]
pub fn classify_remote_failure(stderr: &str) -> RemoteAccessFailure {
    let lower = stderr.to_ascii_lowercase();
    let has = |needle: &str| lower.contains(needle);
    if has("could not resolve host")
        || has("could not resolve hostname")
        || has("temporary failure in name resolution")
        || has("network is unreachable")
        || has("connection refused")
        || has("connection timed out")
        || has("failed to connect")
        || has("operation timed out")
    {
        return RemoteAccessFailure::NetworkUnavailable;
    }
    if has("could not read username")
        || has("could not read password")
        || has("terminal prompts disabled")
        || has("no credential helper")
    {
        return RemoteAccessFailure::CredentialHelperMissing;
    }
    if is_rejected_credential(stderr)
        || has("permission denied (publickey")
        || has("host key verification failed")
    {
        return RemoteAccessFailure::AuthenticationRequired;
    }
    if has("repository not found")
        || has("does not appear to be a git repository")
        || has("not found")
        || has("does not exist")
    {
        return RemoteAccessFailure::RepositoryUnavailable;
    }
    RemoteAccessFailure::Unavailable
}

impl Git<'_> {
    /// Read-only reachability check for one remote, run before a clone or an
    /// attach check. It performs no repository mutation and leaves no working
    /// copy behind.
    ///
    /// A GitHub remote requires the credential the operation will use.
    /// A credential supplied here is added to the probe's environment only.
    /// The Git probe has a fifteen-second budget and uses only the supplied
    /// credential. Ambient credential helpers are disabled.
    ///
    /// # Errors
    ///
    /// Returns the classified [`RemoteAccessFailure`]. An unreachable or
    /// missing folder reads as [`RemoteAccessFailure::RepositoryUnavailable`].
    pub fn preflight_remote(
        &self,
        remote: &str,
        credential: Option<&GitCredential>,
    ) -> Result<(), RemoteAccessFailure> {
        if is_https_github(remote) && credential.is_none() {
            return Err(RemoteAccessFailure::CredentialHelperMissing);
        }
        match self.run_git_unbound(
            &["ls-remote", "--heads", "--", remote],
            remote,
            GitFailureCode::RemotePreflightFailed,
            credential,
        ) {
            Ok(_) => Ok(()),
            Err(failure) => Err(probe_failure(&failure)),
        }
    }
}
