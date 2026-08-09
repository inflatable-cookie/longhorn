//! Pure update policy for Longhorn: channels, version comparison, staged
//! rollout, deferral, and the restart interlock.
//!
//! This crate decides *whether* to offer an update. It never fetches, never
//! verifies a signature, and never installs — those belong to the source
//! adapters and to `longhorn-update-install` respectively. Keeping
//! verification out of here is deliberate: it is what allows the artifact
//! host to be untrusted infrastructure.
//!
//! Everything is pure. No network, no filesystem, no clock.

mod channel;
mod decision;
mod deferral;
mod gate;
mod install;
mod manifest;
mod probes;
mod restart;
mod rollout;
mod source;

pub use channel::{BuildIdentity, Channel};
pub use decision::{CheckKind, OfferReason, UpdateAvailability, UpdateOffer, evaluate};
pub use deferral::{Deferral, DeferralCause};
pub use gate::{InstallAuthorization, UpdateGate};
pub use install::{
    Applied, ConformanceFixtures, ConformanceOutcome, InstallFailure, UpdateInstaller,
    run_conformance,
};
pub use manifest::{Artifact, ChannelManifest, TargetTriple, TargetTripleError};
pub use probes::{CountingProbe, operation_probe, transfer_session_probe};
pub use restart::{OutstandingWork, QuiescenceKind, QuiescenceProbe, QuiescenceReceipt};
pub use rollout::{InstallId, InstallIdError, Rollout, RolloutFraction, RolloutFractionError};
pub use source::{
    EndpointUrl, EndpointUrlError, GitHubReleasesSource, ObjectStorageSource, SourceError,
    SourceRequest, StaticJsonSource, UpdateSource,
};
