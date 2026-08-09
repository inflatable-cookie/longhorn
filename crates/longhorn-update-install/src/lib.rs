//! Longhorn's update installer: verification, extraction, and atomic bundle
//! replacement. One implementation, every host.
//!
//! Contract 018 was amended on 2026-08-09 to make update execution
//! host-independent: Tauri and GPUI applications both install through this
//! crate. Building for the host with no plugin and letting the other inherit
//! is the only ordering that leaves neither under-served.
//!
//! # Why minisign
//!
//! Tauri's plugin verifies with minisign. This uses the same format and the
//! same key, so a product shipping to both hosts signs once. Two signature
//! schemes would mean two keys and two signing steps per release.
//!
//! # Deliberate divergences from Tauri's implementation
//!
//! Tauri's macOS path was read as a specification for platform behaviour.
//! Three things are done differently, on purpose:
//!
//! 1. **No shell interpolation.** Tauri escalates by building a shell string
//!    (`rm -rf '{src}' && mv -f '{new}' '{src}'`) and running it through
//!    AppleScript with administrator privileges. A path containing a quote
//!    would break out of that string. Escalation here is an injected port,
//!    so a host supplies it without Longhorn constructing shell commands.
//! 2. **Classified failures.** Tauri returns generic IO errors. The contract
//!    requires `NotWritable` to be distinguishable from a transient fault,
//!    because one needs a manual download and the other can retry.
//! 3. **Bounded extraction.** Archive entries are checked before they are
//!    written, so a crafted archive cannot escape the destination.
//!
//! Not diverging on quarantine: Tauri does not strip
//! `com.apple.quarantine`, and it is right not to. The attribute is applied
//! by applications that opt into it, not by an ordinary file write, so
//! extracted files do not carry it.

mod provenance;

pub use provenance::{detect_provenance, observe_install};

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use flate2::read::GzDecoder;
use longhorn_update::{Applied, InstallFailure, UpdateInstaller};
use minisign_verify::{PublicKey, Signature};
use semver::Version;

/// Escalates a replacement the current user cannot perform.
///
/// Injected because escalation is host and platform policy, and because
/// Longhorn constructing privileged shell commands from paths is how command
/// injection happens. A host that cannot escalate returns `NotWritable` and
/// the surface offers a manual download.
pub trait PrivilegedReplace {
    /// Replaces `target` with `staged`, with elevated privileges.
    fn replace(&self, staged: &Path, target: &Path) -> Result<(), String>;
}

/// Escalation that always declines.
///
/// The correct default: an application that has not opted into privileged
/// replacement should tell the user to download it themselves rather than
/// prompting for a password it never asked to need.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPrivilegedReplace;

impl PrivilegedReplace for NoPrivilegedReplace {
    fn replace(&self, _staged: &Path, _target: &Path) -> Result<(), String> {
        Err("privileged replacement is not configured".to_owned())
    }
}

/// Installs updates by verifying, extracting, and replacing in place.
pub struct NativeInstaller<E> {
    key: PublicKey,
    target: PathBuf,
    escalate: E,
}

impl NativeInstaller<NoPrivilegedReplace> {
    /// Records an installer that never escalates.
    #[must_use]
    pub fn new(key: PublicKey, target: impl Into<PathBuf>) -> Self {
        Self {
            key,
            target: target.into(),
            escalate: NoPrivilegedReplace,
        }
    }
}

impl<E> NativeInstaller<E> {
    /// Supplies an escalation port.
    #[must_use]
    pub fn with_escalation<N>(self, escalate: N) -> NativeInstaller<N> {
        NativeInstaller {
            key: self.key,
            target: self.target,
            escalate,
        }
    }

    /// Returns the path this installer replaces.
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }
}

impl<E: PrivilegedReplace> UpdateInstaller for NativeInstaller<E> {
    fn apply(
        &self,
        version: &Version,
        artifact: &[u8],
        signature: &str,
    ) -> Result<Applied, InstallFailure> {
        // Verification first, always. Nothing below this line runs on bytes
        // that have not been proved to come from the signing key.
        verify(&self.key, artifact, signature)?;

        let parent = self.target.parent().ok_or_else(|| InstallFailure::Failed {
            detail: "install target has no parent directory".to_owned(),
        })?;
        let staging = parent.join(format!(".longhorn-update-{version}"));

        drop(fs::remove_dir_all(&staging));
        fs::create_dir_all(&staging).map_err(|error| classify(&error, &staging))?;

        let unpacked = unpack(artifact, &staging).inspect_err(|_| {
            drop(fs::remove_dir_all(&staging));
        })?;

        self.swap(&unpacked).inspect_err(|_| {
            drop(fs::remove_dir_all(&staging));
        })?;
        drop(fs::remove_dir_all(&staging));

        Ok(Applied {
            version: version.clone(),
            // Relaunch is the host's. macOS separates replacement from
            // relaunch and Longhorn keeps that separation rather than
            // hiding it, so the caller orders teardown and restart itself.
            relaunched: false,
        })
    }
}

impl<E: PrivilegedReplace> NativeInstaller<E> {
    fn swap(&self, staged: &Path) -> Result<(), InstallFailure> {
        let backup = self.target.with_extension("longhorn-previous");
        drop(fs::remove_dir_all(&backup));

        // Move the current install aside rather than deleting it, so a
        // failed swap leaves something to put back.
        let displaced = match fs::rename(&self.target, &backup) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                return self.escalated_swap(staged);
            }
            Err(error) => return Err(classify(&error, &self.target)),
        };

        match fs::rename(staged, &self.target) {
            Ok(()) => {
                drop(fs::remove_dir_all(&backup));
                Ok(())
            }
            Err(error) => {
                if displaced {
                    // Put the original back before reporting. A failed
                    // update that also removed the application is worse
                    // than a failed update.
                    drop(fs::rename(&backup, &self.target));
                }
                Err(classify(&error, &self.target))
            }
        }
    }

    fn escalated_swap(&self, staged: &Path) -> Result<(), InstallFailure> {
        self.escalate
            .replace(staged, &self.target)
            .map_err(|detail| InstallFailure::NotWritable { detail })
    }
}

/// Verifies a detached minisign signature over the artifact.
fn verify(key: &PublicKey, artifact: &[u8], signature: &str) -> Result<(), InstallFailure> {
    let signature = Signature::decode(signature).map_err(|_| InstallFailure::SignatureRejected)?;
    key.verify(artifact, &signature, false)
        .map_err(|_| InstallFailure::SignatureRejected)
}

/// Extracts a gzip tar into `staging`, returning the unpacked root.
///
/// Matches Tauri's archive shape — a gzip tar whose single top-level entry
/// is the application — so one release unpacks identically on both hosts.
fn unpack(artifact: &[u8], staging: &Path) -> Result<PathBuf, InstallFailure> {
    let mut archive = tar::Archive::new(GzDecoder::new(artifact));
    let entries = archive
        .entries()
        .map_err(|error| InstallFailure::MalformedArtifact {
            detail: error.to_string(),
        })?;

    let mut root: Option<PathBuf> = None;
    for entry in entries {
        let mut entry = entry.map_err(|error| InstallFailure::MalformedArtifact {
            detail: error.to_string(),
        })?;
        let path = entry
            .path()
            .map_err(|error| InstallFailure::MalformedArtifact {
                detail: error.to_string(),
            })?
            .into_owned();

        let safe = bounded(&path)?;
        if root.is_none() {
            root = safe.components().next().map(|first| staging.join(first));
        }

        let destination = staging.join(&safe);
        // `unpack` does not create parent directories, and a tar may list a
        // nested file before the directory holding it.
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| classify(&error, parent))?;
        }
        entry
            .unpack(&destination)
            .map_err(|error| classify(&error, &destination))?;
    }

    root.ok_or_else(|| InstallFailure::MalformedArtifact {
        detail: "archive contained no entries".to_owned(),
    })
}

/// Rejects any path that could escape the destination.
///
/// Tauri strips the first component and unpacks; this checks first. An
/// archive is untrusted input even after its signature verifies, because a
/// signature proves origin, not good intent.
fn bounded(path: &Path) -> Result<PathBuf, InstallFailure> {
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(InstallFailure::MalformedArtifact {
                    detail: format!("archive entry escapes the destination: {}", path.display()),
                });
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err(InstallFailure::MalformedArtifact {
            detail: "archive entry has an empty path".to_owned(),
        });
    }
    Ok(safe)
}

/// Maps an IO error onto the contract's failure classes.
fn classify(error: &std::io::Error, path: &Path) -> InstallFailure {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        return InstallFailure::NotWritable {
            detail: path.display().to_string(),
        };
    }
    InstallFailure::Failed {
        detail: format!("{}: {error}", path.display()),
    }
}
