use core::fmt;
use std::error::Error;

use semver::Version;

/// Applies a downloaded update.
///
/// One implementation serves every host: `longhorn-update-native`. Contract
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
    /// Applies `artifact`, whose detached signature is `signature`.
    ///
    /// An implementation **must** verify before applying. There is no
    /// configuration, host, or build profile under which an unverified
    /// artifact may reach disk.
    fn apply(
        &self,
        version: &Version,
        artifact: &[u8],
        signature: &str,
    ) -> Result<Applied, InstallFailure>;
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
    let mut outcomes = Vec::new();

    let (artifact, signature) = fixtures.valid();
    match installer.apply(&version, &artifact, &signature) {
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

    let (artifact, signature) = fixtures.tampered();
    match installer.apply(&version, &artifact, &signature) {
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
            "the artifact was applied".to_owned(),
        )),
    }

    if let Some((artifact, signature)) = fixtures.signed_but_unusable() {
        match installer.apply(&version, &artifact, &signature) {
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
