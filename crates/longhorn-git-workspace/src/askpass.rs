//! The invocation-scoped Git askpass credential bridge (contract 029, part
//! 5b).
//!
//! One bridge owns a private temporary folder, the mechanism's own askpass
//! program in it, and the token in [`secrecy`] memory. The token reaches a Git
//! child only as one environment value that the askpass program prints, so it
//! never enters a Git argument, a file, a log line, an error or a `Debug`
//! output. The folder is removed when the bridge is dropped.
//!
//! The application names the environment variable and nothing else: the helper
//! program is this crate's, not the application's.

use std::{fmt, fs};

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
/// Keep the bridge alive until the Git invocation finishes: the helper folder
/// exists for exactly that long.
pub struct GitCredentialBridge {
    private_dir: TempDir,
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
        let private_dir = tempfile::tempdir().map_err(|_| GithubAuthFailure::Failed)?;
        #[cfg(unix)]
        {
            // The folder holds a program Git runs as this user and nothing
            // else may read it, so it is owner-only even where the platform's
            // own temporary folder mode is wider.
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(private_dir.path(), fs::Permissions::from_mode(0o700))
                .map_err(|_| GithubAuthFailure::Failed)?;
        }
        let askpass_path = private_dir.path().join("git-askpass");
        fs::write(&askpass_path, askpass_program(&config.token_env))
            .map_err(|_| GithubAuthFailure::Failed)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let permissions = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&askpass_path, permissions)
                .map_err(|_| GithubAuthFailure::Failed)?;
        }
        Ok(Self {
            private_dir,
            token_env: config.token_env.clone(),
            token,
        })
    }

    /// The invocation credential for this bridge: the child environment
    /// values, and the token as the value every Git text is redacted with.
    ///
    /// The environment also disables Git's configured credential helpers for
    /// this child only, so a stale machine-wide helper cannot run before the
    /// bridge.
    #[must_use]
    pub fn credential(&self) -> GitCredential {
        GitCredential::new(self.env_vars(), self.token.expose_secret().to_owned())
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
            ("GIT_CONFIG_COUNT".to_owned(), "1".to_owned()),
            (
                "GIT_CONFIG_KEY_0".to_owned(),
                "credential.helper".to_owned(),
            ),
            ("GIT_CONFIG_VALUE_0".to_owned(), String::new()),
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

/// The askpass program: the token for a password prompt, Git's own convention
/// for a username prompt.
///
/// GitHub accepts any username with a personal-access or OAuth token, and
/// `x-access-token` is the documented spelling for token authentication. The
/// token is never part of this program's text.
fn askpass_program(token_env: &str) -> String {
    let program = r#"#!/bin/sh
# Longhorn Git askpass (contract 029): reads TOKEN_ENV and prints nothing else.
case "${1:-}" in
  *[Uu]sername*) printf '%s\n' 'x-access-token' ;;
  *) printf '%s\n' "${TOKEN_ENV:-}" ;;
esac
"#;
    program.replace("TOKEN_ENV", token_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_reads_the_named_variable_and_holds_no_token() {
        let program = askpass_program("EXAMPLE_TOKEN");
        assert!(program.contains("${EXAMPLE_TOKEN:-}"));
        assert!(program.contains("x-access-token"));
        // The placeholder is replaced, so nothing else names the variable.
        assert!(!program.contains("TOKEN_ENV"));
    }
}
