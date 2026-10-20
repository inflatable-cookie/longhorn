//! Bounded extraction of one executable (contract 027).
//!
//! Extraction takes a [`VerifiedArtifact`]. It first copies the artifact into
//! a private file in the staging directory and checks the declared digest on
//! that copy, so the bytes that get extracted are the bytes that were checked,
//! whatever happens to the source file afterwards. It then writes exactly one
//! file: the requested member, under the caller's staged name, with the mode
//! set here and nothing taken from the archive. Every entry is checked, not
//! only the member, so an archive that holds a link, a special file or an
//! escaping name anywhere is refused whole.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

use flate2::read::{DeflateDecoder, GzDecoder};
use longhorn_archive_path::bounded;
use sha2::{Digest, Sha256};

use crate::{
    budget::{BudgetReader, ByteBudgetExceeded},
    failure::{Fault, ProvisionError, Refusal},
    verify::VerifiedArtifact,
};

/// The most members and real decompressed bytes one extraction may spend.
///
/// The defaults are the donor's bounds (contract 042), kept generic: they
/// bound resources, they name no product. A caller may pass lower bounds;
/// the tests inject small ones to prove the real-byte refusals without
/// building a 4 GiB archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractionLimits {
    max_members: usize,
    max_bytes: u64,
}

impl ExtractionLimits {
    /// 20 000 members and 4 GiB of decompressed bytes.
    pub const DEFAULT: Self = Self {
        max_members: 20_000,
        max_bytes: 4 * 1024 * 1024 * 1024,
    };

    /// Limits of `max_members` entries and `max_bytes` decompressed bytes.
    pub const fn new(max_members: usize, max_bytes: u64) -> Self {
        Self {
            max_members,
            max_bytes,
        }
    }

    /// The most entries an archive may hold.
    pub const fn max_members(&self) -> usize {
        self.max_members
    }

    /// The most decompressed bytes an extraction may read.
    pub const fn max_bytes(&self) -> u64 {
        self.max_bytes
    }
}

/// The shape of a downloaded artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactShape<'a> {
    /// The download is the executable itself.
    Raw,
    /// A gzip tarball; `member` is the executable's path inside it, already
    /// expanded by the caller.
    TarGz {
        /// The executable's path inside the archive.
        member: &'a str,
    },
    /// A zip; `member` is the executable's path inside it, already expanded
    /// by the caller.
    Zip {
        /// The executable's path inside the archive.
        member: &'a str,
    },
}

/// An executable extracted from a verified artifact. It is not installable
/// until [`verify_signature`](crate::verify_signature) passes it. The path is
/// private: a caller cannot run or install it from here.
#[derive(Debug)]
pub struct StagedExecutable {
    path: PathBuf,
    vendor_team: Option<String>,
}

impl StagedExecutable {
    /// The vendor team the executable must carry, when declared.
    pub(crate) fn vendor_team(&self) -> Option<&str> {
        self.vendor_team.as_deref()
    }

    /// The staged file.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Wraps the staged file, with the team the caller declared.
    pub(crate) fn new(path: PathBuf, vendor_team: Option<String>) -> Self {
        Self { path, vendor_team }
    }
}

/// Extracts one executable from a verified artifact into `staging`.
///
/// The artifact is copied into a private file in `staging` and the declared
/// digest is checked on that copy before anything is read from it. The file
/// is written as `staging/staged_name`, created new (an existing file refuses
/// as a fault), and set to mode `0755` afterwards. The archive's own
/// permissions are never applied. The private copy is removed before this
/// returns unless cleanup fails with [`Fault::Io`].
///
/// # Errors
///
/// On failure, removal is attempted for both files owned by this call.
/// A cleanup failure is [`Fault::Io`] with the failed path and the original
/// error in its detail. Other owned files still get a cleanup attempt.
///
/// Refusals: [`Refusal::DigestMismatch`] when the copy does not hash to the
/// declared digest (the source changed after verification),
/// [`Refusal::TooManyMembers`], [`Refusal::ArchiveTooLarge`] (the real
/// decompressed bytes, not declared sizes), [`Refusal::UnsafeEntry`],
/// [`Refusal::LinkEntry`], [`Refusal::UnsupportedEntry`] (devices, fifos,
/// sockets), [`Refusal::DuplicateMember`], [`Refusal::MemberMissing`] and
/// [`Refusal::UnreadableArchive`]. Faults: host I/O, and a `staged_name` or
/// `member` this crate cannot use ([`Fault::Usage`]).
pub fn extract_executable(
    artifact: &VerifiedArtifact,
    shape: ArtifactShape<'_>,
    staging: &Path,
    staged_name: &str,
    limits: ExtractionLimits,
) -> Result<StagedExecutable, ProvisionError> {
    let destination = staged_destination(staging, staged_name)?;
    let member = match shape {
        ArtifactShape::Raw => None,
        ArtifactShape::TarGz { member } | ArtifactShape::Zip { member } => {
            Some(member_path(member)?)
        }
    };
    // Ownership starts only when create_new succeeds, never from an earlier
    // existence check: another extraction may create the file in between.
    let mut created_here = false;
    let sealed = seal(artifact, staging, staged_name)?;
    let asset = artifact.asset();
    let extracted = match (shape, member.as_deref()) {
        (ArtifactShape::TarGz { .. }, Some(member)) => extract_tar_gz(
            &sealed,
            asset,
            member,
            &destination,
            &mut created_here,
            limits,
        ),
        (ArtifactShape::Zip { .. }, Some(member)) => extract_zip(
            &sealed,
            asset,
            member,
            &destination,
            &mut created_here,
            limits,
        ),
        _ => copy_raw(&sealed, asset, &destination, &mut created_here, limits),
    };
    finish_extraction(extracted, &sealed, &destination, created_here)?;
    Ok(StagedExecutable::new(
        destination,
        artifact.vendor_team().map(str::to_owned),
    ))
}

/// Finish only after removing the private copy. Every failed outcome also
/// removes an executable this call created, even if private-copy removal failed.
fn finish_extraction(
    extracted: Result<(), ProvisionError>,
    sealed: &Path,
    destination: &Path,
    created_here: bool,
) -> Result<(), ProvisionError> {
    let finished = cleanup_owned_file(sealed, extracted).and_then(|()| set_executable(destination));
    if finished.is_err() && created_here {
        cleanup_owned_file(destination, finished)
    } else {
        finished
    }
}

/// Removal failures take precedence over the original outcome, retaining
/// its context. Call only for a path this extraction created atomically.
fn cleanup_owned_file(
    path: &Path,
    outcome: Result<(), ProvisionError>,
) -> Result<(), ProvisionError> {
    match fs::remove_file(path) {
        Ok(()) => outcome,
        Err(error) if error.kind() == io::ErrorKind::NotFound => outcome,
        Err(error) => {
            let detail = match outcome {
                Ok(()) => format!("could not remove the extraction file: {error}"),
                Err(original) => {
                    format!("could not remove the extraction file after {original}: {error}")
                }
            };
            Err(ProvisionError::Fault(Fault::Io {
                path: path.to_path_buf(),
                detail,
            }))
        }
    }
}

fn usage(detail: impl Into<String>) -> ProvisionError {
    ProvisionError::Fault(Fault::Usage {
        detail: detail.into(),
    })
}

fn io_fault(path: &Path) -> impl Fn(io::Error) -> ProvisionError + '_ {
    move |error| {
        ProvisionError::Fault(Fault::Io {
            path: path.to_path_buf(),
            detail: error.to_string(),
        })
    }
}

fn staged_destination(staging: &Path, staged_name: &str) -> Result<PathBuf, ProvisionError> {
    let mut parts = Path::new(staged_name).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(staging.join(staged_name)),
        _ => Err(usage(format!(
            "the staged name {staged_name:?} must be one file name"
        ))),
    }
}

/// The member as the rule sees it. A member the rule refuses is the caller's
/// bug, not an archive's.
fn member_path(member: &str) -> Result<PathBuf, ProvisionError> {
    if member.contains('\\') {
        return Err(usage(format!("the member {member:?} contains a backslash")));
    }
    bounded(Path::new(member)).map_err(|refusal| usage(refusal.to_string()))
}

/// Copies the artifact into a private file in `staging` and checks the
/// declared digest on the copy. The bytes hashed are the bytes returned, so
/// a change to the source after verification cannot reach extraction.
fn seal(
    artifact: &VerifiedArtifact,
    staging: &Path,
    staged_name: &str,
) -> Result<PathBuf, ProvisionError> {
    let sealed = staging.join(format!(".{staged_name}.artifact"));
    let out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&sealed)
        .map_err(io_fault(&sealed))?;
    let outcome = copy_and_check(artifact, out, &sealed);
    match outcome {
        // The copy is ours: `create_new` succeeded, so nothing else owned it.
        Err(error) => cleanup_owned_file(&sealed, Err(error)).map(|()| sealed),
        Ok(()) => Ok(sealed),
    }
}

fn copy_and_check(
    artifact: &VerifiedArtifact,
    mut out: File,
    sealed: &Path,
) -> Result<(), ProvisionError> {
    let mut source = File::open(artifact.path()).map_err(io_fault(artifact.path()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_fault(artifact.path())(error)),
        };
        hasher.update(&buffer[..read]);
        out.write_all(&buffer[..read]).map_err(io_fault(sealed))?;
    }
    out.flush().map_err(io_fault(sealed))?;
    if let Some((expected, origin)) = artifact.digest()
        && hex::encode(hasher.finalize()) != expected
    {
        return Err(ProvisionError::Refused(Refusal::DigestMismatch {
            asset: artifact.asset().to_owned(),
            origin,
        }));
    }
    Ok(())
}

/// Maps a read failure: the budget's own error is the bound, anything else
/// means the archive cannot be read.
fn archive_read_failure(asset: &str, limit: u64) -> impl Fn(&io::Error) -> ProvisionError + '_ {
    move |error| {
        if error_is_budget(error) {
            ProvisionError::Refused(Refusal::ArchiveTooLarge {
                asset: asset.to_owned(),
                limit,
            })
        } else {
            ProvisionError::Refused(Refusal::UnreadableArchive {
                asset: asset.to_owned(),
                detail: error.to_string(),
            })
        }
    }
}

fn error_is_budget(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.is::<ByteBudgetExceeded>())
}

/// The entry name, checked by the shared rule plus this crate's backslash
/// refusal. `Ok(None)` is the archive's root directory (`./`), which names
/// nothing to write and is skipped; an empty file name is refused.
fn check_name(asset: &str, name: &str, is_dir: bool) -> Result<Option<PathBuf>, ProvisionError> {
    let refuse = |reason| {
        ProvisionError::Refused(Refusal::UnsafeEntry {
            asset: asset.to_owned(),
            name: name.to_owned(),
            reason,
        })
    };
    if name.contains('\\') {
        return Err(refuse(None));
    }
    match bounded(Path::new(name)) {
        Ok(safe) => Ok(Some(safe)),
        Err(longhorn_archive_path::PathRefusal::Empty) if is_dir => Ok(None),
        Err(reason) => Err(refuse(Some(reason))),
    }
}

fn refuse_link(asset: &str, name: &str) -> ProvisionError {
    ProvisionError::Refused(Refusal::LinkEntry {
        asset: asset.to_owned(),
        name: name.to_owned(),
    })
}

/// Writes `reader` to a new file at `destination`, classifying read failures
/// through `on_read_error` and write failures as host faults.
fn copy_member<R: Read>(
    reader: &mut R,
    destination: &Path,
    created_here: &mut bool,
    on_read_error: impl Fn(&io::Error) -> ProvisionError,
) -> Result<(), ProvisionError> {
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(io_fault(destination))?;
    *created_here = true;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(on_read_error(&error)),
        };
        out.write_all(&buffer[..read])
            .map_err(io_fault(destination))?;
    }
    Ok(())
}

fn copy_raw(
    sealed: &Path,
    asset: &str,
    destination: &Path,
    created_here: &mut bool,
    limits: ExtractionLimits,
) -> Result<(), ProvisionError> {
    let source = File::open(sealed).map_err(io_fault(sealed))?;
    let mut reader = BudgetReader::new(source, limits.max_bytes());
    copy_member(&mut reader, destination, created_here, |error| {
        if error_is_budget(error) {
            ProvisionError::Refused(Refusal::ArchiveTooLarge {
                asset: asset.to_owned(),
                limit: limits.max_bytes(),
            })
        } else {
            ProvisionError::Fault(Fault::Io {
                path: sealed.to_path_buf(),
                detail: error.to_string(),
            })
        }
    })
}

fn extract_tar_gz(
    sealed: &Path,
    asset: &str,
    member: &Path,
    destination: &Path,
    created_here: &mut bool,
    limits: ExtractionLimits,
) -> Result<(), ProvisionError> {
    let file = File::open(sealed).map_err(io_fault(sealed))?;
    // The budget sits on the decompressed side, so it counts every byte the
    // tar reader consumes, members it skips included.
    let budget = BudgetReader::new(GzDecoder::new(file), limits.max_bytes());
    let mut archive = tar::Archive::new(budget);
    archive.set_preserve_permissions(false);
    let on_read = archive_read_failure(asset, limits.max_bytes());
    let entries = archive.entries().map_err(|error| on_read(&error))?;

    let mut seen = 0usize;
    let mut found = false;
    for entry in entries {
        let mut entry = entry.map_err(|error| on_read(&error))?;
        seen += 1;
        if seen > limits.max_members() {
            return Err(too_many(asset, limits));
        }
        let kind = entry.header().entry_type();
        if kind.is_pax_global_extensions() {
            continue;
        }
        let name = entry
            .path()
            .map_err(|error| on_read(&error))?
            .to_string_lossy()
            .into_owned();
        let Some(safe) = check_name(asset, &name, kind.is_dir())? else {
            continue;
        };
        if kind.is_symlink() || kind.is_hard_link() {
            return Err(refuse_link(asset, &name));
        }
        if !(kind.is_file() || kind.is_dir()) {
            return Err(ProvisionError::Refused(Refusal::UnsupportedEntry {
                asset: asset.to_owned(),
                name,
            }));
        }
        if kind.is_file() && safe == member {
            if found {
                return Err(duplicate(asset, member));
            }
            found = true;
            copy_member(&mut entry, destination, created_here, &on_read)?;
        }
    }
    if found {
        Ok(())
    } else {
        Err(missing(asset, member))
    }
}

/// The Unix file-type bits of a zip entry's mode (`st_mode & S_IFMT`).
const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;

fn extract_zip(
    sealed: &Path,
    asset: &str,
    member: &Path,
    destination: &Path,
    created_here: &mut bool,
    limits: ExtractionLimits,
) -> Result<(), ProvisionError> {
    let file = File::open(sealed).map_err(io_fault(sealed))?;
    let unreadable = |error: zip::result::ZipError| {
        ProvisionError::Refused(Refusal::UnreadableArchive {
            asset: asset.to_owned(),
            detail: error.to_string(),
        })
    };
    let mut archive = zip::ZipArchive::new(file).map_err(unreadable)?;

    let mut found = false;
    for index in 0..archive.len() {
        if index + 1 > limits.max_members() {
            return Err(too_many(asset, limits));
        }
        // The raw entry is the stored stream: nothing is decompressed until the
        // member is extracted, and the header's uncompressed size is never read.
        let mut entry = archive.by_index_raw(index).map_err(unreadable)?;
        let name = entry.name().map_err(unreadable)?.into_owned();
        // The type is the mode's, never the name's: a trailing slash on a
        // regular file does not make it a directory.
        let file_type = entry.unix_mode().map(|mode| mode & S_IFMT);
        let is_dir = file_type == Some(S_IFDIR);
        let Some(safe) = check_name(asset, &name, is_dir)? else {
            continue;
        };
        match file_type {
            Some(S_IFLNK) => return Err(refuse_link(asset, &name)),
            // A zip without Unix attributes carries no file type: a file.
            None | Some(0 | S_IFREG | S_IFDIR) => {}
            Some(_) => {
                return Err(ProvisionError::Refused(Refusal::UnsupportedEntry {
                    asset: asset.to_owned(),
                    name,
                }));
            }
        }
        // Checked on every entry, so an archive with an encrypted or
        // undecodable member anywhere is refused whole.
        if entry.encrypted() {
            return Err(ProvisionError::Refused(Refusal::UnreadableArchive {
                asset: asset.to_owned(),
                detail: format!("the encrypted member {name:?} is not supported"),
            }));
        }
        let deflated = match entry.compression() {
            zip::CompressionMethod::Stored => false,
            zip::CompressionMethod::Deflated => true,
            method => {
                return Err(ProvisionError::Refused(Refusal::UnreadableArchive {
                    asset: asset.to_owned(),
                    detail: format!("compression method {method:?} of {name:?} is not supported"),
                }));
            }
        };
        if is_dir || safe != member {
            continue;
        }
        if found {
            return Err(duplicate(asset, member));
        }
        found = true;
        let on_read = archive_read_failure(asset, limits.max_bytes());
        // Decoded here, so the budget counts the bytes this extraction
        // produces, whatever the header claims.
        if deflated {
            let decoder = DeflateDecoder::new(&mut entry);
            let mut budget = BudgetReader::new(decoder, limits.max_bytes());
            copy_member(&mut budget, destination, created_here, &on_read)?;
        } else {
            let mut budget = BudgetReader::new(&mut entry, limits.max_bytes());
            copy_member(&mut budget, destination, created_here, &on_read)?;
        }
    }
    if found {
        Ok(())
    } else {
        Err(missing(asset, member))
    }
}

fn too_many(asset: &str, limits: ExtractionLimits) -> ProvisionError {
    ProvisionError::Refused(Refusal::TooManyMembers {
        asset: asset.to_owned(),
        limit: limits.max_members(),
    })
}

fn duplicate(asset: &str, member: &Path) -> ProvisionError {
    ProvisionError::Refused(Refusal::DuplicateMember {
        asset: asset.to_owned(),
        member: member.display().to_string(),
    })
}

fn missing(asset: &str, member: &Path) -> ProvisionError {
    ProvisionError::Refused(Refusal::MemberMissing {
        asset: asset.to_owned(),
        member: member.display().to_string(),
    })
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), ProvisionError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(io_fault(path))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), ProvisionError> {
    // File modes carry no executable bit on this platform.
    Ok(())
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;

    fn archive_refusal() -> ProvisionError {
        ProvisionError::Refused(Refusal::LinkEntry {
            asset: "tool.tar.gz".to_owned(),
            name: "link".to_owned(),
        })
    }

    fn cleanup_fault(error: ProvisionError, failed_path: &Path) -> String {
        match error {
            ProvisionError::Fault(Fault::Io { path, detail }) => {
                assert_eq!(path, failed_path);
                detail
            }
            error => panic!("expected a cleanup I/O fault, got {error}"),
        }
    }

    // Fixture-local directories force remove_file to fail on every platform,
    // without depending on user IDs or filesystem permission enforcement.
    #[test]
    fn a_staged_cleanup_failure_preserves_the_refusal_and_removes_the_copy() {
        let temp = tempfile::tempdir().unwrap();
        let sealed = temp.path().join(".tool.artifact");
        let staged = temp.path().join("tool");
        fs::write(&sealed, b"artifact").unwrap();
        fs::create_dir(&staged).unwrap();
        let original = archive_refusal();

        let error = finish_extraction(Err(original.clone()), &sealed, &staged, true).unwrap_err();
        assert!(cleanup_fault(error, &staged).contains(&original.to_string()));
        assert!(!sealed.exists(), "the private copy was still cleaned up");
        assert!(staged.is_dir(), "the failed path is reported to the caller");
    }

    #[test]
    fn a_copy_cleanup_failure_is_not_hidden_by_an_extraction_refusal() {
        let temp = tempfile::tempdir().unwrap();
        let sealed = temp.path().join(".tool.artifact");
        let staged = temp.path().join("tool");
        fs::create_dir(&sealed).unwrap();
        fs::write(&staged, b"partial executable").unwrap();
        let original = archive_refusal();

        let error = finish_extraction(Err(original.clone()), &sealed, &staged, true).unwrap_err();
        assert!(cleanup_fault(error, &sealed).contains(&original.to_string()));
        assert!(
            !staged.exists(),
            "staged cleanup runs after copy cleanup fails"
        );
    }

    #[test]
    fn both_cleanup_failures_retain_both_paths_and_the_original_refusal() {
        let temp = tempfile::tempdir().unwrap();
        let sealed = temp.path().join(".tool.artifact");
        let staged = temp.path().join("tool");
        fs::create_dir(&sealed).unwrap();
        fs::create_dir(&staged).unwrap();
        let original = archive_refusal();

        let error = finish_extraction(Err(original.clone()), &sealed, &staged, true).unwrap_err();
        let detail = cleanup_fault(error, &staged);
        assert!(detail.contains(&sealed.display().to_string()));
        assert!(detail.contains(&original.to_string()));
    }

    #[test]
    fn a_copy_cleanup_failure_also_removes_a_successfully_extracted_binary() {
        let temp = tempfile::tempdir().unwrap();
        let sealed = temp.path().join(".tool.artifact");
        let staged = temp.path().join("tool");
        fs::create_dir(&sealed).unwrap();
        fs::write(&staged, b"executable").unwrap();

        let error = finish_extraction(Ok(()), &sealed, &staged, true).unwrap_err();
        cleanup_fault(error, &sealed);
        assert!(!staged.exists(), "cleanup failure cannot finish extraction");
    }

    #[test]
    fn sealing_cleanup_failure_preserves_the_copy_fault() {
        let temp = tempfile::tempdir().unwrap();
        let sealed = temp.path().join(".tool.artifact");
        fs::create_dir(&sealed).unwrap();
        let original = ProvisionError::Fault(Fault::Io {
            path: temp.path().join("source"),
            detail: "scripted copy failure".to_owned(),
        });

        let error = cleanup_owned_file(&sealed, Err(original.clone())).unwrap_err();
        assert!(cleanup_fault(error, &sealed).contains(&original.to_string()));
    }

    #[test]
    fn an_already_removed_file_preserves_the_original_failure() {
        let temp = tempfile::tempdir().unwrap();
        let original = archive_refusal();
        assert_eq!(
            cleanup_owned_file(&temp.path().join("absent"), Err(original.clone())),
            Err(original)
        );
    }
}
