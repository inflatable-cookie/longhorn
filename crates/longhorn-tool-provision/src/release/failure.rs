//! Release failure classes (contract 027, part 4b).
//!
//! Like [`crate::ProvisionError`], the split is the verdict: a refusal is a
//! judgment on the release or asset, and the same answer refuses again; a
//! fault is a host or network problem, and a retry may succeed. Callers
//! match on the class and the variant, never on the message.

use std::{fmt, path::PathBuf};

/// Why the release or asset was refused. The same answer refuses again.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReleaseRefusal {
    /// No release satisfies the query: the pinned release is absent, the
    /// latest release is a draft or pre-release, or it sits below the floor.
    NoCompatibleRelease {
        /// The floor the query set, when one was set.
        floor: Option<semver::Version>,
        /// The exact tag the query pinned, when one was pinned.
        requested: Option<String>,
    },
    /// The pinned tag's answer names a different tag than the one pinned.
    TagMismatch {
        /// The tag the query pinned.
        requested: String,
        /// The tag the listing names.
        found: String,
    },
    /// The listing is not the release JSON this crate reads.
    Malformed {
        /// The parser's description.
        detail: String,
    },
    /// The resolved release does not list the asset.
    MissingAsset {
        /// The release's tag.
        release: String,
        /// The asset the caller asked for.
        asset: String,
    },
    /// A redirect target is not allowed (for example, a different host on a
    /// listing request, or a non-HTTPS target on a download).
    UnsafeRedirect {
        /// What the redirect policy refused.
        detail: String,
    },
    /// The asset is larger than the bound: its declared length, or the bytes
    /// the server actually sent.
    ArtifactTooLarge {
        /// The asset that failed.
        asset: String,
        /// The byte bound that was exceeded.
        limit: u64,
    },
}

/// A host or network problem. A retry may succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReleaseFault {
    /// The release source could not answer: a transport failure, a timeout,
    /// or a failing status other than not-found and rate-limited.
    Unreachable {
        /// The transport's or the status's description. Never a credential.
        detail: String,
    },
    /// The API rate limit for this identity (or address) is exhausted.
    RateLimited {
        /// The unix time the source named for the reset, when it named one.
        reset_unix: Option<i64>,
    },
    /// A file could not be created, written or removed.
    Io {
        /// The file involved.
        path: PathBuf,
        /// The operating system's description, and any cleanup failure.
        detail: String,
    },
    /// The HTTP client could not be built, for example because TLS could not
    /// start.
    Client {
        /// The builder's description.
        detail: String,
    },
    /// The caller asked for something this crate does not accept: a
    /// non-HTTPS endpoint, a name with a separator, or a zero bound. A caller
    /// bug, not a release or host problem.
    Usage {
        /// What was refused.
        detail: String,
    },
}

/// Any release-source failure: a verdict on the release, or a host fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReleaseError {
    /// The release or asset was refused.
    Refused(ReleaseRefusal),
    /// The host could not finish.
    Fault(ReleaseFault),
}

impl fmt::Display for ReleaseRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCompatibleRelease { floor, requested } => match (requested, floor) {
                (Some(requested), _) => {
                    write!(formatter, "the pinned release {requested} is not available")
                }
                (None, Some(floor)) => {
                    write!(formatter, "no release at or above {floor} is available")
                }
                (None, None) => write!(formatter, "no release is available"),
            },
            Self::TagMismatch { requested, found } => write!(
                formatter,
                "the release tagged {found} is not the pinned release {requested}"
            ),
            Self::Malformed { detail } => {
                write!(formatter, "the release listing could not be read: {detail}")
            }
            Self::MissingAsset { release, asset } => {
                write!(
                    formatter,
                    "release {release} does not carry the asset {asset}"
                )
            }
            Self::UnsafeRedirect { detail } => {
                write!(formatter, "a redirect was refused: {detail}")
            }
            Self::ArtifactTooLarge { asset, limit } => {
                write!(
                    formatter,
                    "the asset {asset} is larger than the {limit}-byte bound"
                )
            }
        }
    }
}

impl fmt::Display for ReleaseFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable { detail } => {
                write!(
                    formatter,
                    "the release source could not be reached: {detail}"
                )
            }
            Self::RateLimited {
                reset_unix: Some(reset),
            } => {
                write!(
                    formatter,
                    "the release API rate limit is exhausted until unix time {reset}"
                )
            }
            Self::RateLimited { reset_unix: None } => {
                write!(
                    formatter,
                    "the release API rate limit is exhausted; retry later"
                )
            }
            Self::Io { path, detail } => write!(formatter, "{}: {detail}", path.display()),
            Self::Client { detail } => {
                write!(formatter, "the HTTP client could not be built: {detail}")
            }
            Self::Usage { detail } => write!(formatter, "invalid release request: {detail}"),
        }
    }
}

impl fmt::Display for ReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => refusal.fmt(formatter),
            Self::Fault(fault) => fault.fmt(formatter),
        }
    }
}

impl std::error::Error for ReleaseError {}
