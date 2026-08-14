use core::fmt;
use std::error::Error;

use semver::Version;

use crate::{ArtifactKey, VerifiedArtifact, verify_artifact};

/// Applies a downloaded update.
///
/// One implementation serves every host: `longhorn-update-install`. Contract
/// 018 was amended on 2026-08-09 to make execution host-independent, after
/// Card 162 established that the Tauri updater plugin cannot satisfy this
/// trait at all — its verification lives inside its own downloader, and its
/// `install` accepts caller-supplied bytes without verifying them.
///
/// The conformance suite below therefore has one implementation rather than
/// two. It stays, because it is what makes "verify before anything reaches
/// disk" a checked claim rather than a comment, and because a second
/// implementation for Windows installers would have to meet it.
///
/// The contract is deliberately coarse: it names observable outcomes rather
/// than separable steps, so a platform whose install is one opaque operation
/// can still satisfy it. What an implementation must promise is
/// the *observable* behaviour: what reaches disk, and what is reported.
pub trait UpdateInstaller {
    /// Applies an artifact that has already been proved genuine.
    ///
    /// The parameter is a [`VerifiedArtifact`], which only
    /// [`verify_artifact`](crate::verify_artifact) can construct. "No
    /// unverified artifact reaches disk" is therefore a property of the call
    /// rather than an instruction an implementation has to follow: there is no
    /// configuration, host, or build profile under which this can be reached
    /// with unproved bytes, because there is no way to make the argument.
    ///
    /// It used to take `&[u8]` and `&str` and say the same thing in prose,
    /// with a conformance case to catch an implementation that ignored it.
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure>;
}

/// What an applied update left behind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Applied {
    /// The version now on disk.
    pub version: Version,
    /// Whether the application relaunched itself.
    ///
    /// macOS separates replacement from relaunch, and relaunch is known to
    /// fail there (tauri#11392). An install that reached disk without
    /// relaunching is still an applied update — see contract 018's
    /// Reporting section.
    pub relaunched: bool,
}

/// Why an update was not applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallFailure {
    /// The signature did not verify against the embedded key.
    ///
    /// Terminal. Not retried, not reported as a transient fault, and never
    /// applied anyway.
    SignatureRejected,
    /// The artifact was not the shape the host expects.
    MalformedArtifact {
        /// What was wrong.
        detail: String,
    },
    /// The installed application could not be written.
    ///
    /// Homebrew casks and administrator-installed copies land here. The
    /// remedy is a manual download, not a retry.
    NotWritable {
        /// What could not be written.
        detail: String,
    },
    /// Replacement failed for another reason.
    Failed {
        /// What went wrong.
        detail: String,
    },
}

impl InstallFailure {
    /// Returns whether retrying unattended could succeed.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Returns whether the user must act for this to resolve.
    #[must_use]
    pub const fn needs_manual_action(&self) -> bool {
        matches!(self, Self::NotWritable { .. })
    }
}

impl fmt::Display for InstallFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignatureRejected => formatter.write_str("update signature did not verify"),
            Self::MalformedArtifact { detail } => write!(formatter, "malformed artifact: {detail}"),
            Self::NotWritable { detail } => {
                write!(formatter, "installation is not writable: {detail}")
            }
            Self::Failed { detail } => write!(formatter, "update install failed: {detail}"),
        }
    }
}

impl Error for InstallFailure {}

/// Fixtures a conformance run needs from the implementation under test.
///
/// Supplied by the implementation because a valid artifact is host-shaped:
/// Tauri wants a signed `.tar.gz` of an application bundle, and a native
/// installer may want something else. The suite does not care what they are,
/// only that one verifies and one does not.
pub trait ConformanceFixtures {
    /// A version the artifacts claim.
    fn version(&self) -> Version;

    /// The key the fixtures were signed with.
    ///
    /// Needed since verification moved out of the installer: the suite proves
    /// the tampered fixture is rejected, and it now has to do the verifying
    /// itself rather than watching an implementation do it.
    fn key(&self) -> ArtifactKey;

    /// An artifact and signature that must verify and apply.
    fn valid(&self) -> (Vec<u8>, String);

    /// An artifact whose signature must be rejected.
    ///
    /// Typically the valid artifact with one byte flipped.
    fn tampered(&self) -> (Vec<u8>, String);

    /// Bytes that are correctly signed but are not a usable artifact.
    ///
    /// Distinguishes "we do not trust this" from "we trust it and cannot use
    /// it" — different messages, and only one of them is a security event.
    fn signed_but_unusable(&self) -> Option<(Vec<u8>, String)> {
        None
    }

    /// Signed archives crafted to escape the install destination.
    ///
    /// Each archive verifies; each must still be refused as
    /// `MalformedArtifact`. A signature proves origin, not good intent, so
    /// extraction safety is a per-implementation claim and lives here rather
    /// than in any one installer's tests. The default is empty; an installer
    /// whose format cannot express an escaping entry leaves it so.
    fn signed_but_escaping(&self) -> Vec<(Vec<u8>, String)> {
        Vec::new()
    }
}

/// One conformance check and its outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceOutcome {
    /// Stable name of the claim.
    pub claim: &'static str,
    /// Whether the implementation satisfied it.
    pub satisfied: bool,
    /// What happened, when it did not.
    pub detail: Option<String>,
}

/// Runs every claim both installers must satisfy.
///
/// Returns one outcome per claim rather than failing fast, so a report says
/// everything an implementation gets wrong instead of the first thing.
pub fn run_conformance<I, F>(installer: &I, fixtures: &F) -> Vec<ConformanceOutcome>
where
    I: UpdateInstaller,
    F: ConformanceFixtures,
{
    let version = fixtures.version();
    let key = fixtures.key();
    let mut outcomes = Vec::new();

    let (artifact, signature) = fixtures.valid();
    let applied = verify_artifact(&key, &version, artifact, &signature)
        .and_then(|verified| installer.apply(&verified));
    match applied {
        Ok(applied) => {
            outcomes.push(satisfied("a verified artifact is applied"));
            outcomes.push(check(
                "the applied version is the one requested",
                applied.version == version,
                format!("applied {} for a request of {version}", applied.version),
            ));
        }
        Err(failure) => {
            outcomes.push(failed("a verified artifact is applied", &failure));
            outcomes.push(failed("the applied version is the one requested", &failure));
        }
    }

    // A claim about the shared verifier rather than about this installer,
    // since the installer can no longer be reached with a tampered artifact.
    // It stays in the suite because it is still per-implementation: it proves
    // the fixture's own signing agrees with the verifier the controller uses.
    let (artifact, signature) = fixtures.tampered();
    match verify_artifact(&key, &version, artifact, &signature) {
        Err(InstallFailure::SignatureRejected) => {
            outcomes.push(satisfied("a tampered artifact is rejected as unsigned"));
        }
        Err(other) => outcomes.push(check(
            "a tampered artifact is rejected as unsigned",
            false,
            format!("rejected, but as {other} rather than a signature failure"),
        )),
        Ok(_) => outcomes.push(check(
            "a tampered artifact is rejected as unsigned",
            false,
            "the artifact verified".to_owned(),
        )),
    }

    if let Some((artifact, signature)) = fixtures.signed_but_unusable() {
        let outcome = verify_artifact(&key, &version, artifact, &signature)
            .and_then(|verified| installer.apply(&verified));
        match outcome {
            Err(InstallFailure::MalformedArtifact { .. }) => outcomes.push(satisfied(
                "a signed but unusable artifact is not a signature failure",
            )),
            Err(other) => outcomes.push(check(
                "a signed but unusable artifact is not a signature failure",
                false,
                format!("reported as {other}"),
            )),
            Ok(_) => outcomes.push(check(
                "a signed but unusable artifact is not a signature failure",
                false,
                "the artifact was applied".to_owned(),
            )),
        }
    }

    for (index, (artifact, signature)) in fixtures.signed_but_escaping().into_iter().enumerate() {
        let outcome = verify_artifact(&key, &version, artifact, &signature)
            .and_then(|verified| installer.apply(&verified));
        match outcome {
            Err(InstallFailure::MalformedArtifact { .. }) => outcomes.push(satisfied(
                "a signed archive escaping the destination is refused as malformed",
            )),
            Err(other) => outcomes.push(check(
                "a signed archive escaping the destination is refused as malformed",
                false,
                format!("fixture {index}: reported as {other}"),
            )),
            Ok(_) => outcomes.push(check(
                "a signed archive escaping the destination is refused as malformed",
                false,
                format!("fixture {index}: the archive was applied"),
            )),
        }
    }

    outcomes
}

fn satisfied(claim: &'static str) -> ConformanceOutcome {
    ConformanceOutcome {
        claim,
        satisfied: true,
        detail: None,
    }
}

fn check(claim: &'static str, satisfied: bool, detail: String) -> ConformanceOutcome {
    ConformanceOutcome {
        claim,
        satisfied,
        detail: (!satisfied).then_some(detail),
    }
}

fn failed(claim: &'static str, failure: &InstallFailure) -> ConformanceOutcome {
    check(claim, false, failure.to_string())
}
