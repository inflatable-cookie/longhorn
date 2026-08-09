use std::{error::Error, fmt};

use crate::BrowserUrl;

/// Opens a URL in the user's own browser.
///
/// Contract 020 names system browser launch as a delegated capability, and
/// contract 019's RFC 8252 flow is what needs it: a native application must
/// authorize in the system browser, not an embedded webview, so the user's
/// password manager and SSO work and so the application never sees the
/// credentials.
///
/// Injectable because a product may want confirmation, logging, or a policy
/// on which hosts it will send a user to. Neither backend provides this:
/// Tauri has a plugin Longhorn does not take, and GPUI has nothing, so
/// Longhorn implements it once for both.
pub trait SystemBrowser {
    /// Hands one validated URL to the platform and returns once it is
    /// accepted.
    ///
    /// Returning `Ok` means the platform accepted the request, not that the
    /// user completed anything. The flow is finished by the loopback
    /// redirect, not by this call.
    fn open(&mut self, url: &BrowserUrl) -> Result<(), BrowserLaunchError>;
}

/// Why a launch did not happen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserLaunchError {
    /// No launcher is implemented for this platform.
    ///
    /// Recorded rather than guessed. A launcher that pipes a URL through a
    /// command interpreter it has not been tested against is a worse answer
    /// than saying so.
    UnsupportedPlatform {
        /// The target family that has no implementation.
        target: &'static str,
    },
    /// The platform launcher could not be started.
    LauncherUnavailable {
        /// The program that could not run.
        program: &'static str,
        /// Boundary diagnostic.
        detail: String,
    },
    /// The platform launcher ran and refused.
    LauncherFailed {
        /// The program that ran.
        program: &'static str,
        /// Exit status rendering, when there was one.
        status: String,
    },
    /// Product policy declined to send the user anywhere.
    Declined {
        /// Why.
        reason: String,
    },
}

impl fmt::Display for BrowserLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform { target } => {
                write!(formatter, "no system browser launcher for {target}")
            }
            Self::LauncherUnavailable { program, detail } => {
                write!(formatter, "could not run {program}: {detail}")
            }
            Self::LauncherFailed { program, status } => {
                write!(formatter, "{program} exited with {status}")
            }
            Self::Declined { reason } => {
                write!(formatter, "browser launch declined: {reason}")
            }
        }
    }
}

impl Error for BrowserLaunchError {}

/// The platform launcher.
///
/// **Never a shell.** Every implementation spawns a program directly with the
/// URL as one argument, so no part of the URL is ever parsed as a command.
/// That is the reason this is not simply `sh -c "open $url"`, which is how
/// this capability is usually written and why it is usually a vulnerability.
/// [`BrowserUrl`] refuses shell metacharacters as well, so the two defences
/// are independent.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeSystemBrowser;

impl SystemBrowser for NativeSystemBrowser {
    fn open(&mut self, url: &BrowserUrl) -> Result<(), BrowserLaunchError> {
        let program = launcher_program()?;
        let status = std::process::Command::new(program)
            // One argument. Not a format string, not a shell word.
            .arg(url.as_str())
            .status()
            .map_err(|error| BrowserLaunchError::LauncherUnavailable {
                program,
                detail: error.to_string(),
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(BrowserLaunchError::LauncherFailed {
                program,
                status: status.to_string(),
            })
        }
    }
}

const fn launcher_program() -> Result<&'static str, BrowserLaunchError> {
    #[cfg(target_os = "macos")]
    {
        Ok("/usr/bin/open")
    }
    // The freedesktop launcher. Absolute paths differ across distributions,
    // so this one resolves through `PATH` — the argument still never reaches
    // a shell.
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Ok("xdg-open")
    }
    #[cfg(not(unix))]
    {
        Err(BrowserLaunchError::UnsupportedPlatform {
            target: std::env::consts::FAMILY,
        })
    }
}

/// A launcher that refuses, with a reason.
///
/// The right default for a product that has not decided whether it sends
/// users to a browser at all. Declining is a policy; silently doing nothing
/// is a bug that looks like one.
#[derive(Clone, Debug)]
pub struct DecliningSystemBrowser {
    reason: String,
}

impl DecliningSystemBrowser {
    /// Records why this product will not launch a browser.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl SystemBrowser for DecliningSystemBrowser {
    fn open(&mut self, _url: &BrowserUrl) -> Result<(), BrowserLaunchError> {
        Err(BrowserLaunchError::Declined {
            reason: self.reason.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_launcher_is_a_program_and_never_a_shell() {
        // The whole safety argument rests on this: no `sh`, no `cmd`, no
        // `-c`. If this assertion ever needs relaxing, the URL is being
        // parsed by something that also parses commands.
        let program = launcher_program().unwrap_or("unsupported");

        assert!(!program.contains("sh"), "{program}");
        assert!(!program.contains("cmd"), "{program}");
    }

    #[test]
    fn declining_is_a_reported_policy_and_not_a_silent_no_op() {
        let url = BrowserUrl::new("https://accounts.example.com/authorize").unwrap();
        let mut browser = DecliningSystemBrowser::new("this build has no account flow");

        assert_eq!(
            browser.open(&url),
            Err(BrowserLaunchError::Declined {
                reason: "this build has no account flow".to_owned(),
            })
        );
    }
}
