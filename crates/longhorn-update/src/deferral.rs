use core::fmt;

use semver::Version;

use crate::InstallManager;
use serde::{Deserialize, Serialize};

/// Why an install did not proceed.
///
/// Deferral is a decision with a cause, never a silent skip: an update that
/// quietly does not happen is indistinguishable from one that is broken.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "cause")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum DeferralCause {
    /// The user chose to wait.
    UserPostponed,
    /// Longhorn-owned work was in flight.
    ///
    /// Raised by the restart interlock, which is the only thing that knows
    /// whether a transfer is mid-commit or a flush is pending.
    WorkInFlight {
        /// What was in flight, for display.
        detail: String,
    },
    /// The installed copy is not writable.
    ///
    /// An administrator-installed copy this user cannot replace. The remedy
    /// is a manual download, not a retry.
    ///
    /// Package-manager installations are **not** this — they are
    /// [`Self::ExternallyManaged`], caught before an update is offered. This
    /// variant claimed to cover Homebrew casks until 2026-08-09 and never
    /// did: `/Applications` is group-writable by admin users, so a cask
    /// passes the permission check and self-updates.
    InstallationNotWritable {
        /// What could not be written, for display.
        detail: String,
    },
    /// A package manager owns the installation.
    ///
    /// Not a failure and not retryable: the update is real and the user can
    /// have it, through the tool that installed the application. Self-updating
    /// here would leave the manager's database describing a version that is
    /// no longer on disk.
    ExternallyManaged {
        /// Who owns it, for display.
        manager: InstallManager,
        /// What the user should run, when the answer is a command.
        command: Option<String>,
    },
    /// The download or replacement failed.
    ///
    /// Distinct from `WorkInFlight`: nothing Longhorn owns was running, and
    /// saying otherwise would tell the user the wrong story about why the
    /// update did not happen. A retry can succeed.
    InstallFailed {
        /// What went wrong, for display.
        detail: String,
    },
}

impl DeferralCause {
    /// Returns whether retrying later could succeed unattended.
    ///
    /// A non-writable or externally managed installation cannot resolve
    /// itself; the other causes can. The client surface uses this to decide
    /// between "we will try again" and "here is how to do it yourself".
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        !matches!(
            self,
            Self::InstallationNotWritable { .. } | Self::ExternallyManaged { .. }
        )
    }
}

impl fmt::Display for DeferralCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserPostponed => formatter.write_str("postponed by the user"),
            Self::WorkInFlight { detail } => write!(formatter, "work in flight: {detail}"),
            Self::InstallationNotWritable { detail } => {
                write!(formatter, "installation is not writable: {detail}")
            }
            Self::InstallFailed { detail } => {
                write!(formatter, "update install failed: {detail}")
            }
            Self::ExternallyManaged { manager, command } => match command {
                Some(command) => {
                    write!(formatter, "installed by {manager}; update with `{command}`")
                }
                None => write!(formatter, "installed by {manager}; update it there"),
            },
        }
    }
}

/// A recorded decision not to install yet.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deferral {
    /// The version that was not installed.
    pub version: Version,
    /// Why it was not installed.
    pub cause: DeferralCause,
}

impl Deferral {
    /// Records a deferral.
    #[must_use]
    pub const fn new(version: Version, cause: DeferralCause) -> Self {
        Self { version, cause }
    }

    /// Returns whether this deferral still applies to an offered version.
    ///
    /// A deferral covers the version it was taken against and nothing later:
    /// declining `1.3.0` is not a standing refusal of `1.3.1`, and treating
    /// it as one would silently strand an install.
    #[must_use]
    pub fn applies_to(&self, offered: &Version) -> bool {
        &self.version == offered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(value: &str) -> Version {
        Version::parse(value).unwrap()
    }

    #[test]
    fn a_deferral_does_not_carry_to_a_later_version() {
        let deferral = Deferral::new(version("1.3.0"), DeferralCause::UserPostponed);

        assert!(deferral.applies_to(&version("1.3.0")));
        assert!(!deferral.applies_to(&version("1.3.1")));
    }

    #[test]
    fn only_a_non_writable_installation_is_unretryable() {
        assert!(DeferralCause::UserPostponed.is_retryable());
        assert!(
            DeferralCause::WorkInFlight {
                detail: "transfer session open".into()
            }
            .is_retryable()
        );
        assert!(
            !DeferralCause::InstallationNotWritable {
                detail: "/Applications/Example.app".into()
            }
            .is_retryable()
        );
    }

    #[test]
    fn a_deferral_states_its_cause() {
        let deferral = Deferral::new(
            version("1.3.0"),
            DeferralCause::WorkInFlight {
                detail: "2 pending flushes".into(),
            },
        );

        assert_eq!(
            deferral.cause.to_string(),
            "work in flight: 2 pending flushes"
        );
    }
}
