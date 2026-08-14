//! Longhorn's update installer: verification, extraction, and atomic bundle
//! replacement. One implementation, every host.
//!
//! Contract 018 was amended on 2026-08-09 to make update execution
//! host-independent: Tauri and GPUI applications both install through this
//! crate. Building for the host with no plugin and letting the other inherit
//! is the only ordering that leaves neither under-served.
//!
//! # Verification is not here
//!
//! It was, and for the right reason: beside the write it guards. It moved to
//! `longhorn-update`'s `verify_artifact` on 2026-08-12 so that the type
//! system, rather than this crate's diligence, is what stops unverified bytes
//! reaching disk. `apply` takes a `VerifiedArtifact` and there is no way to
//! make one without verifying.
//!
//! Still minisign, and still Tauri's format and key, for the reason recorded
//! when this crate chose it: a product shipping to both hosts signs once, and
//! two schemes would mean two keys and two signing steps per release.
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
//!    written, so a crafted archive cannot escape the destination. Entry
//!    *names* reject absolute and `..` components (`bounded`); link entries
//!    additionally reject absolute or `..` *targets*, so a planted symlink or
//!    hard link cannot point outside staging; and the write itself goes
//!    through tar's `unpack_in`, which canonicalizes the destination's parent
//!    and refuses anything resolving outside — defence in depth behind this
//!    crate's own checks. The bundle root must be a real directory, and entry
//!    types an application bundle cannot contain (devices, fifos) are
//!    refused.
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
use longhorn_update::{Applied, InstallFailure, UpdateInstaller, VerifiedArtifact};

/// Staging directories carry this prefix so a later apply can sweep the ones
/// a crashed or failed install left behind.
const STAGING_PREFIX: &str = ".longhorn-update-";

/// The most one artifact may unpack to.
///
/// The archive is gzip: a small download can expand without bound, and the
/// signature over it proves origin, not intent. Four GiB is far past any
/// desktop application bundle this crate will meet and still finite, which
/// is the whole point.
const MAX_EXTRACTED_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Escalates a replacement the current user cannot perform.
///
/// Injected because escalation is host and platform policy, and because
/// Longhorn constructing privileged shell commands from paths is how command
/// injection happens. A host that cannot escalate returns `NotWritable` and
/// the surface offers a manual download.
///
/// # The artifact, not a staged path
///
/// The signature covers the archive bytes — not an extracted tree — and a
/// tree staged in a user-writable directory can be modified between unpack
/// and the privileged move. So the port receives the [`VerifiedArtifact`]
/// itself: an implementor extracts it into a location the user cannot write
/// ([`extract_bundle`] is that extraction, with the same bounds the
/// unprivileged path gets) and moves the result onto the target. A
/// privileged move of user-writable content is the exact hole this shape
/// closes.
pub trait PrivilegedReplace {
    /// Replaces `target` with `artifact`, with elevated privileges.
    fn replace(&self, artifact: &VerifiedArtifact, target: &Path) -> Result<(), String>;
}

/// Escalation that always declines.
///
/// The correct default: an application that has not opted into privileged
/// replacement should tell the user to download it themselves rather than
/// prompting for a password it never asked to need.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPrivilegedReplace;

impl PrivilegedReplace for NoPrivilegedReplace {
    fn replace(&self, _artifact: &VerifiedArtifact, _target: &Path) -> Result<(), String> {
        Err("privileged replacement is not configured".to_owned())
    }
}

/// Extracts a verified artifact into `staging`, bounded, returning the
/// unpacked bundle root.
///
/// Public so a [`PrivilegedReplace`] implementor performs the same bounded
/// extraction in its own protected staging rather than trusting a tree the
/// user could have modified. `staging` must already exist, and should be a
/// directory the invoking user cannot write.
pub fn extract_bundle(
    artifact: &VerifiedArtifact,
    staging: &Path,
) -> Result<PathBuf, InstallFailure> {
    unpack(artifact.bytes(), staging)
}

/// Installs updates by extracting and replacing in place.
///
/// Holds no key. The controller verifies and hands over a `VerifiedArtifact`;
/// an installer with its own key could have been built with a different one
/// from the controller checking on its behalf, and nothing would have noticed.
pub struct NativeInstaller<E> {
    target: PathBuf,
    escalate: E,
}

impl NativeInstaller<NoPrivilegedReplace> {
    /// Records an installer that never escalates.
    #[must_use]
    pub fn new(target: impl Into<PathBuf>) -> Self {
        Self {
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
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        let version = artifact.version();

        let parent = self.target.parent().ok_or_else(|| InstallFailure::Failed {
            detail: "install target has no parent directory".to_owned(),
        })?;
        self.restore_if_displaced()?;
        sweep_staging(parent);

        // A unique, exclusively-created staging directory: a pre-planted
        // symlink at a predictable path cannot redirect the extraction.
        //
        // An unwritable parent is exactly the case escalation exists for, so
        // it escalates here — with the artifact, not a staged tree — rather
        // than failing before the port is ever reached.
        let staging = match tempfile::Builder::new()
            .prefix(&format!("{STAGING_PREFIX}{version}-"))
            .tempdir_in(parent)
        {
            Ok(staging) => staging,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                self.escalated_swap(artifact)?;
                return Ok(Applied {
                    version: version.clone(),
                    relaunched: false,
                });
            }
            Err(error) => return Err(classify(&error, parent)),
        };

        let unpacked = unpack(artifact.bytes(), staging.path())?;

        self.swap(&unpacked, artifact)?;
        // The swapped-in root has been renamed out; whatever else remains is
        // removed when `staging` drops, and the next apply sweeps regardless.
        drop(staging);

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
    fn swap(&self, staged: &Path, artifact: &VerifiedArtifact) -> Result<(), InstallFailure> {
        let backup = self.target.with_extension("longhorn-previous");
        drop(fs::remove_dir_all(&backup));

        // Move the current install aside rather than deleting it, so a
        // failed swap leaves something to put back.
        let displaced = match fs::rename(&self.target, &backup) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                return self.escalated_swap(artifact);
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

    /// Escalation re-extracts from the verified artifact in the privileged
    /// context — the user-writable staging tree is never what gets moved.
    fn escalated_swap(&self, artifact: &VerifiedArtifact) -> Result<(), InstallFailure> {
        self.escalate
            .replace(artifact, &self.target)
            .map_err(|detail| InstallFailure::NotWritable { detail })
    }

    /// Puts back an install a crash displaced.
    ///
    /// `swap` renames the target aside before the staged bundle moves in. A
    /// kill between the two renames leaves the application missing and a
    /// `*.longhorn-previous` backup beside the gap; the next apply restores
    /// it before attempting anything new, so a crashed update can never cost
    /// the install it was replacing. A backup beside an *existing* target is
    /// leftover from after a successful swap and is cleared there instead.
    fn restore_if_displaced(&self) -> Result<(), InstallFailure> {
        let backup = self.target.with_extension("longhorn-previous");
        if self.target.exists() || !backup.exists() {
            return Ok(());
        }
        fs::rename(&backup, &self.target).map_err(|error| classify(&error, &self.target))
    }
}

/// Extracts a gzip tar into `staging`, returning the unpacked root.
///
/// Matches Tauri's archive shape — a gzip tar whose single top-level entry
/// is the application — so one release unpacks identically on both hosts.
///
/// Three layers keep the archive inside `staging`: `bounded` rejects
/// escaping entry names and link targets before anything is written,
/// `assert_inside` refuses a destination whose existing ancestors resolve
/// outside staging (a link an earlier entry planted), and tar's own
/// `unpack_in` canonicalizes the destination's parent as the backstop.
fn unpack(artifact: &[u8], staging: &Path) -> Result<PathBuf, InstallFailure> {
    let mut archive = tar::Archive::new(GzDecoder::new(artifact));
    let entries = archive
        .entries()
        .map_err(|error| InstallFailure::MalformedArtifact {
            detail: error.to_string(),
        })?;

    let mut root: Option<PathBuf> = None;
    let mut declared_bytes: u64 = 0;
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

        // Global pax headers carry archive-wide metadata, not content; the
        // iterator hands them through rather than consuming them, and there
        // is nothing to unpack.
        if entry.header().entry_type().is_pax_global_extensions() {
            continue;
        }

        let safe = bounded(&path)?;

        // An application bundle contains files, directories, and links.
        // Anything else — devices, fifos — is not an update, it is a crafted
        // archive.
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir() || kind.is_symlink() || kind.is_hard_link()) {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!("archive entry has an unsupported type: {}", path.display()),
            });
        }

        // A link's *name* was checked by `bounded`; its target is data too.
        // Links stay relative and in-tree, which is all a bundle needs —
        // absolute or `..` targets exist only to point outside staging.
        if kind.is_symlink() || kind.is_hard_link() {
            let target = entry
                .link_name()
                .map_err(|error| InstallFailure::MalformedArtifact {
                    detail: error.to_string(),
                })?
                .ok_or_else(|| InstallFailure::MalformedArtifact {
                    detail: format!("archive link has no target: {}", path.display()),
                })?;
            bounded(&target)?;
        }

        if root.is_none() {
            root = safe.components().next().map(|first| staging.join(first));
        }

        // The declared sizes are the archive's own claim — the only bound
        // available before a byte is written. A header understating its
        // payload desyncs the entry stream and fails as malformed.
        declared_bytes = declared_bytes.saturating_add(entry.header().size().map_err(|error| {
            InstallFailure::MalformedArtifact {
                detail: error.to_string(),
            }
        })?);
        if declared_bytes > MAX_EXTRACTED_BYTES {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!("archive exceeds the {MAX_EXTRACTED_BYTES}-byte extraction quota"),
            });
        }

        let destination = staging.join(&safe);
        assert_inside(staging, &destination)?;
        // `unpack_in` creates parents and re-validates canonically. A `false`
        // is tar skipping an entry it considers unsafe; after `bounded` that
        // should be unreachable, and it is refused rather than trusted.
        if !entry
            .unpack_in(staging)
            .map_err(|error| classify(&error, &destination))?
        {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!("archive entry was skipped as unsafe: {}", path.display()),
            });
        }
    }

    let root = root.ok_or_else(|| InstallFailure::MalformedArtifact {
        detail: "archive contained no entries".to_owned(),
    })?;
    // The root is renamed onto the install target, so it must be a real
    // directory — a symlink here would make the application a pointer at
    // wherever the archive's author chose.
    let metadata = fs::symlink_metadata(&root).map_err(|error| classify(&error, &root))?;
    if !metadata.file_type().is_dir() {
        return Err(InstallFailure::MalformedArtifact {
            detail: "archive root is not a directory".to_owned(),
        });
    }
    Ok(root)
}

/// Rejects any path that could escape the destination.
///
/// Tauri strips the first component and unpacks; this checks first. An
/// archive is untrusted input even after its signature verifies, because a
/// signature proves origin, not good intent. Applied to entry names and to
/// link targets alike.
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

/// Refuses a destination whose existing ancestors resolve outside `staging`.
///
/// `bounded` checks the path as written; this checks it as the filesystem
/// will resolve it, so an entry cannot write through a link an earlier entry
/// planted. Walks up to the deepest ancestor that exists — the destination
/// itself usually does not yet — and canonicalizes from there.
fn assert_inside(staging: &Path, destination: &Path) -> Result<(), InstallFailure> {
    let canonical_root = staging
        .canonicalize()
        .map_err(|error| classify(&error, staging))?;
    let mut probe = destination;
    loop {
        match probe.canonicalize() {
            Ok(canonical) => {
                if canonical.starts_with(&canonical_root) {
                    return Ok(());
                }
                return Err(InstallFailure::MalformedArtifact {
                    detail: format!(
                        "archive entry escapes the destination through a link: {}",
                        destination.display()
                    ),
                });
            }
            Err(_) => match probe.parent() {
                Some(parent) if parent.starts_with(staging) => probe = parent,
                _ => return Ok(()),
            },
        }
    }
}

/// Removes staging directories a crashed or failed install left behind.
///
/// Best-effort, and `remove_dir_all` does not follow symlinks: a planted
/// link is removed, not its target. Leftover `*.longhorn-previous` backups
/// are the swap's own concern — it clears one before displacing, and a
/// backup beside a *missing* target is recovery material, not litter.
fn sweep_staging(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)
        {
            drop(fs::remove_dir_all(entry.path()));
        }
    }
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
