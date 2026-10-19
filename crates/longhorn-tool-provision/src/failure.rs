//! Failure classes (contract 027).
//!
//! A refusal is a verdict on the artifact: the same bytes refuse again. A
//! fault is a host problem: the bytes may be fine and a retry may succeed.
//! Callers match on the class, never on the message.

use std::{fmt, path::PathBuf};

use longhorn_archive_path::PathRefusal;

use crate::verify::DigestOrigin;

/// Why an artifact or its executable was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refusal {
    /// The caller declared neither a digest nor a vendor team.
    NothingDeclared {
        /// The asset the caller asked about.
        asset: String,
    },
    /// The release checksum file has no entry for the asset.
    ChecksumNotCovered {
        /// The asset the checksum file does not list.
        asset: String,
    },
    /// A checksum file lists the asset more than once, or the entry for it
    /// is not a SHA-256 digest.
    MalformedChecksum {
        /// The asset whose entry is malformed or ambiguous.
        asset: String,
    },
    /// A `.sha256` sidecar does not name the asset in the required format.
    MalformedSidecar {
        /// The asset the sidecar should name.
        asset: String,
    },
    /// The pinned digest is not a lowercase SHA-256 digest.
    MalformedPin {
        /// The asset the pin covers.
        asset: String,
    },
    /// The artifact's bytes do not hash to the declared digest.
    DigestMismatch {
        /// The asset that failed.
        asset: String,
        /// Where the declared digest came from.
        origin: DigestOrigin,
    },
    /// The archive cannot be read (corrupt, truncated, or encrypted).
    UnreadableArchive {
        /// The asset that failed.
        asset: String,
        /// The reader's own description.
        detail: String,
    },
    /// The archive holds more members than the extraction limit.
    TooManyMembers {
        /// The asset that failed.
        asset: String,
        /// The member limit that was exceeded.
        limit: usize,
    },
    /// The archive's real decompressed bytes exceed the extraction limit.
    ArchiveTooLarge {
        /// The asset that failed.
        asset: String,
        /// The byte limit that was exceeded.
        limit: u64,
    },
    /// An entry name escapes the destination, is empty, or uses a backslash.
    UnsafeEntry {
        /// The asset that failed.
        asset: String,
        /// The entry name as the archive wrote it.
        name: String,
        /// The path rule's reason, when the shared rule refused the name.
        reason: Option<PathRefusal>,
    },
    /// A link entry (symbolic or hard). Links are refused, never followed.
    LinkEntry {
        /// The asset that failed.
        asset: String,
        /// The link's entry name.
        name: String,
    },
    /// An entry that is neither a file nor a directory (device, fifo).
    UnsupportedEntry {
        /// The asset that failed.
        asset: String,
        /// The entry's name.
        name: String,
    },
    /// The archive holds the requested member more than once.
    DuplicateMember {
        /// The asset that failed.
        asset: String,
        /// The member name.
        member: String,
    },
    /// The archive does not contain the requested executable.
    MemberMissing {
        /// The asset that failed.
        asset: String,
        /// The member the caller asked for.
        member: String,
    },
    /// The executable carries a vendor team other than the expected one.
    TeamMismatch {
        /// The team the signature names.
        found: String,
        /// The team the caller declared.
        expected: String,
    },
    /// The executable has no signature that verifies, or no team identifier.
    Unsigned {
        /// The team the caller declared.
        expected: String,
        /// The verifier's reason.
        detail: String,
    },
    /// This platform has no vendor-signature adapter, so a declared team
    /// cannot be checked and the executable is refused.
    SignatureUnsupported {
        /// The operating system the adapter does not cover.
        platform: String,
    },
}

/// A host-side problem. The artifact may be fine.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fault {
    /// A file could not be opened, read or written.
    Io {
        /// The file involved.
        path: PathBuf,
        /// The operating system's description.
        detail: String,
    },
    /// The signature verifier could not run (for example, codesign is
    /// missing). Unlike [`Refusal::Unsigned`], the signature was not judged.
    Verifier {
        /// The verifier's description.
        detail: String,
    },
    /// The caller passed a value this crate cannot use (for example, a
    /// staged name with a directory part). A bug in the caller, not an
    /// artifact problem.
    Usage {
        /// What was wrong.
        detail: String,
    },
}

/// Every failure this crate reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProvisionError {
    /// The artifact or executable was refused.
    Refused(Refusal),
    /// The host could not complete the check.
    Fault(Fault),
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NothingDeclared { asset } => write!(
                formatter,
                "{asset} has neither a digest nor a vendor signature; it cannot be verified"
            ),
            Self::ChecksumNotCovered { asset } => write!(
                formatter,
                "the release checksum file does not cover {asset}; the artifact cannot be verified"
            ),
            Self::MalformedChecksum { asset } => write!(
                formatter,
                "the checksum entry for {asset} is missing its digest, is not SHA-256, or appears twice"
            ),
            Self::MalformedSidecar { asset } => write!(
                formatter,
                "the SHA-256 sidecar does not name {asset} in the required format"
            ),
            Self::MalformedPin { asset } => write!(
                formatter,
                "the pinned digest for {asset} is not a lowercase SHA-256 digest"
            ),
            Self::DigestMismatch { asset, origin } => write!(
                formatter,
                "the artifact digest did not match the {} for {asset}",
                origin.describe()
            ),
            Self::UnreadableArchive { asset, detail } => {
                write!(formatter, "{asset} is not a readable archive: {detail}")
            }
            Self::TooManyMembers { asset, limit } => write!(
                formatter,
                "{asset} holds more than {limit} members and cannot be unpacked safely"
            ),
            Self::ArchiveTooLarge { asset, limit } => write!(
                formatter,
                "{asset} unpacks to more than {limit} bytes, the accepted bound"
            ),
            Self::UnsafeEntry {
                asset,
                name,
                reason,
            } => match reason {
                Some(reason) => write!(formatter, "{asset} holds an unsafe entry: {reason}"),
                None => write!(
                    formatter,
                    "{asset} holds an entry with a backslash in its name: {name}"
                ),
            },
            Self::LinkEntry { asset, name } => {
                write!(formatter, "{asset} holds a link entry, refused: {name}")
            }
            Self::UnsupportedEntry { asset, name } => write!(
                formatter,
                "{asset} holds an entry that is neither a file nor a directory: {name}"
            ),
            Self::DuplicateMember { asset, member } => write!(
                formatter,
                "{asset} holds the expected executable {member} more than once"
            ),
            Self::MemberMissing { asset, member } => write!(
                formatter,
                "{asset} does not contain the expected executable {member}"
            ),
            Self::TeamMismatch { found, expected } => write!(
                formatter,
                "the executable is signed by team {found}, not the expected vendor team {expected}"
            ),
            Self::Unsigned { expected, detail } => write!(
                formatter,
                "the executable carries no vendor signature; the expected vendor team is {expected}: {detail}"
            ),
            Self::SignatureUnsupported { platform } => write!(
                formatter,
                "vendor signatures are checked only on macOS, not {platform}; the executable is refused"
            ),
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, detail } => {
                write!(formatter, "{} could not be used: {detail}", path.display())
            }
            Self::Verifier { detail } => {
                write!(formatter, "the signature verifier could not run: {detail}")
            }
            Self::Usage { detail } => write!(formatter, "invalid request: {detail}"),
        }
    }
}

impl fmt::Display for ProvisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => write!(formatter, "refused: {refusal}"),
            Self::Fault(fault) => write!(formatter, "fault: {fault}"),
        }
    }
}

impl std::error::Error for Refusal {}
impl std::error::Error for Fault {}
impl std::error::Error for ProvisionError {}
