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
mod workspace;

pub use provenance::{detect_provenance, observe_install};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use flate2::read::GzDecoder;
use longhorn_update::{Applied, InstallFailure, UpdateInstaller, VerifiedArtifact};

use workspace::{Backup, InstallWorkspace, LockedTarget, entry_exists};

/// The most one artifact may unpack to.
///
/// The archive is gzip: a small download can expand without bound, and the
/// signature over it proves origin, not intent. Four GiB is far past any
/// desktop application bundle this crate will meet and still finite, which
/// is the whole point.
const MAX_EXTRACTED_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// The most entries one artifact may unpack to.
///
/// Bytes are not the only finite resource. Directories, links and empty files
/// all declare a size of zero, so any number of them passes the byte quota
/// untouched while still consuming inodes and directory space. A large
/// desktop bundle is tens of thousands of files; a quarter of a million is
/// not a bundle.
const MAX_EXTRACTED_ENTRIES: usize = 250_000;

/// What one extraction may spend.
///
/// Injectable so both limits can be proved against a small archive. Reaching
/// the shipped values in a test would mean building the hostile artifact the
/// quota exists to refuse.
#[derive(Clone, Copy)]
struct ExtractionQuota {
    bytes: u64,
    entries: usize,
}

impl ExtractionQuota {
    const DEFAULT: Self = Self {
        bytes: MAX_EXTRACTED_BYTES,
        entries: MAX_EXTRACTED_ENTRIES,
    };
}

/// Escalates a replacement the current user cannot perform.
///
/// Injected because escalation is host and platform policy, and because
/// Longhorn constructing privileged shell commands from paths is how command
/// injection happens. A host that cannot escalate reports
/// [`InstallFailure::NotWritable`] and the surface offers a manual download.
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
///
/// # The failure is the contract's, not a string
///
/// Escalated replacement is the one path this crate cannot observe, so the
/// port says which of contract 018's classes its failure belongs to. A string
/// would flatten all three into one, and the controller acts on the
/// difference: malformed is terminal and discarded from staging, `NotWritable`
/// needs a manual download, and a partial replacement is retryable.
pub trait PrivilegedReplace {
    /// Replaces the `target` entry with `artifact`, with elevated privileges.
    /// The parent is resolved and locked. A final symlink is replaced as an entry.
    ///
    /// Returns [`InstallFailure::MalformedArtifact`] when the implementor's own
    /// bounded extraction refuses the artifact (terminal — the controller
    /// discards the staged bytes rather than replaying them),
    /// [`InstallFailure::NotWritable`] when escalation is declined or refused,
    /// and [`InstallFailure::Failed`] naming both faults and the retained
    /// bundle's location when a partial replacement could not be completed.
    fn replace(&self, artifact: &VerifiedArtifact, target: &Path) -> Result<(), InstallFailure>;
}

/// Escalation that always declines.
///
/// The correct default: an application that has not opted into privileged
/// replacement should tell the user to download it themselves rather than
/// prompting for a password it never asked to need.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPrivilegedReplace;

impl PrivilegedReplace for NoPrivilegedReplace {
    fn replace(&self, _artifact: &VerifiedArtifact, _target: &Path) -> Result<(), InstallFailure> {
        Err(InstallFailure::NotWritable {
            detail: "privileged replacement is not configured".to_owned(),
        })
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
/// Native bundle replacement requires Unix directory locks.
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

    /// Returns the configured destination path.
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }
}

impl<E: PrivilegedReplace> UpdateInstaller for NativeInstaller<E> {
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        self.apply_with_rename(artifact, |from, to| fs::rename(from, to))
    }
}

impl<E: PrivilegedReplace> NativeInstaller<E> {
    fn apply_with_rename(
        &self,
        artifact: &VerifiedArtifact,
        rename: impl Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<Applied, InstallFailure> {
        let locked = LockedTarget::acquire(&self.target)?;
        let target = &locked.path;
        match InstallWorkspace::acquire(target)? {
            Some(workspace) => self.apply_in_workspace(target, &workspace, artifact, rename)?,
            None => {
                check_legacy_backup(target)?;
                self.escalated_swap(target, artifact)?;
            }
        }

        Ok(Applied {
            version: artifact.version().clone(),
            // Relaunch is the host's. macOS separates replacement from
            // relaunch and Longhorn keeps that separation rather than
            // hiding it, so the caller orders teardown and restart itself.
            relaunched: false,
        })
    }

    fn apply_in_workspace(
        &self,
        target: &Path,
        workspace: &InstallWorkspace,
        artifact: &VerifiedArtifact,
        rename: impl Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<(), InstallFailure> {
        let backup = workspace.backup();
        backup.restore(target)?;
        check_legacy_backup(target)?;

        // Only this attempt's TempDir is eligible for staging cleanup.
        let staging = match tempfile::Builder::new()
            .prefix("stage-")
            .tempdir_in(&workspace.root)
        {
            Ok(staging) => staging,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                return self.escalated_swap(target, artifact);
            }
            Err(error) => return Err(classify(&error, &workspace.root)),
        };
        let unpacked = unpack(artifact.bytes(), staging.path())?;
        self.swap(target, &unpacked, &backup, artifact, rename)
    }

    fn swap(
        &self,
        target: &Path,
        staged: &Path,
        backup: &Backup<'_>,
        artifact: &VerifiedArtifact,
        rename: impl Fn(&Path, &Path) -> io::Result<()>,
    ) -> Result<(), InstallFailure> {
        // Move the current install aside rather than deleting it, so a
        // failed swap leaves something to put back.
        backup.record(target)?;
        let displaced = match rename(target, &backup.bundle) {
            Ok(()) => true,
            Err(error) => {
                backup.clear_receipt();
                match error.kind() {
                    io::ErrorKind::NotFound => false,
                    io::ErrorKind::PermissionDenied => {
                        return self.escalated_swap(target, artifact);
                    }
                    _ => return Err(classify(&error, target)),
                }
            }
        };

        match rename(staged, target) {
            Ok(()) => {
                if displaced {
                    backup.retire();
                }
                Ok(())
            }
            Err(error) => {
                if displaced {
                    if let Err(rollback) = rename(&backup.bundle, target) {
                        return Err(InstallFailure::Failed {
                            detail: format!(
                                "could not replace {} ({error}). Rollback failed ({rollback}). \
                                 Previous bundle preserved at {}",
                                target.display(),
                                backup.bundle.display(),
                            ),
                        });
                    }
                    backup.clear_receipt();
                }
                Err(classify(&error, target))
            }
        }
    }

    /// Escalation re-extracts from the verified artifact in the privileged
    /// context — the user-writable staging tree is never what gets moved, and
    /// the port reports its own failure class so a terminal one stays terminal.
    fn escalated_swap(
        &self,
        target: &Path,
        artifact: &VerifiedArtifact,
    ) -> Result<(), InstallFailure> {
        self.escalate.replace(artifact, target)
    }
}

fn check_legacy_backup(target: &Path) -> Result<(), InstallFailure> {
    let backup = target.with_extension("longhorn-previous");
    if !entry_exists(target)? && entry_exists(&backup)? {
        return Err(InstallFailure::Failed {
            detail: format!(
                "{} is missing; identify and restore the legacy backup at {} before retrying",
                target.display(),
                backup.display(),
            ),
        });
    }
    Ok(())
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
///
/// Two quotas keep it finite: declared bytes, and entry count — a zero-byte
/// entry costs no bytes and still costs an inode.
fn unpack(artifact: &[u8], staging: &Path) -> Result<PathBuf, InstallFailure> {
    unpack_within(artifact, staging, ExtractionQuota::DEFAULT)
}

/// [`unpack`], against a stated quota.
fn unpack_within(
    artifact: &[u8],
    staging: &Path,
    quota: ExtractionQuota,
) -> Result<PathBuf, InstallFailure> {
    let mut archive = tar::Archive::new(GzDecoder::new(artifact));
    let entries = archive
        .entries()
        .map_err(|error| InstallFailure::MalformedArtifact {
            detail: error.to_string(),
        })?;

    let mut root: Option<PathBuf> = None;
    let mut declared_bytes: u64 = 0;
    let mut written_entries: usize = 0;
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

        // Counted before the write, alongside the byte quota: an entry that
        // costs no bytes still costs an inode.
        written_entries += 1;
        if written_entries > quota.entries {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!(
                    "archive exceeds the {}-entry extraction quota",
                    quota.entries
                ),
            });
        }

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
        // The shared archive rule refuses absolute paths and parent
        // components, including in relative link targets.
        if kind.is_symlink() || kind.is_hard_link() {
            let target = entry
                .link_name()
                .map_err(|error| InstallFailure::MalformedArtifact {
                    detail: error.to_string(),
                })?
                .ok_or_else(|| InstallFailure::MalformedArtifact {
                    detail: format!("archive link has no target: {}", path.display()),
                })?;
            let target = bounded(&target)?;
            // Hard links resolve from the archive root and must name an entry
            // already extracted. Symlinks may legitimately dangle.
            if kind.is_hard_link() {
                let link_target = staging.join(&target);
                match fs::symlink_metadata(&link_target) {
                    Ok(metadata) if !metadata.is_dir() => {}
                    Ok(_) => {
                        return Err(InstallFailure::MalformedArtifact {
                            detail: format!(
                                "archive hard link target is a directory: {}",
                                target.display()
                            ),
                        });
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        return Err(InstallFailure::MalformedArtifact {
                            detail: format!(
                                "archive hard link {} names a missing target: {}",
                                path.display(),
                                target.display()
                            ),
                        });
                    }
                    Err(error) => return Err(classify(&error, &link_target)),
                }
            }
        }

        let mut components = safe.components();
        let first = components
            .next()
            .ok_or_else(|| InstallFailure::MalformedArtifact {
                detail: "archive entry has no top-level component".to_owned(),
            })?;
        let entry_root = staging.join(first);
        if root.as_ref().is_some_and(|root| *root != entry_root) {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!(
                    "archive has more than one top-level entry: {}",
                    path.display()
                ),
            });
        }
        if components.next().is_none() && !kind.is_dir() {
            return Err(InstallFailure::MalformedArtifact {
                detail: "archive root is not a directory".to_owned(),
            });
        }
        root.get_or_insert(entry_root);

        // The declared sizes are the archive's own claim — the only bound
        // available before a byte is written. A header understating its
        // payload desyncs the entry stream and fails as malformed.
        declared_bytes = declared_bytes.saturating_add(entry.header().size().map_err(|error| {
            InstallFailure::MalformedArtifact {
                detail: error.to_string(),
            }
        })?);
        if declared_bytes > quota.bytes {
            return Err(InstallFailure::MalformedArtifact {
                detail: format!("archive exceeds the {}-byte extraction quota", quota.bytes),
            });
        }

        let destination = staging.join(&safe);
        assert_inside(staging, &destination)?;
        // `unpack_in` creates parents and re-validates canonically. A `false`
        // is tar skipping an entry it considers unsafe; after `bounded` that
        // should be unreachable, and it is refused rather than trusted.
        if !entry
            .unpack_in(staging)
            .map_err(|error| classify_entry(&error, &destination, artifact))?
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
/// link targets alike. The rule is `longhorn-archive-path`'s, shared with
/// `longhorn-tool-provision` (contract 027), so the two cannot drift.
fn bounded(path: &Path) -> Result<PathBuf, InstallFailure> {
    longhorn_archive_path::bounded(path).map_err(|refusal| InstallFailure::MalformedArtifact {
        detail: refusal.to_string(),
    })
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

/// [`classify`] for a failure inside `unpack_in`, which reads and writes in
/// one call.
///
/// Tar's error kinds do not say which side failed: a payload that ends before
/// its header declares is an `Other` fault, a broken deflate stream is
/// `InvalidData`, and a write to staging carries the filesystem's own kind.
/// The contract needs the answer, because the two are not the same outcome —
/// malformed bytes are terminal and discarded, a replacement fault is
/// retryable, and reporting a truncated archive as retryable replays the same
/// bytes on every attempt.
///
/// So the archive is judged on its own: it is already in memory, and reading
/// it again without a destination costs a second decode only after an
/// extraction has failed. Bytes that read to the end of every declared
/// payload are intact, and the fault was staging's.
fn classify_entry(error: &std::io::Error, path: &Path, artifact: &[u8]) -> InstallFailure {
    if error.kind() == std::io::ErrorKind::PermissionDenied || archive_is_intact(artifact) {
        return classify(error, path);
    }
    InstallFailure::MalformedArtifact {
        detail: format!("{}: {error}", path.display()),
    }
}

/// Returns whether every entry's declared payload is present in `artifact`.
///
/// A tar stream that ends early is not an error to `Read`: the copy simply
/// returns fewer bytes, which is why tar compares the copy against the size
/// the entry declares. This does the same comparison with no destination, so
/// a truncated archive is recognised as input rather than as a fault of the
/// directory it was unpacked into. The size comes from [`tar::Entry::size`],
/// not the raw header field, so a header format that stores it elsewhere (a
/// pax extension) is compared against what the reader is actually bounded by.
fn archive_is_intact(artifact: &[u8]) -> bool {
    let mut archive = tar::Archive::new(GzDecoder::new(artifact));
    let Ok(entries) = archive.entries() else {
        return false;
    };
    for entry in entries {
        let Ok(mut entry) = entry else {
            return false;
        };
        let declared = entry.size();
        if !io::copy(&mut entry, &mut io::sink()).is_ok_and(|read| read == declared) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use flate2::{Compression, write::GzEncoder};
    use longhorn_update::{ArtifactKey, verify_artifact};
    use minisign::KeyPair;

    use super::*;

    /// A bundle-shaped archive of `count` empty files under one root.
    fn archive_of(count: usize) -> Vec<u8> {
        let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        for index in 0..count {
            let mut header = tar::Header::new_gnu();
            header.set_size(0);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append_data(
                    &mut header,
                    format!("Example.app/{index}"),
                    std::io::empty(),
                )
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    /// A gzip tar whose one entry declares more bytes than the stream carries.
    ///
    /// The gzip member itself is complete, so nothing but the tar structure is
    /// wrong: exactly what an interrupted transfer leaves.
    fn truncated_archive() -> Vec<u8> {
        use std::io::Write as _;

        let payload = vec![b'x'; 4096];
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder
            .append_data(&mut header, "Example.app/0", &payload[..])
            .unwrap();
        let mut plain = builder.into_inner().unwrap();
        plain.truncate(plain.len() - 3000);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&plain).unwrap();
        encoder.finish().unwrap()
    }

    fn signed_archive() -> VerifiedArtifact {
        sign_archive(archive_of(1))
    }

    fn sign_archive(bytes: Vec<u8>) -> VerifiedArtifact {
        let key = KeyPair::generate_unencrypted_keypair().unwrap();
        let public_key = ArtifactKey::from_key_file(&key.pk.to_box().unwrap().to_string()).unwrap();
        let signature = minisign::sign(None, &key.sk, io::Cursor::new(&bytes), None, None)
            .unwrap()
            .to_string();
        verify_artifact(
            &public_key,
            &semver::Version::new(1, 0, 0),
            bytes,
            &signature,
        )
        .unwrap()
    }

    #[cfg(unix)]
    fn existing_install(root: &Path) -> NativeInstaller<NoPrivilegedReplace> {
        let installer = NativeInstaller::new(root.canonicalize().unwrap().join("Example.app"));
        fs::create_dir(installer.target()).unwrap();
        fs::write(installer.target().join("original"), b"old").unwrap();
        installer
    }

    #[cfg(not(unix))]
    #[test]
    fn apply_refuses_to_replace_a_bundle_without_directory_locking() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("Example.app");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("original"), b"old").unwrap();

        let error = NativeInstaller::new(&target)
            .apply(&signed_archive())
            .expect_err("native replacement requires directory locks");

        assert!(matches!(error, InstallFailure::NotWritable { .. }));
        assert_eq!(fs::read(target.join("original")).unwrap(), b"old");
    }

    #[cfg(unix)]
    fn fail_swap_and_rollback(
        installer: &NativeInstaller<NoPrivilegedReplace>,
        artifact: &VerifiedArtifact,
    ) -> InstallFailure {
        installer
            .apply_with_rename(artifact, |from, to| {
                if from == installer.target() {
                    return fs::rename(from, to);
                }
                if from.file_name().is_some_and(|name| name == "previous") {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected rollback failure",
                    ));
                }
                Err(io::Error::other("injected swap failure"))
            })
            .expect_err("both renames into the target failed")
    }

    #[cfg(unix)]
    #[test]
    fn an_install_leaves_another_targets_live_staging_intact() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let artifact = signed_archive();
        let first = NativeInstaller::new(root.join("release/Example.app"));
        let second = NativeInstaller::new(root.join("preview/Example.app"));
        let sibling = NativeInstaller::new(root.join("release/Example.preview"));
        fs::create_dir_all(first.target()).unwrap();
        fs::create_dir_all(second.target()).unwrap();

        first
            .apply_with_rename(&artifact, |from, to| {
                if to == first.target() {
                    for overlapping in [&first, &sibling] {
                        let error = overlapping
                            .apply(&artifact)
                            .expect_err("the parent lock is held");
                        assert!(error.to_string().contains("another update is active"));
                    }
                    second
                        .apply(&artifact)
                        .expect("an install in a separate branch can proceed");
                    assert!(from.join("0").is_file(), "active staging was touched");
                }
                fs::rename(from, to)
            })
            .expect("first target still installs");

        assert!(first.target().join("0").is_file());
        assert!(second.target().join("0").is_file());
        assert!(!sibling.target().exists());
    }

    #[cfg(unix)]
    #[test]
    fn an_ancestor_install_cannot_remove_a_live_descendant_workspace() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("parent");
        fs::create_dir(&parent).unwrap();
        let outer = existing_install(&parent);
        let inner = NativeInstaller::new(outer.target().join("Inner.app"));
        fs::create_dir(inner.target()).unwrap();
        let alias = directory.path().join("alias");
        symlink(&parent, &alias).unwrap();
        let outer_via_alias = NativeInstaller::new(alias.join("Example.app"));
        let artifact = signed_archive();

        inner
            .apply_with_rename(&artifact, |from, to| {
                if to.ends_with("Inner.app") {
                    let workspace = from.parent().unwrap().parent().unwrap();
                    for ancestor in [&outer, &outer_via_alias] {
                        let error = ancestor
                            .apply(&artifact)
                            .expect_err("the descendant update is active");
                        assert!(error.to_string().contains("another update is active"));
                    }
                    assert!(workspace.join("previous").is_dir(), "backup was touched");
                    assert!(workspace.join("longhorn-update-install-v1.lock").is_file());
                    assert!(from.join("0").is_file(), "active staging was touched");
                }
                fs::rename(from, to)
            })
            .expect("the descendant can still publish its staged bundle");

        assert!(inner.target().join("0").is_file());
        assert!(outer.target().join("original").is_file());
        outer_via_alias
            .apply(&artifact)
            .expect("the ancestor can install after the descendant finishes");
        assert!(outer.target().join("0").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_live_workspace_and_its_staged_bundle_cannot_be_install_targets() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();

        installer
            .apply_with_rename(&artifact, |from, to| {
                if to == installer.target() {
                    let stage = from.parent().unwrap();
                    let workspace = stage.parent().unwrap();
                    let assert_reserved = |target: &Path| {
                        let error = NativeInstaller::new(target)
                            .apply(&artifact)
                            .expect_err("the live workspace is reserved");
                        assert!(
                            error
                                .to_string()
                                .contains("overlaps the workspace namespace")
                        );
                    };
                    assert_reserved(workspace);
                    assert_reserved(from);
                    let case_alias = workspace
                        .with_file_name(workspace.file_name().unwrap().to_ascii_uppercase());
                    assert_reserved(&case_alias);
                    let alias = directory.path().join("workspace-alias");
                    std::os::unix::fs::symlink(workspace, &alias).unwrap();
                    let nested = alias.join(stage.file_name().unwrap()).join("Example.app");
                    assert_reserved(&alias);
                    assert_reserved(&nested);
                    assert!(from.join("0").is_file(), "active staging was touched");
                }
                fs::rename(from, to)
            })
            .expect("the original update can still publish its staged bundle");

        assert!(installer.target().join("0").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn failed_rollback_reports_both_faults_and_leaves_the_bundle_for_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let artifact = signed_archive();
        let installer = existing_install(directory.path());
        let error = fail_swap_and_rollback(&installer, &artifact);
        let backup = installer
            .target()
            .parent()
            .unwrap()
            .join(".--.longhorn-install-Example.app/previous");

        assert!(matches!(error, InstallFailure::Failed { .. }));
        let detail = error.to_string();
        assert!(detail.contains("injected swap failure"));
        assert!(detail.contains("Rollback failed (injected rollback failure)"));
        assert!(detail.contains(&*backup.to_string_lossy()));
        assert_eq!(fs::read(backup.join("original")).unwrap(), b"old");
        assert!(!installer.target().exists());

        installer
            .apply(&artifact)
            .expect("retry recovers and replaces");
        assert!(installer.target().join("0").is_file());
        assert!(!backup.exists());
    }

    #[test]
    #[cfg(unix)]
    fn a_displaced_install_is_restored_even_when_the_new_artifact_fails() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        drop(fail_swap_and_rollback(&installer, &signed_archive()));

        let error = installer
            .apply(&sign_archive(Vec::new()))
            .expect_err("extraction should fail after recovery");

        assert!(matches!(error, InstallFailure::MalformedArtifact { .. }));
        assert_eq!(
            fs::read(installer.target().join("original")).unwrap(),
            b"old"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_occupied_target_keeps_the_displaced_bundle_for_inspection() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        drop(fail_swap_and_rollback(&installer, &artifact));
        fs::create_dir(installer.target()).unwrap();
        let backup = installer
            .target()
            .parent()
            .unwrap()
            .join(".--.longhorn-install-Example.app/previous");

        let error = installer.apply(&artifact).expect_err("both paths exist");

        assert!(error.to_string().contains(&*backup.to_string_lossy()));
        assert_eq!(fs::read(backup.join("original")).unwrap(), b"old");
        assert_eq!(fs::read_dir(installer.target()).unwrap().count(), 0);
    }

    #[test]
    #[cfg(unix)]
    fn a_backup_receipt_cannot_authorize_a_different_directory() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        drop(fail_swap_and_rollback(&installer, &artifact));
        let backup = directory
            .path()
            .join(".--.longhorn-install-Example.app/previous");
        let original = directory.path().join("saved-original");
        fs::rename(&backup, &original).unwrap();
        fs::create_dir(&backup).unwrap();
        fs::write(backup.join("unrelated"), b"keep").unwrap();

        let error = installer
            .apply(&artifact)
            .expect_err("the identity changed");

        assert!(
            error
                .to_string()
                .contains("could not verify the previous bundle")
        );
        assert_eq!(fs::read(backup.join("unrelated")).unwrap(), b"keep");
        assert_eq!(fs::read(original.join("original")).unwrap(), b"old");
        assert!(!installer.target().exists());
    }

    #[test]
    #[cfg(unix)]
    fn moving_a_workspace_does_not_transfer_its_backup_to_another_target() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        drop(fail_swap_and_rollback(&installer, &artifact));
        let other_parent = directory.path().join("other-parent");
        fs::create_dir(&other_parent).unwrap();
        let workspace = other_parent.join(".--.longhorn-install-Example.app");
        fs::rename(
            directory.path().join(".--.longhorn-install-Example.app"),
            &workspace,
        )
        .unwrap();
        let other = NativeInstaller::new(other_parent.join("Example.app"));

        let error = other
            .apply(&artifact)
            .expect_err("the target parent changed");

        assert!(
            error
                .to_string()
                .contains("could not verify the previous bundle")
        );
        assert_eq!(
            fs::read(workspace.join("previous/original")).unwrap(),
            b"old"
        );
        assert!(!other.target().exists());
    }

    /// Empty entries declare no bytes, so only the entry quota bounds them.
    #[test]
    fn the_entry_quota_refuses_an_archive_of_zero_byte_entries() {
        let staging = tempfile::tempdir().unwrap();
        let quota = ExtractionQuota {
            bytes: MAX_EXTRACTED_BYTES,
            entries: 8,
        };

        let outcome = unpack_within(&archive_of(9), staging.path(), quota);

        assert!(
            matches!(outcome, Err(InstallFailure::MalformedArtifact { .. })),
            "expected an entry-quota refusal, found {outcome:?}"
        );
    }

    /// The boundary is inclusive: exactly the quota unpacks.
    #[test]
    fn an_archive_at_the_entry_quota_unpacks() {
        let staging = tempfile::tempdir().unwrap();
        let quota = ExtractionQuota {
            bytes: MAX_EXTRACTED_BYTES,
            entries: 8,
        };

        unpack_within(&archive_of(8), staging.path(), quota).expect("unpack at the quota");
    }

    /// Tar reports a short payload and a short write the same way, so the
    /// class comes from reading the archive, not from tar's error kind.
    #[test]
    fn an_extraction_failure_is_malformed_only_when_the_archive_is_broken() {
        let intact = archive_of(1);
        let broken = truncated_archive();
        let path = Path::new("Example.app/0");
        let short_write = io::Error::other("failed to write entire file");

        assert!(archive_is_intact(&intact));
        assert!(!archive_is_intact(&broken));
        assert!(matches!(
            classify_entry(&short_write, path, &intact),
            InstallFailure::Failed { .. }
        ));
        // The same fault against a short stream is malformed input: terminal,
        // discarded, never retried with the same bytes.
        assert!(matches!(
            classify_entry(&short_write, path, &broken),
            InstallFailure::MalformedArtifact { .. }
        ));
        assert!(matches!(
            classify_entry(
                &io::Error::from(io::ErrorKind::PermissionDenied),
                path,
                &intact
            ),
            InstallFailure::NotWritable { .. }
        ));
    }

    /// A cleanup that fails after publication does not turn a completed install
    /// into a reported failure: the new bundle reached the target, and the next
    /// apply reclaims the leftover instead of refusing to start forever.
    #[cfg(unix)]
    #[test]
    fn a_failed_cleanup_stays_applied_and_the_next_apply_reclaims_the_leftover() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        let workspace = directory
            .path()
            .canonicalize()
            .unwrap()
            .join(".--.longhorn-install-Example.app");
        let previous = workspace.join("previous");
        let locked = previous.join("locked");

        let applied = installer
            .apply_with_rename(&artifact, |from, to| {
                fs::rename(from, to)?;
                if to == previous {
                    // A directory the removal cannot empty: the swap itself
                    // still succeeds.
                    fs::create_dir(&locked)?;
                    fs::write(locked.join("child"), b"kept")?;
                    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555))?;
                }
                Ok(())
            })
            .expect("a cleanup fault does not fail an install that reached disk");

        assert_eq!(applied.version, artifact.version().clone());
        // The new bundle is in place; the displaced one is still here, and the
        // marker says which kind of leftover it is.
        assert!(installer.target().join("0").is_file());
        assert!(previous.join("locked/child").is_file());
        assert!(workspace.join("previous.published").is_file());

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        installer
            .apply(&artifact)
            .expect("the next apply reclaims the leftover and installs");

        assert!(installer.target().join("0").is_file());
        assert!(!previous.exists());
        assert!(!workspace.join("previous.identity").exists());
        assert!(!workspace.join("previous.published").exists());
    }

    /// A leftover a *later* apply cannot reclaim is reported by that apply,
    /// before it touches the install.
    #[cfg(unix)]
    #[test]
    fn an_unreclaimable_leftover_refuses_the_next_apply() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        let workspace = directory
            .path()
            .canonicalize()
            .unwrap()
            .join(".--.longhorn-install-Example.app");
        let locked = workspace.join("previous/locked");

        installer
            .apply_with_rename(&artifact, |from, to| {
                fs::rename(from, to)?;
                if to == workspace.join("previous") {
                    fs::create_dir(&locked)?;
                    fs::write(locked.join("child"), b"kept")?;
                    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555))?;
                }
                Ok(())
            })
            .expect("the swap succeeds");

        let error = installer
            .apply(&artifact)
            .expect_err("the leftover cannot be reclaimed");

        assert!(matches!(error, InstallFailure::Failed { .. }));
        assert!(error.to_string().contains("could not reclaim"), "{error}");
        // Nothing was displaced: the install this apply found is untouched.
        assert!(installer.target().join("0").is_file());
        assert!(locked.join("child").is_file());
    }

    /// A bundle a failed rollback left for recovery keeps its identity and is
    /// never reclaimed, even once the target is occupied again.
    #[cfg(unix)]
    #[test]
    fn a_rollback_bundle_is_not_reclaimed_when_the_target_is_occupied() {
        let directory = tempfile::tempdir().unwrap();
        let installer = existing_install(directory.path());
        let artifact = signed_archive();
        drop(fail_swap_and_rollback(&installer, &artifact));
        fs::create_dir(installer.target()).unwrap();

        let error = installer
            .apply(&artifact)
            .expect_err("a recovery bundle needs a human");

        assert!(error.to_string().contains("Inspect and move"), "{error}");
        assert!(
            installer
                .target()
                .parent()
                .unwrap()
                .join(".--.longhorn-install-Example.app/previous/original")
                .is_file()
        );
    }
}
