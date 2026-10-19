//! Artifact verification (contract 027).
//!
//! A downloaded artifact becomes a [`VerifiedArtifact`] only when every
//! declared check passes. The type is the gate: extraction takes a
//! `&VerifiedArtifact`, so bytes cannot reach an extracted executable
//! without the checks the caller declared.

use std::path::{Path, PathBuf};

use crate::{
    digest::{digest_from_checksums, digest_from_sidecar, is_sha256_hex, sha256_file},
    failure::{ProvisionError, Refusal},
};

/// Where a declared digest came from. Sidecars have a stricter format than
/// multi-asset checksum files, so the origin is named in refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestOrigin {
    /// A release checksum file that lists many assets.
    ReleaseChecksums,
    /// An artifact-specific `.sha256` sidecar.
    Sidecar,
    /// A digest the caller pinned after review.
    Pinned,
}

impl DigestOrigin {
    /// The noun a refusal names for this origin.
    pub const fn describe(self) -> &'static str {
        match self {
            Self::ReleaseChecksums => "release checksum",
            Self::Sidecar => "SHA-256 sidecar",
            Self::Pinned => "pinned digest",
        }
    }
}

/// A digest the caller declares for an artifact, with the text it came from.
#[derive(Clone, Copy, Debug)]
pub enum DigestSource<'a> {
    /// The release's checksum file, read as text.
    ReleaseChecksums(&'a str),
    /// The artifact's `.sha256` sidecar, read as text.
    Sidecar(&'a str),
    /// A lowercase SHA-256 digest pinned in the caller's catalogue.
    Pinned(&'a str),
}

/// What the caller declares for one artifact.
///
/// At least one of `digest` and `vendor_team` must be present. A digest is
/// checked before extraction; a vendor team is checked on the extracted
/// executable before it counts as installable (see
/// [`verify_signature`](crate::verify_signature)).
#[derive(Clone, Copy, Debug)]
pub struct Declaration<'a> {
    /// The digest source, if the caller declares one.
    pub digest: Option<DigestSource<'a>>,
    /// The vendor's team identifier, if the caller declares a signature.
    pub vendor_team: Option<&'a str>,
}

/// The digest a [`VerifiedArtifact`] was checked against.
#[derive(Clone, Debug)]
struct VerifiedDigest {
    hex: String,
    origin: DigestOrigin,
}

/// An artifact whose declared checks passed. Only [`verify_artifact`]
/// makes one; extraction requires one.
///
/// The value records what was checked, not a claim about the file. The
/// file may change after this point, so extraction copies it and checks the
/// digest on the copy before reading a byte of it.
#[derive(Debug)]
pub struct VerifiedArtifact {
    path: PathBuf,
    asset: String,
    digest: Option<VerifiedDigest>,
    vendor_team: Option<String>,
}

impl VerifiedArtifact {
    /// The asset name the artifact was verified as.
    pub(crate) fn asset(&self) -> &str {
        &self.asset
    }

    /// The downloaded file as it was when verified.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The declared digest to check the copy against, when one was declared.
    pub(crate) fn digest(&self) -> Option<(&str, DigestOrigin)> {
        self.digest
            .as_ref()
            .map(|digest| (digest.hex.as_str(), digest.origin))
    }

    /// The vendor team the executable must carry, when declared.
    pub(crate) fn vendor_team(&self) -> Option<&str> {
        self.vendor_team.as_deref()
    }
}

/// Verifies one downloaded artifact against what the caller declared.
///
/// The digest, if declared, is parsed from its source and compared with the
/// SHA-256 of the file. Neither declared means a refusal before any byte is
/// read.
///
/// # Errors
///
/// [`Refusal::NothingDeclared`] when neither a digest nor a team is declared;
/// the digest refusals of [`digest_from_checksums`], [`digest_from_sidecar`]
/// and [`Refusal::MalformedPin`]; [`Refusal::DigestMismatch`] when the bytes
/// differ; and a host fault when the file cannot be read.
pub fn verify_artifact(
    path: &Path,
    asset: &str,
    declared: Declaration<'_>,
) -> Result<VerifiedArtifact, ProvisionError> {
    if declared.digest.is_none() && declared.vendor_team.is_none() {
        return Err(ProvisionError::Refused(Refusal::NothingDeclared {
            asset: asset.to_owned(),
        }));
    }
    let mut digest = None;
    if let Some(source) = declared.digest {
        let (expected, origin) = match source {
            DigestSource::ReleaseChecksums(checksums) => (
                digest_from_checksums(checksums, asset)?,
                DigestOrigin::ReleaseChecksums,
            ),
            DigestSource::Sidecar(sidecar) => {
                (digest_from_sidecar(sidecar, asset)?, DigestOrigin::Sidecar)
            }
            DigestSource::Pinned(digest) => {
                if !is_sha256_hex(digest) {
                    return Err(ProvisionError::Refused(Refusal::MalformedPin {
                        asset: asset.to_owned(),
                    }));
                }
                (digest.to_owned(), DigestOrigin::Pinned)
            }
        };
        if sha256_file(path)? != expected {
            return Err(ProvisionError::Refused(Refusal::DigestMismatch {
                asset: asset.to_owned(),
                origin,
            }));
        }
        digest = Some(VerifiedDigest {
            hex: expected,
            origin,
        });
    }
    Ok(VerifiedArtifact {
        path: path.to_path_buf(),
        asset: asset.to_owned(),
        digest,
        vendor_team: declared.vendor_team.map(str::to_owned),
    })
}
