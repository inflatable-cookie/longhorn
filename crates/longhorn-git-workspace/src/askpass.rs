//! The invocation-scoped Git askpass credential bridge (contract 029, part
//! 5b).
//!
//! One bridge owns a private temporary folder, the mechanism's own askpass
//! program in it, and the token in [`secrecy`] memory. The token reaches a Git
//! child only as one environment value that the askpass program prints, so it
//! never enters a Git argument, a file, a log line, an error or a `Debug`
//! output. The last bridge or invocation credential removes the folder.
//!
//! The application names the environment variable and nothing else: the helper
//! program is this crate's, not the application's.

use std::{fmt, fs, io, sync::Arc};

use secrecy::{ExposeSecret as _, SecretString};
use tempfile::TempDir;

use crate::{auth::GithubAuthFailure, policy::GitCredential};

/// The application's askpass setting: the name of the environment variable the
/// bridge carries the token in.
///
/// The mechanism writes the helper program itself, so the application supplies
/// no code and no path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskpassConfig {
    token_env: String,
}

impl AskpassConfig {
    /// Names the token environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when the name is not a
    /// shell-safe variable name (`[A-Za-z_][A-Za-z0-9_]*`), because the helper
    /// program reads it by that name.
    pub fn new(token_env: &str) -> Result<Self, GithubAuthFailure> {
        let mut characters = token_env.chars();
        let named = characters
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
            && characters.all(|character| character == '_' || character.is_ascii_alphanumeric());
        if !named {
            return Err(GithubAuthFailure::NotConfigured);
        }
        Ok(Self {
            token_env: token_env.to_owned(),
        })
    }

    /// The environment variable name the bridge carries the token in.
    #[must_use]
    pub fn token_env(&self) -> &str {
        &self.token_env
    }
}

/// One Git invocation's credential: a private askpass program, the token
/// behind it, and the environment values that point Git at both.
///
/// The bridge and credentials derived from it share ownership of the helper folder.
pub struct GitCredentialBridge {
    private_dir: Arc<TempDir>,
    token_env: String,
    token: SecretString,
}

impl GitCredentialBridge {
    /// Writes the askpass program into a new owner-only temporary folder and
    /// keeps the token in memory for this bridge's lifetime.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::CredentialStoreUnavailable`] for an empty
    /// token (an empty value in the store is a store problem, not a signed-out
    /// operator) and [`GithubAuthFailure::Failed`] when the private folder or
    /// the program could not be written.
    pub fn from_token(
        config: &AskpassConfig,
        token: SecretString,
    ) -> Result<Self, GithubAuthFailure> {
        if token.expose_secret().trim().is_empty() {
            return Err(GithubAuthFailure::CredentialStoreUnavailable);
        }
        let program = askpass_program(
            "github.com",
            &format!(
                r#"case "$kind" in
  Username) printf '%s\n' 'x-access-token' ;;
  Password) printf '%s\n' "${{{}:-}}" ;;
esac"#,
                config.token_env
            ),
        );
        let private_dir = write_program(&program).map_err(|_| GithubAuthFailure::Failed)?;
        Ok(Self {
            private_dir: Arc::new(private_dir),
            token_env: config.token_env.clone(),
            token,
        })
    }

    /// The invocation credential for this bridge: the child environment
    /// values, and the token as the value every Git text is redacted with.
    ///
    /// The credential keeps the helper alive through the Git invocation.
    #[must_use]
    pub fn credential(&self) -> GitCredential {
        GitCredential::github(
            self.env_vars(),
            self.token.clone(),
            Arc::clone(&self.private_dir),
        )
    }

    fn env_vars(&self) -> Vec<(String, String)> {
        let askpass = self
            .private_dir
            .path()
            .join("git-askpass")
            .to_string_lossy()
            .into_owned();
        vec![
            ("GIT_ASKPASS".to_owned(), askpass),
            (
                self.token_env.clone(),
                self.token.expose_secret().to_owned(),
            ),
            ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ]
    }
}

impl fmt::Debug for GitCredentialBridge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The token is never named here, and neither is the folder: a path is
        // not a secret, but a diagnostic has no use for it.
        formatter
            .debug_struct("GitCredentialBridge")
            .finish_non_exhaustive()
    }
}

pub(crate) const ASKPASS_DELEGATE: &str = "LONGHORN_GIT_ASKPASS_DELEGATE";

pub(crate) fn guarded_delegate(host: &str) -> io::Result<TempDir> {
    write_program(&askpass_program(
        host,
        r#"exec "$LONGHORN_GIT_ASKPASS_DELEGATE" "$1""#,
    ))
}

pub(crate) fn write_program(program: &str) -> io::Result<TempDir> {
    let private_dir = tempfile::tempdir()?;
    let path = private_dir.path().join("git-askpass");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(private_dir.path(), fs::Permissions::from_mode(0o700))?;
        fs::write(&path, program)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    fs::write(&path, program)?;
    Ok(private_dir)
}

// Git's credential prompts have no path when credential.useHttpPath is false.
// Reject ambiguous prompts before extracting the host from an optional user.
fn askpass_program(host: &str, answer: &str) -> String {
    format!(
        r#"#!/bin/sh
case "${{1:-}}" in
  "Username for '"https://*"': ") kind=Username ;;
  "Password for '"https://*"': ") kind=Password ;;
  *) exit 1 ;;
esac
authority=${{1#*https://}}
authority=${{authority%"': "}}
case "$authority" in
  */*|*\?*|*\#*|*\\*|*"'"*|*[[:space:]]*) exit 1 ;;
esac
host=$(printf '%s' "${{authority##*@}}" | tr '[:upper:]' '[:lower:]')
case "$host" in
  '{host}') ;;
  *) exit 1 ;;
esac
{answer}
"#
    )
}
