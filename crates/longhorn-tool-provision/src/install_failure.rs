//! Install-pipeline failure classes (contract 027, part 4c).

use std::{fmt, path::PathBuf};

use crate::{ProvisionError, ReleaseError};

/// Why a candidate executable does not satisfy the requested install.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstallRefusal {
    /// The staged executable reports a version below the caller's floor.
    BelowFloor {
        /// The version reported by the executable.
        found: semver::Version,
        /// The caller's minimum version.
        floor: semver::Version,
    },
    /// The executable's version does not match the resolved release version.
    VersionMismatch {
        /// The version reported by the executable.
        found: semver::Version,
        /// The version named by the release.
        expected: semver::Version,
    },
}

/// Whether a staged or installed executable was being probed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbePhase {
    /// The staged candidate, before the current install can change.
    Staged,
    /// The binary at its final managed path, after the swap.
    Installed,
}

/// A host-side failure while the pipeline is running.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstallFault {
    /// A managed file or directory could not be used.
    Io {
        /// The filesystem path involved.
        path: PathBuf,
        /// A bounded, human-readable host description.
        detail: String,
    },
    /// The version probe could not start.
    ProcessStart {
        /// The fixed executable path the runner was asked to start.
        program: PathBuf,
        /// A bounded, redacted runner detail.
        detail: String,
    },
    /// A version process failed or did not report a semantic version.
    Probe {
        /// The probe position in the install protocol.
        phase: ProbePhase,
        /// A bounded, redacted standard output capture.
        stdout: String,
        /// A bounded, redacted standard error capture.
        stderr: String,
        /// The child exit code, if it exited normally.
        exit_status: Option<i32>,
        /// Whether standard output was cut to its limit.
        stdout_truncated: bool,
        /// Whether standard error was cut to its limit.
        stderr_truncated: bool,
        /// A generic reason that carries no child output.
        detail: String,
    },
    /// Another install already owns this managed binary's lock.
    ConcurrentInstall {
        /// The binary path whose install is active.
        binary: PathBuf,
    },
    /// A previous run left a backup that must be resolved before installing.
    RecoveryRequired {
        /// The retained previous binary.
        backup: PathBuf,
    },
    /// The install failed and rollback also encountered a host error.
    Rollback {
        /// The failure that caused rollback.
        cause: Box<InstallError>,
        /// The rollback failure description.
        detail: String,
    },
}

/// Any install-pipeline outcome, preserving the underlying 4a and 4b classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// Release resolution or download failed; its refusal/fault distinction is retained.
    Release(ReleaseError),
    /// Verification, extraction or signature checking failed; its class is retained.
    Provision(ProvisionError),
    /// The artifact's executable version was refused.
    Refused(InstallRefusal),
    /// The host could not complete the install transaction.
    Fault(InstallFault),
}

impl fmt::Display for InstallRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BelowFloor { found, floor } => {
                write!(
                    formatter,
                    "version {found} is below the required floor {floor}"
                )
            }
            Self::VersionMismatch { found, expected } => write!(
                formatter,
                "the staged executable reports {found}, not the release version {expected}"
            ),
        }
    }
}

impl fmt::Display for InstallFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, detail } => write!(formatter, "{}: {detail}", path.display()),
            Self::ProcessStart { program, detail } => {
                write!(
                    formatter,
                    "{} could not be run: {detail}",
                    program.display()
                )
            }
            Self::Probe {
                phase,
                exit_status,
                detail,
                ..
            } => write!(
                formatter,
                "the {phase:?} version probe failed (exit {exit_status:?}): {detail}"
            ),
            Self::ConcurrentInstall { binary } => {
                write!(formatter, "an install already owns {}", binary.display())
            }
            Self::RecoveryRequired { backup } => write!(
                formatter,
                "a previous install backup needs recovery: {}",
                backup.display()
            ),
            Self::Rollback { cause, detail } => {
                write!(formatter, "{cause}; rollback also failed: {detail}")
            }
        }
    }
}

impl fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Release(error) => fmt::Display::fmt(error, formatter),
            Self::Provision(error) => fmt::Display::fmt(error, formatter),
            Self::Refused(refusal) => write!(formatter, "refused: {refusal}"),
            Self::Fault(fault) => write!(formatter, "install fault: {fault}"),
        }
    }
}

impl std::error::Error for InstallRefusal {}
impl std::error::Error for InstallFault {}
impl std::error::Error for InstallError {}

impl From<ReleaseError> for InstallError {
    fn from(error: ReleaseError) -> Self {
        Self::Release(error)
    }
}

impl From<ProvisionError> for InstallError {
    fn from(error: ProvisionError) -> Self {
        Self::Provision(error)
    }
}
