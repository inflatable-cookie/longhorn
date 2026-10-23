//! The caller's policy for Git mechanism invocations.
//!
//! The policy is an ordinary caller-owned value. It supplies process
//! isolation, the credential for a remote it considers credential-scoped, and
//! a replacement for a credential the remote rejected. The mechanism never
//! fetches policy from an application module and never falls back to ambient
//! Git configuration or an interactive prompt.

use std::{fmt, process::Command};

use secrecy::{ExposeSecret, SecretString};

use crate::failure::GitFailure;

/// The application-owned policy for Git mechanism invocations.
///
/// Isolation is application configuration law and the credential posture is
/// application credential policy. The mechanism stops rather than falling
/// back to an ambient helper or a prompt.
///
/// Every hook runs synchronously on the caller's thread. The wall clock of
/// [`crate::Git::run`] bounds Git processes, not these hooks: a hook that
/// blocks past the bound delays the call's return until it answers. A
/// credential lookup or refresh that can be slow carries its own limit.
pub trait GitPolicy: Send + Sync {
    /// Applies application-owned process isolation and Git configuration.
    ///
    /// The mechanism calls this before it adds the invocation credential, so a
    /// credential environment value wins over isolation.
    fn isolate(&self, command: &mut Command);

    /// The credential for a remote the mechanism identified as one this
    /// application supplies credentials for.
    ///
    /// `Ok(None)` means the application holds no credential for the remote.
    /// `Err` means the application could not decide: the operation stops
    /// instead of running Git with whatever the machine has configured.
    fn credential(&self, remote: &str) -> Result<Option<GitCredential>, GitFailure>;

    /// A replacement for a credential the remote reported as rejected.
    ///
    /// `None` means the application cannot refresh it; the operation keeps the
    /// original refusal.
    fn refresh_credential(&self, rejected: &GitCredential) -> Option<GitCredential>;
}

/// A caller-supplied credential for one Git invocation.
///
/// The environment values are added to the child Git process only. `secret` is
/// the value the mechanism keeps out of every failure detail it returns; the
/// caller decides which environment value carries it.
pub struct GitCredential {
    env_vars: Vec<(String, String)>,
    secret: SecretString,
}

impl GitCredential {
    /// Builds one credential from the child environment values and the secret
    /// value to redact.
    #[must_use]
    pub fn new(env_vars: Vec<(String, String)>, secret: impl Into<String>) -> Self {
        Self {
            env_vars,
            secret: SecretString::from(secret.into()),
        }
    }

    /// The environment values added to a credential-scoped child process.
    #[must_use]
    pub fn env_vars(&self) -> &[(String, String)] {
        &self.env_vars
    }

    /// The value kept out of failure detail. Callers redact their own Git text
    /// with [`crate::redact_git_detail`] instead of reading it here.
    pub(crate) fn secret(&self) -> &str {
        self.secret.expose_secret()
    }
}

impl fmt::Debug for GitCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GitCredential")
            .field(
                "env_keys",
                &self.env_vars.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}
