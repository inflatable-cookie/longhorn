use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{BuildIdentity, ChannelManifest, InstallId, InstallManager, InstallProvenance};

/// Why an update check ran.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum CheckKind {
    /// A background or startup check.
    #[default]
    Automatic,
    /// The user asked.
    ///
    /// Bypasses rollout. Someone who opens the dialog and presses the button
    /// has opted in, and telling them "no update" while one exists is a
    /// worse failure than widening the stage by one install.
    UserInitiated,
}

impl CheckKind {
    /// Returns whether this check bypasses staged rollout.
    #[must_use]
    pub const fn bypasses_rollout(self) -> bool {
        matches!(self, Self::UserInitiated)
    }
}

/// Why an update is being offered.
///
/// camelCase, as every other string union in this domain is. It was
/// kebab-case and alone in that, which read as a typo to anyone mirroring the
/// type. Poodle mirrors this union structurally, so the two spellings had to
/// move together -- they landed either side of 2026-08-13 and the window
/// between them was red on `check:svelte`, which is the drift check working.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum OfferReason {
    /// The release is offered to everyone, or to this install's stage.
    Staged,
    /// The install is below the mandatory floor.
    ///
    /// Rollout does not apply; this update is not optional.
    BelowMinimumVersion,
    /// The user asked, so rollout was bypassed.
    UserInitiated,
}

impl OfferReason {
    /// Returns whether the user may reasonably decline.
    #[must_use]
    pub const fn is_optional(self) -> bool {
        !matches!(self, Self::BelowMinimumVersion)
    }
}

/// An available update.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOffer {
    /// The version offered.
    pub version: Version,
    /// Why it is offered.
    pub reason: OfferReason,
    /// Release notes, when published.
    pub notes: Option<String>,
}

/// The outcome of one update check.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum UpdateAvailability {
    /// An update is available now.
    Offer(UpdateOffer),
    /// The install is already on the channel's current version.
    UpToDate,
    /// The install is ahead of the channel it selected.
    ///
    /// Reached by switching from a faster channel to a slower one: an install
    /// on `1.3.0-nightly.4` that selects production sits ahead of production
    /// `1.2.9` and receives nothing until `1.3.0` ships. Correct, and
    /// indistinguishable from a broken updater unless it is said out loud —
    /// which is why this is its own state and not `UpToDate`.
    AheadOfChannel {
        /// What is installed.
        installed: Version,
        /// What the channel currently publishes.
        channel: Version,
    },
    /// A newer version exists but this install is not in the current stage.
    WithheldByRollout {
        /// The version being staged.
        version: Version,
    },
    /// A newer version exists, and a package manager owns this installation.
    ///
    /// Deliberately not `UpToDate`. There *is* an update and the user can
    /// have it — through the tool that installed the application. Reporting
    /// no update available would be false, and reporting an offer would
    /// invite an install that corrupts the manager's database.
    ///
    /// The surface derives the command from
    /// [`crate::InstallManager::upgrade_command`], because only the surface
    /// knows the application's package name.
    ManagedElsewhere {
        /// The version available through the manager.
        version: Version,
        /// Who owns the installation.
        manager: InstallManager,
    },
}

/// Decides what one update check should surface.
///
/// Pure: every input is explicit, and the same inputs always give the same
/// answer. Ordering matters and is deliberate — the mandatory floor is
/// checked before rollout so that a security release is never withheld.
#[must_use]
pub fn evaluate(
    build: &BuildIdentity,
    manifest: &ChannelManifest,
    install: &InstallId,
    check: CheckKind,
    provenance: InstallProvenance,
) -> UpdateAvailability {
    if manifest.version == build.version {
        return UpdateAvailability::UpToDate;
    }

    if manifest.version < build.version {
        return UpdateAvailability::AheadOfChannel {
            installed: build.version.clone(),
            channel: manifest.version.clone(),
        };
    }

    // Checked before every offer path, including the mandatory floor. A
    // security release is never withheld from the *user* — they are told
    // where to get it — but Longhorn cannot install it here whatever the
    // urgency, and offering an install that would desync the package manager
    // is not a way to make it more urgent.
    if let InstallProvenance::ExternallyManaged { manager } = provenance {
        return UpdateAvailability::ManagedElsewhere {
            version: manifest.version.clone(),
            manager,
        };
    }

    let offer = |reason| {
        UpdateAvailability::Offer(UpdateOffer {
            version: manifest.version.clone(),
            reason,
            notes: manifest.notes.clone(),
        })
    };

    if manifest.is_below_minimum(&build.version) {
        return offer(OfferReason::BelowMinimumVersion);
    }

    if check.bypasses_rollout() {
        return offer(OfferReason::UserInitiated);
    }

    match &manifest.rollout {
        // Rollout is a production-only mechanism. Beta and nightly exist to
        // receive releases early, so staging them would defeat the channel.
        Some(rollout) if manifest.channel.stages_rollout() && !rollout.includes(install) => {
            UpdateAvailability::WithheldByRollout {
                version: manifest.version.clone(),
            }
        }
        _ => offer(OfferReason::Staged),
    }
}
