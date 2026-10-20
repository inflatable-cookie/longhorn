//! Login-shell PATH discovery and executable lookup (contract 027, part 4c).

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::runner::{MAX_PROCESS_OUTPUT_BYTES, ProcessRunner};

/// The neutral opening marker printed by the fixed login-shell script.
pub const LOGIN_PATH_BEGIN: &str = "__LONGHORN_LOGIN_PATH_BEGIN__";
/// The neutral closing marker printed by the fixed login-shell script.
pub const LOGIN_PATH_END: &str = "__LONGHORN_LOGIN_PATH_END__";

const LOGIN_PATH_SCRIPT: &str = "printf '%s\\n' '__LONGHORN_LOGIN_PATH_BEGIN__'; printf '%s\\n' \"$PATH\"; printf '%s\\n' '__LONGHORN_LOGIN_PATH_END__'";

/// The result of resolving a login-shell PATH.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginPath {
    value: OsString,
    detail: Option<String>,
}

impl LoginPath {
    /// The PATH from the login shell, or the process PATH after a fallback.
    pub fn value(&self) -> &OsStr {
        &self.value
    }

    /// The fallback explanation, if the login shell could not be read.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

/// Per-host cache for login-shell PATH discovery.
///
/// The first call to [`get`](Self::get) resolves the login shell. Subsequent
/// calls return the cached value until [`refresh`](Self::refresh) is called.
#[derive(Debug, Default)]
pub struct LoginPathCache {
    cached: Mutex<Option<LoginPath>>,
}

impl LoginPathCache {
    /// Returns the cached value, resolving the login shell on first use.
    pub fn get(&self, runner: &dyn ProcessRunner) -> LoginPath {
        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cached
            .get_or_insert_with(|| resolve_login_path(runner))
            .clone()
    }

    /// Resolves the login shell again and replaces the cached value.
    pub fn refresh(&self, runner: &dyn ProcessRunner) -> LoginPath {
        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let refreshed = resolve_login_path(runner);
        *cached = Some(refreshed.clone());
        refreshed
    }
}

/// Why the fixed login-shell PATH markers could not be parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginPathParseError {
    /// The opening marker was absent.
    BeginMarkerMissing,
    /// No PATH line followed the opening marker.
    PathMissing,
    /// The PATH line was empty.
    PathEmpty,
    /// The closing marker did not immediately follow the PATH line.
    EndMarkerMissing,
}

impl std::fmt::Display for LoginPathParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BeginMarkerMissing => "the login shell returned no opening marker",
            Self::PathMissing => "the login shell returned no PATH line",
            Self::PathEmpty => "the login shell returned an empty PATH",
            Self::EndMarkerMissing => "the login shell returned no closing marker",
        })
    }
}

impl std::error::Error for LoginPathParseError {}

/// Parses the PATH between the fixed markers, ignoring shell startup output
/// before the opening marker.
pub fn parse_login_path_output(output: &str) -> Result<OsString, LoginPathParseError> {
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        if line == LOGIN_PATH_BEGIN {
            let path = lines.next().ok_or(LoginPathParseError::PathMissing)?;
            if path.trim().is_empty() {
                return Err(LoginPathParseError::PathEmpty);
            }
            if lines.next() != Some(LOGIN_PATH_END) {
                return Err(LoginPathParseError::EndMarkerMissing);
            }
            return Ok(OsString::from(path));
        }
    }
    Err(LoginPathParseError::BeginMarkerMissing)
}

/// Finds the first executable named `name` on a PATH, preserving PATH order.
/// Relative PATH entries are ignored.
#[must_use]
pub fn find_executable_on_path(name: &str, path: &OsStr) -> Option<PathBuf> {
    if !is_plain_name(name) {
        return None;
    }
    env::split_paths(path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

/// Whether `path` names a regular executable file. On Unix this follows
/// symlinks and requires at least one executable permission bit.
#[must_use]
pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn resolve_login_path(runner: &dyn ProcessRunner) -> LoginPath {
    let (shell, arguments) = login_shell_command();
    let path_result = runner
        .run(Path::new(shell), &arguments)
        .ok()
        .filter(|output| {
            output.success()
                && !output.stdout_truncated()
                && output.stdout().len() <= MAX_PROCESS_OUTPUT_BYTES
        })
        .and_then(|output| String::from_utf8(output.stdout().to_vec()).ok())
        .ok_or("the login shell failed or its output was not readable")
        .and_then(|output| {
            parse_login_path_output(&output).map_err(|_| "the login PATH markers were invalid")
        });

    match path_result {
        Ok(value) => LoginPath {
            value,
            detail: None,
        },
        Err(detail) => LoginPath {
            value: env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin")),
            detail: Some(format!(
                "Could not read the login-shell PATH ({detail}); using the process PATH."
            )),
        },
    }
}

fn login_shell_command() -> (&'static str, Vec<OsString>) {
    #[cfg(target_os = "macos")]
    let shell = "/bin/zsh";
    #[cfg(not(target_os = "macos"))]
    let shell = "/bin/bash";
    (
        shell,
        vec![OsString::from("-lc"), OsString::from(LOGIN_PATH_SCRIPT)],
    )
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
}
