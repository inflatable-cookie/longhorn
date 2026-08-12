//! Artifact verification, and the type that makes it unreachable-around.
//!
//! Verification used to live in `longhorn-update-install`, beside the bundle
//! replacement it guards. That was one verifier in the right place, and it
//! still relied on every `UpdateInstaller` implementation choosing to call it:
//! the trait took raw bytes and a signature, its doc comment said an
//! implementation must verify first, and the conformance suite carried an
//! `Unverifying` case to catch one that did not.
//!
//! A suite that catches a mistake is worse than a design that cannot make it.
//! Verification moved here — the pure crate both sides already depend on — and
//! `UpdateInstaller::apply` now takes a [`VerifiedArtifact`], which nothing
//! outside this module can construct. It is a relocation, not a second line:
//! `longhorn-update-install` no longer verifies, and no longer holds a key.
//!
//! # Why minisign
//!
//! Tauri's answer, adopted for the reason `longhorn-update-install` recorded
//! when it chose it: a product shipping to both hosts signs once, with one key
//! and one signing step per release. Verification is pure computation, so it
//! costs this crate none of its purity.

use core::fmt;
use std::error::Error;

use minisign_verify::{PublicKey, Signature};
use semver::Version;

use crate::InstallFailure;

/// The public key an artifact must verify against.
///
/// Held by the controller rather than by an installer. An installer that
/// carried its own key could be constructed with a different one from the
/// controller checking on its behalf, and nothing would notice.
#[derive(Clone, Debug)]
pub struct ArtifactKey(PublicKey);

impl ArtifactKey {
    /// Reads a minisign public key in its untagged base64 form.
    pub fn from_base64(value: &str) -> Result<Self, ArtifactKeyError> {
        PublicKey::from_base64(value)
            .map(Self)
            .map_err(|error| ArtifactKeyError {
                detail: error.to_string(),
            })
    }

    /// Reads a minisign public key file, comment line included.
    pub fn from_key_file(value: &str) -> Result<Self, ArtifactKeyError> {
        PublicKey::decode(value)
            .map(Self)
            .map_err(|error| ArtifactKeyError {
                detail: error.to_string(),
            })
    }
}

/// A public key could not be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactKeyError {
    /// What was wrong with it.
    pub detail: String,
}

impl fmt::Display for ArtifactKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unreadable artifact key: {}", self.detail)
    }
}

impl Error for ArtifactKeyError {}

/// Bytes proved to come from the signing key.
///
/// Constructible only by [`verify_artifact`]. That is the whole point of the
/// type: an installer cannot be handed unverified bytes, so "verify before
/// anything reaches disk" is a property of the call rather than a promise in
/// a doc comment.
///
/// It carries the version it was fetched for. The version is **not** proved by
/// the signature — see the module note on downgrade below — so this records
/// what the controller asked for, not what the artifact claims to be.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    version: Version,
    bytes: Vec<u8>,
}

impl VerifiedArtifact {
    /// The version this artifact was fetched to install.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// The verified bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Verifies `artifact` against `key`, or refuses it.
///
/// # Downgrade, and what this does not defend
///
/// A signature proves the bytes came from the signing key. It does not prove
/// they are the *current* release: an old artifact with its own valid
/// signature verifies. Defending that needs the version bound into something
/// signed, which minisign has room for in the trusted comment and Tauri's
/// signing does not populate with a version today.
///
/// The manifest carries the version and travels over HTTPS, and `source.rs`
/// already records the residual — "a tampered manifest cannot forge an
/// artifact, though it can withhold one or pin an install to a stale version".
/// This is that same residual, unchanged. Closing it is a signing-side change
/// and is recorded as an open decision on Card 196 rather than taken here.
pub fn verify_artifact(
    key: &ArtifactKey,
    version: &Version,
    artifact: Vec<u8>,
    signature: &str,
) -> Result<VerifiedArtifact, InstallFailure> {
    let decoded = Signature::decode(signature).map_err(|_| InstallFailure::SignatureRejected)?;
    key.0
        .verify(&artifact, &decoded, false)
        .map_err(|_| InstallFailure::SignatureRejected)?;

    Ok(VerifiedArtifact {
        version: version.clone(),
        bytes: artifact,
    })
}
