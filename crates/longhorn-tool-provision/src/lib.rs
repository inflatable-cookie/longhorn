//! Verified artifacts and bounded extraction for managed command-line tools
//! (contract 027).
//!
//! Three steps, each a type the next step needs:
//!
//! 1. [`verify_artifact`] checks a digest against the download when declared and
//!    returns a [`VerifiedArtifact`].
//! 2. [`extract_executable`] takes that artifact and writes one requested
//!    member into a caller's staging directory, within [`ExtractionLimits`],
//!    as a [`StagedExecutable`] that cannot be installed yet.
//! 3. [`verify_signature`] checks the declared vendor team on the staged
//!    executable and returns an [`InstallableExecutable`].
//!
//! The crate names no product: the caller supplies the artifact shape, the
//! member path, the digest source and the expected team. Catalogue entries,
//! pins and release names stay with the caller. Release resolution, download
//! and the install pipeline are later parts of this contract (4b and 4c).

mod budget;
mod digest;
mod extract;
mod failure;
mod signature;
mod verify;

pub use digest::{digest_from_checksums, digest_from_sidecar, sha256_file};
pub use extract::{ArtifactShape, ExtractionLimits, StagedExecutable, extract_executable};
pub use failure::{Fault, ProvisionError, Refusal};
pub use signature::{
    CodesignVerifier, InstallableExecutable, SignatureReport, SignatureVerifier, verify_signature,
};
pub use verify::{Declaration, DigestOrigin, DigestSource, VerifiedArtifact, verify_artifact};
