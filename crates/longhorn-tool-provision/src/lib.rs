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
//! Part 4b adds release resolution and a bounded download: a [`ReleaseSource`]
//! resolves one release of a repository and downloads one of its assets. The
//! production source is [`HttpReleases`], over blocking HTTP with rustls. The
//! caller supplies the owner, repository, pin, floor, endpoints, bounds and
//! optional token. The bytes it writes are checked by step 1, not trusted.
//!
//! The crate names no product: the caller supplies the artifact shape, the
//! member path, the digest source, the expected team, and the release query.
//! Catalogue entries, pins and release names stay with the caller. The install
//! pipeline is part 4c.
//!
//! Test support is behind the `fixture-server` feature: a loopback server
//! (`FixtureServer`) and `ReleaseEndpoints::loopback_for_tests`.

mod budget;
mod digest;
mod extract;
mod failure;
#[cfg(feature = "fixture-server")]
mod fixture;
mod install;
mod install_failure;
mod path;
mod release;
mod runner;
mod signature;
mod verify;

pub use digest::{digest_from_checksums, digest_from_sidecar, sha256_file};
pub use extract::{ArtifactShape, ExtractionLimits, StagedExecutable, extract_executable};
pub use failure::{Fault, ProvisionError, Refusal};
#[cfg(feature = "fixture-server")]
pub use fixture::{FixtureRoute, FixtureServer};
pub use install::{
    DigestDeclaration, InstallReceipt, InstallStage, MAX_DIGEST_METADATA_BYTES,
    MAX_PROBE_FIELD_BYTES, ToolDescription, ToolHealth, install_tool, parse_version, tool_health,
};
pub use install_failure::{InstallError, InstallFault, InstallRefusal, ProbePhase};
pub use path::{
    LOGIN_PATH_BEGIN, LOGIN_PATH_END, LoginPath, LoginPathCache, LoginPathParseError,
    find_executable_on_path, is_executable, parse_login_path_output,
};
pub use release::{
    HttpReleases, Release, ReleaseEndpoints, ReleaseError, ReleaseFault, ReleaseOptions,
    ReleaseQuery, ReleaseRefusal, ReleaseSource, version_from_tag,
};
pub use runner::{
    CommandRunner, MAX_PROCESS_OUTPUT_BYTES, ProcessOutput, ProcessRunner, RunnerError,
};
/// Semantic version used by release queries, floors and install probes.
pub use semver::Version;
pub use signature::{
    CodesignVerifier, InstallableExecutable, SignatureReport, SignatureVerifier, verify_signature,
};
pub use verify::{Declaration, DigestOrigin, DigestSource, VerifiedArtifact, verify_artifact};
