//! Exact metadata-only update protocol projections.
//!
//! The domain types below this module hold `semver::Version`, borrow probes,
//! and are generic over closures. None of that crosses a boundary, so this
//! module restates what a client needs as owned, versioned, payload-free
//! records — the same division every other Longhorn domain draws.
//!
//! Versions are strings on the wire. `semver::Version` serialises as one
//! already; saying so in the binding keeps a consumer from receiving an object
//! it has to reassemble.

use serde::{Deserialize, Serialize};

use crate::{Channel, Deferral, DeferralCause, InstallManager, OfferReason, UpdateAvailability};

/// Exact update protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct UpdateProtocolVersion(u32);

impl UpdateProtocolVersion {
    /// The only line this build speaks.
    pub const CURRENT: Self = Self(UPDATE_PROTOCOL_VERSION);
}

/// The protocol line this build speaks.
pub const UPDATE_PROTOCOL_VERSION: u32 = 1;

/// The outcome of the last check, as a client reads it.
///
/// `AheadOfChannel` is its own state rather than folded into `UpToDate`. An
/// install on `1.3.0-nightly.4` that selects production sits ahead of
/// production `1.2.9` and receives nothing until `1.3.0` ships; that is
/// correct and indistinguishable from a broken updater unless it is said out
/// loud. Collapsing it here would make the surface that says it impossible.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum UpdateAvailabilityProjection {
    /// An update is available now.
    Offer {
        /// The version offered.
        version: String,
        /// Why it is offered.
        reason: OfferReason,
        /// Release notes, when published.
        notes: Option<String>,
    },
    /// The install is on the channel's current version.
    UpToDate,
    /// The install is ahead of the channel it selected.
    AheadOfChannel {
        /// What is installed.
        installed: String,
        /// What the channel currently publishes.
        channel: String,
    },
    /// A newer version exists, and this install is not in the current stage.
    WithheldByRollout {
        /// The version being staged.
        version: String,
    },
    /// A newer version exists, and something other than this application
    /// installs it.
    ManagedElsewhere {
        /// The version available.
        version: String,
        /// What manages the install.
        manager: InstallManager,
    },
}

impl UpdateAvailabilityProjection {
    /// Projects one checked availability.
    #[must_use]
    pub fn from_availability(availability: &UpdateAvailability) -> Self {
        match availability {
            UpdateAvailability::Offer(offer) => Self::Offer {
                version: offer.version.to_string(),
                reason: offer.reason,
                notes: offer.notes.clone(),
            },
            UpdateAvailability::UpToDate => Self::UpToDate,
            UpdateAvailability::AheadOfChannel { installed, channel } => Self::AheadOfChannel {
                installed: installed.to_string(),
                channel: channel.to_string(),
            },
            UpdateAvailability::WithheldByRollout { version } => Self::WithheldByRollout {
                version: version.to_string(),
            },
            UpdateAvailability::ManagedElsewhere { version, manager } => Self::ManagedElsewhere {
                version: version.to_string(),
                manager: *manager,
            },
        }
    }
}

/// A recorded decision not to install yet.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateDeferralProjection {
    /// The version that was not installed.
    pub version: String,
    /// Why it was not installed.
    pub cause: DeferralCause,
}

impl UpdateDeferralProjection {
    /// Projects one recorded deferral.
    #[must_use]
    pub fn from_deferral(deferral: &Deferral) -> Self {
        Self {
            version: deferral.version.to_string(),
            cause: deferral.cause.clone(),
        }
    }
}

/// What the update is doing right now.
///
/// The authority reports this; it does not perform the download. Card 153 drew
/// that line for installation and it holds here: the host transfers and
/// reports, and a protocol that crossed it would make Longhorn responsible for
/// work it does not run.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum UpdateProgressProjection {
    /// Nothing in flight.
    Idle,
    /// Fetching the artifact.
    Downloading {
        /// How far through, when the source reports a length.
        ///
        /// Absent rather than zero when it does not. A source with no content
        /// length cannot produce a fraction, and a bar that invents one is
        /// worse than a bar that says it does not know.
        fraction: Option<f64>,
    },
    /// Checking the artifact before it is offered for install.
    Verifying,
    /// Downloaded and verified; waiting for the operator or for quiescence.
    ReadyToInstall {
        /// The version waiting.
        version: String,
    },
    /// The application is installing.
    Installing {
        /// The version being installed.
        version: String,
    },
}

/// One update state projection.
///
/// No timestamp. "Last checked at" needs a clock this crate does not own, and
/// none of the surfaces this protocol exists for asks for one; adding a
/// host-supplied stamp would drag a cross-domain dependency in for a field
/// nothing reads yet.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Live authority lifetime. A plain count, as the operation and
    /// notification domains carry it -- the history epoch type lives in
    /// `longhorn-history`, and depending on that domain for one integer would
    /// be a worse trade than restating it.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// The channel this install follows.
    pub channel: Channel,
    /// The version installed now.
    pub installed_version: String,
    /// The last check's outcome.
    pub availability: UpdateAvailabilityProjection,
    /// The standing deferral, when one applies.
    pub deferral: Option<UpdateDeferralProjection>,
    /// What is in flight.
    pub progress: UpdateProgressProjection,
}

/// Ask the source for the channel's current manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateCheckCommand {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
}

/// Follow a different channel from now on.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateSelectChannelCommand {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// The channel to follow.
    pub channel: Channel,
}

/// Decline a version for now.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateDeferCommand {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// The version being declined. Named, because a deferral covers the
    /// version it was taken against and nothing later.
    pub version: String,
    /// Why.
    pub cause: DeferralCause,
}

/// Authorize an install.
///
/// Longhorn authorizes; the application installs. Card 153 settled that, so
/// this returns a decision and never an installed state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateInstallCommand {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// The version to install.
    pub version: String,
}

/// The answer to an install request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum UpdateInstallAuthorizationProjection {
    /// Nothing is in flight; the application may install.
    Approved,
    /// Something is in flight. Not a failure, and the reason is the point:
    /// a refused restart that does not say why reads as a broken updater.
    Deferred {
        /// What was in flight.
        cause: DeferralCause,
    },
}

/// Why a consumer's held state is stale.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum UpdateChangedKind {
    /// A check completed.
    Checked,
    /// The selected channel changed.
    ChannelSelected,
    /// A version was deferred.
    Deferred,
    /// Progress moved.
    Progressed,
}

/// Non-durable invalidation hint.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: UpdateProtocolVersion,
    /// Live authority lifetime. A plain count, as the operation and
    /// notification domains carry it -- the history epoch type lives in
    /// `longhorn-history`, and depending on that domain for one integer would
    /// be a worse trade than restating it.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// Coarse invalidation category.
    pub kind: UpdateChangedKind,
}

/// Why an update command was refused.
///
/// Separate from `InstallFailure`, which is the installer's vocabulary and
/// says nothing about a stale caller or an absent offer. The controller maps
/// one onto the other rather than leaking the trait's error across the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum UpdateRejectionCode {
    /// The caller's authority lifetime is behind the live one.
    StaleAuthority,
    /// No such version is on offer. Covers an install request for a version
    /// that was never offered, one the last check superseded, and an
    /// externally managed install, which never reaches an offer at all.
    NoOffer,
    /// The source answered, and not with the artifact the manifest promised.
    Unavailable,
    /// The manifest fetched for the selected channel claims a different one.
    /// A mislabel silently restaging a rollout is an operational-integrity
    /// fault, so the manifest is refused rather than evaluated.
    ChannelMismatch,
    /// The transfer did not complete. Retryable.
    Unreachable,
    /// The artifact did not come from the signing key. Terminal, never
    /// retried, and never applied anyway.
    SignatureRejected,
    /// The installation cannot be written -- a Homebrew cask, or an
    /// administrator-installed copy. The remedy is a manual download.
    NotWritable,
    /// Replacement failed for another reason.
    InstallFailed,
}

/// The answer to an update command.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum UpdateOutcomeProjection {
    /// The command was applied. The snapshot is the state after it.
    Committed {
        /// The state the command produced.
        snapshot: UpdateSnapshot,
    },
    /// The command was refused, and the state is unchanged.
    Rejected {
        /// What a surface should tell the operator.
        code: UpdateRejectionCode,
        /// The state as it remains.
        snapshot: UpdateSnapshot,
    },
}
