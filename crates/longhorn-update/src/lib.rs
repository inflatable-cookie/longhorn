//! Pure update policy for Longhorn: channels, version comparison, staged
//! rollout, and deferral.
//!
//! This crate decides *whether* to offer an update. It never fetches, never
//! verifies a signature, and never installs — those belong to the source
//! adapters and to `longhorn-update-native` respectively. Keeping
//! verification out of here is deliberate: it is what allows the artifact
//! host to be untrusted infrastructure.
//!
//! Everything is pure. No network, no filesystem, no clock.

mod channel;
mod decision;
mod deferral;
mod install;
mod manifest;
mod restart;
mod rollout;
mod source;

pub use channel::{BuildIdentity, Channel};
pub use decision::{CheckKind, OfferReason, UpdateAvailability, UpdateOffer, evaluate};
pub use deferral::{Deferral, DeferralCause};
pub use install::{
    Applied, ConformanceFixtures, ConformanceOutcome, InstallFailure, UpdateInstaller,
    run_conformance,
};
pub use manifest::{Artifact, ChannelManifest, TargetTriple, TargetTripleError};
pub use restart::{OutstandingWork, QuiescenceKind, QuiescenceProbe, QuiescenceReceipt};
pub use rollout::{InstallId, InstallIdError, Rollout, RolloutFraction, RolloutFractionError};
pub use source::{
    EndpointUrl, EndpointUrlError, GitHubReleasesSource, ObjectStorageSource, SourceError,
    SourceRequest, StaticJsonSource, UpdateSource,
};
