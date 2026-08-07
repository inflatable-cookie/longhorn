//! Pure licence policy for Longhorn: the licence shape, entitlement
//! evaluation, trust basis, lease and grace.
//!
//! Longhorn owns the shape of a licence and the evaluation of it.
//! Applications own where a licence comes from, what its entitlements mean,
//! and what happens when one is absent.
//!
//! Two rules run through everything here.
//!
//! **Longhorn answers "entitled?" and never enforces.** Nothing in this
//! crate disables, refuses, or degrades anything. What the absence of an
//! entitlement means — including whether it means anything at all — is the
//! application's decision.
//!
//! **Licensing is not a security boundary.** The check runs on hardware the
//! user controls. Where a choice trades customer friction against piracy
//! resistance, it resolves toward the customer: an unreachable backend fails
//! open within the lease, and a wrong clock is tolerated before it is
//! refused.
//!
//! Everything is pure. No network, no filesystem, no ambient clock — `now`
//! is always supplied.

mod activation;
mod entitlement;
mod key;
mod licence;
mod status;
mod time;
mod verify;

pub use activation::{
    Activation, ActivationError, ActivationRequest, ActivationSource, ActivationUrl,
    ActivationUrlError, Credential, SignedFileSource, TokenRedemptionSource, asserted_remotely,
};
pub use entitlement::{EntitlementId, EntitlementIdError, Entitlements, Limit};
pub use key::{LicenceKey, LicenceKeyError};
pub use licence::{GracePolicy, LicencePayload, TrustBasis, VerifiedLicence};
pub use status::{ClockGuard, Usability, usability};
pub use time::{Span, Timestamp};
pub use verify::{SignedLicence, VerificationError, verify};
