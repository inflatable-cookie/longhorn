//! Pure update policy for Longhorn: channels, version comparison, staged
//! rollout, and deferral.
//!
//! This crate decides *whether* to offer an update. It never fetches, never
//! verifies a signature, and never installs — those belong to the source
//! adapters and to the Tauri updater plugin respectively. Keeping
//! verification out of here is deliberate: it is what allows the artifact
//! host to be untrusted infrastructure.
//!
//! Everything is pure. No network, no filesystem, no clock.

mod channel;
mod decision;
mod deferral;
mod manifest;
mod rollout;

pub use channel::{BuildIdentity, Channel};
pub use decision::{CheckKind, OfferReason, UpdateAvailability, UpdateOffer, evaluate};
pub use deferral::{Deferral, DeferralCause};
pub use manifest::{Artifact, ChannelManifest, TargetTriple, TargetTripleError};
pub use rollout::{InstallId, InstallIdError, Rollout, RolloutFraction, RolloutFractionError};
