//! The artifact transfer: described here, performed by the host.
//!
//! The same division Card 152 set for manifests. `UpdateSource` already
//! composes the request — `artifact_request` has existed since then — and this
//! is the other half: who performs it, and how progress comes back.
//!
//! Longhorn drives the download and observes it. It does not run it. A crate
//! with no network cannot, and a crate that grew one would be responsible for
//! retry policy, proxy configuration and TLS trust on two hosts.

use core::fmt;
use std::error::Error;

use crate::SourceRequest;

/// How far through a transfer is.
///
/// `expected` is `Option` because a source need not report a content length.
/// Everything downstream of that — the `null` fraction in
/// `UpdateProgressProjection` — follows from this one absence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FetchProgress {
    /// Bytes received so far.
    pub received: u64,
    /// Bytes the source said to expect, when it said.
    pub expected: Option<u64>,
}

impl FetchProgress {
    /// Records progress with no declared length.
    #[must_use]
    pub const fn unbounded(received: u64) -> Self {
        Self {
            received,
            expected: None,
        }
    }

    /// Records progress against a declared length.
    #[must_use]
    pub const fn of(received: u64, expected: u64) -> Self {
        Self {
            received,
            expected: Some(expected),
        }
    }

    /// How far through, as a fraction, when that can be said at all.
    ///
    /// `None` rather than zero when the length is unknown, and `None` rather
    /// than a division by zero when the length is zero. A bar that invents a
    /// number is worse than a bar that says it does not know.
    #[must_use]
    pub fn fraction(self) -> Option<f64> {
        let expected = self.expected.filter(|expected| *expected > 0)?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a progress fraction does not need more precision than an f64 gives"
        )]
        Some((self.received as f64 / expected as f64).clamp(0.0, 1.0))
    }
}

/// Performs one artifact transfer.
///
/// Implemented by the host, called by the controller — the same shape as
/// [`UpdateInstaller`](crate::UpdateInstaller).
///
/// # Resume is out of scope
///
/// There is no offset, no partial handle, and no way to hand a half-finished
/// transfer back. A transfer either produces the whole artifact or fails, and
/// a failed one leaves nothing a later check has to reason about. Said here
/// rather than left to be inferred from an absent parameter: a caller that
/// wants resume needs a different trait, not a convention on this one.
pub trait ArtifactFetch {
    /// Fetches the artifact `request` addresses, reporting progress.
    ///
    /// `report` is called as bytes arrive. How often is the host's business:
    /// the controller records whatever it is told and does not require a
    /// cadence, because a source that delivers in one chunk cannot invent one.
    fn fetch(
        &self,
        request: &SourceRequest,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError>;
}

/// Why an artifact did not arrive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FetchError {
    /// The transfer did not complete.
    ///
    /// Retryable. Covers every transport fault the host does not distinguish;
    /// the crate has no way to tell a reset connection from a DNS failure and
    /// no decision that would change if it could.
    Interrupted {
        /// What the host reported.
        detail: String,
    },
    /// The source answered, and not with the artifact.
    ///
    /// A 404 on a URL the manifest gave means the manifest and the host
    /// disagree about what was published. Not retryable, and a different
    /// message from a dropped connection.
    Unavailable {
        /// What the host reported.
        detail: String,
    },
}

impl FetchError {
    /// Returns whether retrying unattended could succeed.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Interrupted { .. })
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted { detail } => write!(formatter, "artifact transfer failed: {detail}"),
            Self::Unavailable { detail } => write!(formatter, "artifact unavailable: {detail}"),
        }
    }
}

impl Error for FetchError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole reason the fraction is `Option`. Card 190 built the field and
    /// had nothing that could produce the absent case.
    #[test]
    fn a_source_with_no_length_yields_no_fraction() {
        assert_eq!(FetchProgress::unbounded(4_096).fraction(), None);
    }

    #[test]
    fn a_declared_length_yields_a_fraction() {
        assert_eq!(FetchProgress::of(50, 200).fraction(), Some(0.25));
    }

    /// Zero is a length a source can honestly report, and dividing by it is
    /// not a fraction of anything.
    #[test]
    fn a_zero_length_yields_no_fraction_rather_than_a_division() {
        assert_eq!(FetchProgress::of(0, 0).fraction(), None);
    }

    /// A host that over-reports must not produce a bar past its own end.
    #[test]
    fn more_bytes_than_declared_clamps() {
        assert_eq!(FetchProgress::of(300, 200).fraction(), Some(1.0));
    }
}
