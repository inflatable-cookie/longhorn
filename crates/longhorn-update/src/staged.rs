//! The staged artifact: verified bytes retained between prepare and apply.
//!
//! Contract 018 was amended on 2026-09-22. The install sequence used to be one
//! call that fetched, verified, gated and replaced, which left a surface no
//! way to show byte progress or to keep a download through "Later". Prepare
//! and apply are now separate, and something has to hold the artifact between
//! them.
//!
//! # The trust boundary does not move
//!
//! [`StagedArtifact`] holds a [`VerifiedArtifact`] and a digest of its bytes.
//! Its only constructor takes a `VerifiedArtifact`, which only
//! [`verify_artifact`](crate::verify_artifact) can make, so there is no
//! configuration under which a staged artifact holds unverified bytes. The
//! earlier design refused to have a half-finished transfer type at all; this
//! reverses that, and it is safe to reverse precisely because the type it
//! retains is the verified one rather than raw bytes. An artifact that fails
//! verification is discarded, never staged.

use semver::Version;
use sha2::{Digest, Sha256};

use crate::{Channel, UpdateStagedArtifactProjection, VerifiedArtifact};

/// SHA-256 of a staged artifact's bytes, as lowercase hex.
///
/// The digest is what binds the retained bytes to the release the manifest
/// named. It is computed here rather than carried in the manifest: it proves
/// the bytes are the ones that were verified, not that the manifest was the
/// one the host meant to read — the signature already covers origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactDigest(String);

impl ArtifactDigest {
    /// Digests `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte & 0x0f));
        }
        Self(hex)
    }

    /// The digest as lowercase hex.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn hex_digit(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble)])
}

/// An identity-bound verified artifact retained between prepare and apply.
///
/// Exactly one of these is retained per controller. The version, channel and
/// digest travel with the bytes so a surface can show what is waiting and
/// `apply` can refuse a request naming something else.
#[derive(Clone, Debug)]
pub struct StagedArtifact {
    version: Version,
    channel: Channel,
    digest: ArtifactDigest,
    verified: VerifiedArtifact,
}

impl StagedArtifact {
    /// Binds a verified artifact to the release identity it was fetched for.
    ///
    /// Takes a [`VerifiedArtifact`], so the staged type cannot be built from
    /// bytes that were never verified.
    #[must_use]
    pub fn new(version: Version, channel: Channel, verified: VerifiedArtifact) -> Self {
        let digest = ArtifactDigest::of(verified.bytes());
        Self {
            version,
            channel,
            digest,
            verified,
        }
    }

    /// The version the bytes were verified for.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// The channel the bytes were fetched for.
    #[must_use]
    pub const fn channel(&self) -> Channel {
        self.channel
    }

    /// The digest of the retained bytes.
    #[must_use]
    pub const fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }

    /// The verified artifact `apply` hands the installer.
    #[must_use]
    pub const fn verified(&self) -> &VerifiedArtifact {
        &self.verified
    }

    /// The client-facing identity of this staged artifact.
    #[must_use]
    pub fn projection(&self) -> UpdateStagedArtifactProjection {
        UpdateStagedArtifactProjection {
            version: self.version.to_string(),
            channel: self.channel,
            digest: self.digest.as_str().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use minisign::KeyPair;
    use semver::Version;

    use super::*;
    use crate::{ArtifactKey, verify_artifact};

    fn verified(keys: &KeyPair, bytes: &[u8]) -> VerifiedArtifact {
        let signature = minisign::sign(None, &keys.sk, Cursor::new(bytes), None, None)
            .unwrap()
            .to_string();
        verify_artifact(
            &ArtifactKey::from_base64(&keys.pk.to_base64()).unwrap(),
            &Version::parse("1.4.0").unwrap(),
            bytes.to_vec(),
            &signature,
        )
        .unwrap()
    }

    /// The digest is the identity of the bytes, so different bytes cannot
    /// share one.
    #[test]
    fn the_digest_follows_the_bytes() {
        assert_eq!(
            ArtifactDigest::of(b"a bundle"),
            ArtifactDigest::of(b"a bundle")
        );
        assert_ne!(
            ArtifactDigest::of(b"a bundle"),
            ArtifactDigest::of(b"another bundle")
        );
        assert_eq!(ArtifactDigest::of(b"").as_str().len(), 64);
    }

    #[test]
    fn a_staged_artifact_carries_its_verified_identity() {
        let keys = KeyPair::generate_unencrypted_keypair().unwrap();
        let verified = verified(&keys, b"a bundle");
        let staged = StagedArtifact::new(
            Version::parse("1.4.0").unwrap(),
            Channel::Production,
            verified.clone(),
        );

        assert_eq!(staged.version().to_string(), "1.4.0");
        assert_eq!(staged.channel(), Channel::Production);
        assert_eq!(staged.verified(), &verified);
        assert_eq!(staged.digest(), &ArtifactDigest::of(b"a bundle"));
        assert_eq!(
            staged.projection().digest,
            ArtifactDigest::of(b"a bundle").as_str()
        );
        assert_eq!(staged.projection().channel, Channel::Production);
    }
}
