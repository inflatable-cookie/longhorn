//! The vendor-signature port (contract 027).
//!
//! A declared vendor team is checked on the extracted executable, never on
//! the archive: Apple signs Mach-O binaries, not archives. The check is an
//! injected port so tests script it. The macOS adapter runs the fixed
//! `/usr/bin/codesign` with an argument vector and never through a shell.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(target_os = "macos")]
use std::process::Command;

use crate::{
    extract::StagedExecutable,
    failure::{Fault, ProvisionError, Refusal},
};

/// What a signature check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureReport {
    /// The signature verifies and names this team identifier.
    Team(String),
    /// The signature does not verify, or verifies with no team identifier.
    Unverified {
        /// The verifier's reason, for the refusal's message.
        detail: String,
    },
}

/// Judges one extracted executable's signature.
///
/// `Ok` is a judgment; `Err` is a refusal the adapter makes without judging
/// (for example, the platform has no adapter) or a fault (the verifier could
/// not run).
pub trait SignatureVerifier {
    /// Reports the signature's team, or why it does not verify.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] when the adapter cannot judge at all, or a [`Fault`] when
    /// the verifier cannot run.
    fn report(&self, binary: &Path) -> Result<SignatureReport, ProvisionError>;
}

/// The production adapter.
///
/// On macOS it runs `/usr/bin/codesign --verify --strict` and then
/// `codesign -dv` for the `TeamIdentifier`, reading no credential. On other
/// platforms it refuses with [`Refusal::SignatureUnsupported`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CodesignVerifier;

#[cfg(target_os = "macos")]
const CODESIGN: &str = "/usr/bin/codesign";

#[cfg(target_os = "macos")]
impl SignatureVerifier for CodesignVerifier {
    fn report(&self, binary: &Path) -> Result<SignatureReport, ProvisionError> {
        let run = |arguments: &[&str]| {
            Command::new(CODESIGN)
                .args(arguments)
                .arg(binary)
                .env("PATH", "/usr/bin:/bin")
                .output()
                .map_err(|error| {
                    ProvisionError::Fault(Fault::Verifier {
                        detail: format!("codesign could not run: {error}"),
                    })
                })
        };
        let verify = run(&["--verify", "--strict", "--verbose=1"])?;
        if !verify.status.success() {
            let detail = String::from_utf8_lossy(&verify.stderr);
            return Ok(SignatureReport::Unverified {
                detail: format!("the signature did not verify: {}", detail.trim()),
            });
        }
        // `codesign -dv` writes its facts on stderr.
        let facts = run(&["-dv"])?;
        match team_from_facts(&String::from_utf8_lossy(&facts.stderr)) {
            Some(team) => Ok(SignatureReport::Team(team)),
            None => Ok(SignatureReport::Unverified {
                detail: "the signature names no team identifier".to_owned(),
            }),
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl SignatureVerifier for CodesignVerifier {
    fn report(&self, _binary: &Path) -> Result<SignatureReport, ProvisionError> {
        Err(ProvisionError::Refused(Refusal::SignatureUnsupported {
            platform: std::env::consts::OS.to_owned(),
        }))
    }
}

/// The last `TeamIdentifier=` line that names a team. `not set` and
/// `unknown` are codesign's words for no team.
#[cfg(target_os = "macos")]
fn team_from_facts(facts: &str) -> Option<String> {
    let mut team = None;
    for line in facts.lines() {
        if let Some(value) = line.strip_prefix("TeamIdentifier=") {
            let value = value.trim();
            if !value.is_empty() && value != "not set" && value != "unknown" {
                team = Some(value.to_owned());
            }
        }
    }
    team
}

/// An executable that counts as installable: the signature (when a team was
/// declared) passed. Only [`verify_signature`] makes one.
#[derive(Debug)]
pub struct InstallableExecutable {
    path: PathBuf,
    vendor_team: Option<String>,
}

impl InstallableExecutable {
    /// The staged executable, ready for a later install step to move.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The vendor team the signature verified against, when one was declared.
    pub fn vendor_team(&self) -> Option<&str> {
        self.vendor_team.as_deref()
    }
}

/// Checks the declared vendor team on a staged executable.
///
/// With no team declared the executable passes through: its digest was the
/// declared check, and it was checked before extraction. With a team, the
/// verifier must report that same team.
///
/// # Errors
///
/// [`Refusal::TeamMismatch`] for another team, [`Refusal::Unsigned`] for a
/// signature that does not verify or names no team, the adapter's own
/// [`Refusal`] or [`Fault`]. Every failed check removes the staged file.
/// If removal fails, [`Fault::Io`] reports the path, cleanup failure and
/// original signature error instead.
pub fn verify_signature(
    staged: StagedExecutable,
    verifier: &dyn SignatureVerifier,
) -> Result<InstallableExecutable, ProvisionError> {
    let Some(expected) = staged.vendor_team().map(str::to_owned) else {
        return Ok(InstallableExecutable {
            path: staged.path().to_path_buf(),
            vendor_team: None,
        });
    };
    let error = match verifier.report(staged.path()) {
        Ok(SignatureReport::Team(found)) if found == expected => {
            return Ok(InstallableExecutable {
                path: staged.path().to_path_buf(),
                vendor_team: Some(expected),
            });
        }
        Ok(SignatureReport::Team(found)) => {
            ProvisionError::Refused(Refusal::TeamMismatch { found, expected })
        }
        Ok(SignatureReport::Unverified { detail }) => {
            ProvisionError::Refused(Refusal::Unsigned { expected, detail })
        }
        Err(error) => error,
    };
    if let Err(cleanup) = fs::remove_file(staged.path())
        && cleanup.kind() != io::ErrorKind::NotFound
    {
        return Err(ProvisionError::Fault(Fault::Io {
            path: staged.path().to_path_buf(),
            detail: format!(
                "could not remove the staged executable after signature verification failed ({error}): {cleanup}"
            ),
        }));
    }
    Err(error)
}
