//! Generic remote access classification and preflight.
//!
//! One classification of remote refusals, shared by the preflight and every
//! remote call that reports one. It reads remote availability only. The
//! application credential is supplied explicitly as a [`GitCredential`]; the
//! mechanism never writes, prompts, stores or discovers one, and ambient
//! credential helpers are never consulted — not as a first choice and not as a
//! fallback.

use crate::{execution::Git, failure::GitFailureCode, policy::GitCredential};

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

/// GitHub over the HTTP and HTTPS client routes is the one that needs a stored
/// credential; the donor treats both spellings alike. SSH remotes carry their
/// own key material and are preflighted by reachability alone.
#[must_use]
pub fn is_https_github(remote: &str) -> bool {
    let lower = remote.trim().to_ascii_lowercase();
    lower.starts_with("https://github.com/") || lower.starts_with("http://github.com/")
}

/// True when a credential was supplied for the operation.
///
/// The mechanism reads the caller's decision, not the machine's Git
/// configuration: a host-configured helper is never part of this reading.
#[must_use]
pub fn credential_bridge_configured(credential: Option<&GitCredential>) -> bool {
    credential.is_some()
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
    if has("authentication failed")
        || has("invalid username or password")
        || has("permission denied")
        || has("host key verification failed")
        || has("access denied")
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
    /// Ambient credential helpers are never consulted, so what the machine has
    /// configured cannot change this reading.
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
        if is_https_github(remote) && !credential_bridge_configured(credential) {
            return Err(RemoteAccessFailure::CredentialHelperMissing);
        }
        match self.run_git_unbound(
            &["ls-remote", "--heads", "--", remote],
            GitFailureCode::RemotePreflightFailed,
            credential,
        ) {
            Ok(_) => Ok(()),
            Err(failure) => Err(classify_remote_failure(failure.detail())),
        }
    }
}
