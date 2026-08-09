//! Projects an update decision into a banner.
//!
//! Like licence, this domain has no Svelte counterpart, so there is no port
//! to compare against.
//!
//! The interesting states are the two that are neither "you are current" nor
//! "here is an update": `AheadOfChannel` and `ManagedElsewhere`. Both are
//! correct outcomes that look like a broken updater unless the surface says
//! what happened, which is the whole reason `longhorn-update` gave each its
//! own variant instead of folding them into `UpToDate`.

use longhorn_update::{OfferReason, UpdateAvailability, UpdateOffer};
use poodle_specs::{BannerSpec, StatusTone};

/// What a surface should show for one update decision.
///
/// `package` names the application to a package manager — `soundcheck-app`,
/// not "Soundcheck" — because only the surface knows it. When the manager has
/// no upgrade command (the App Store, an AppImage, a distribution package)
/// the banner says who owns the install and stops there, rather than inventing
/// an instruction that would not work.
#[must_use]
pub fn availability_banner(availability: &UpdateAvailability, package: &str) -> Option<BannerSpec> {
    match availability {
        // Nothing to say. A banner reading "you are up to date" is noise on
        // every launch, and the check's result is available without one.
        UpdateAvailability::UpToDate => None,

        UpdateAvailability::Offer(offer) => Some(
            BannerSpec::new()
                .with_tone(offer_tone(offer))
                .with_title(format!("Version {} is available", offer.version))
                .with_message(offer_message(offer))
                .with_dismissible(offer.reason.is_optional()),
        ),

        // Correct, and indistinguishable from a broken updater unless said
        // out loud: an install on a newer prerelease that has selected a
        // slower channel receives nothing until that channel catches up.
        UpdateAvailability::AheadOfChannel { installed, channel } => Some(
            BannerSpec::new()
                .with_tone(StatusTone::Info)
                .with_title("Ahead of this channel")
                .with_message(format!(
                    "Version {installed} is installed and this channel publishes {channel}. \
                     No update will arrive until the channel catches up."
                )),
        ),

        // A newer version exists and this install is not in the current
        // stage. Saying so beats silence, which reads as a broken check.
        UpdateAvailability::WithheldByRollout { version } => Some(
            BannerSpec::new()
                .with_tone(StatusTone::Info)
                .with_title(format!("Version {version} is rolling out"))
                .with_message("It is not yet available to this installation. No action is needed."),
        ),

        // There *is* an update and the user can have it — through the tool
        // that installed the application. Offering an in-app install here
        // would corrupt the manager's database, which is the live defect
        // Card 168 found.
        UpdateAvailability::ManagedElsewhere { version, manager } => {
            let title = format!("Version {version} is available from {manager}");
            let message = manager.upgrade_command(package).map_or_else(
                || {
                    format!(
                        "{manager} manages this installation, so the update is installed there \
                         rather than from inside the application."
                    )
                },
                |command| format!("Run `{command}` to install it."),
            );

            Some(
                BannerSpec::new()
                    .with_tone(StatusTone::Info)
                    .with_title(title)
                    .with_message(message),
            )
        }
    }
}

/// A mandatory update is a warning; an optional one is information.
///
/// Not `Danger`. The application still works, and the loudest tone available
/// should be kept for a state that has actually stopped working — see the
/// licence domain, where it means exactly that.
fn offer_tone(offer: &UpdateOffer) -> StatusTone {
    if offer.reason.is_optional() {
        StatusTone::Info
    } else {
        StatusTone::Warning
    }
}

/// The offer's own words, which depend on why it was made rather than on the
/// version.
///
/// `UserInitiated` is worth its own sentence: the user asked, so confirming
/// that the check ran and found something is the answer to their question.
fn offer_message(offer: &UpdateOffer) -> String {
    match offer.reason {
        OfferReason::BelowMinimumVersion => {
            "This update is required and cannot be postponed.".to_owned()
        }
        OfferReason::UserInitiated => "You asked, and this is what is available.".to_owned(),
        OfferReason::Staged => "Install it when convenient.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_update::InstallManager;
    use semver::Version;

    fn version(value: &str) -> Version {
        Version::parse(value).expect("version")
    }

    fn offer(reason: OfferReason) -> UpdateAvailability {
        UpdateAvailability::Offer(UpdateOffer {
            version: version("1.3.0"),
            reason,
            notes: None,
        })
    }

    #[test]
    fn being_current_says_nothing() {
        // A banner on every launch reading "you are up to date" is noise.
        assert!(availability_banner(&UpdateAvailability::UpToDate, "app").is_none());
    }

    #[test]
    fn a_mandatory_update_is_a_warning_and_cannot_be_dismissed() {
        let banner =
            availability_banner(&offer(OfferReason::BelowMinimumVersion), "app").expect("banner");

        assert_eq!(banner.tone, StatusTone::Warning);
        assert!(!banner.is_dismissible);
    }

    #[test]
    fn an_optional_update_is_information_and_can_be_dismissed() {
        for reason in [OfferReason::Staged, OfferReason::UserInitiated] {
            let banner = availability_banner(&offer(reason), "app").expect("banner");
            assert_eq!(banner.tone, StatusTone::Info, "{reason:?}");
            assert!(banner.is_dismissible, "{reason:?}");
        }
    }

    #[test]
    fn a_managed_install_is_told_the_command_that_actually_works() {
        let banner = availability_banner(
            &UpdateAvailability::ManagedElsewhere {
                version: version("1.3.0"),
                manager: InstallManager::HomebrewCask,
            },
            "soundcheck",
        )
        .expect("banner");

        let message = banner.message.expect("message");
        assert!(
            message.contains("brew upgrade --cask soundcheck"),
            "{message}"
        );
    }

    #[test]
    fn a_manager_with_no_command_names_the_owner_instead_of_inventing_one() {
        // The App Store has no upgrade command. Printing a fake one would be
        // worse than saying who owns the install.
        let banner = availability_banner(
            &UpdateAvailability::ManagedElsewhere {
                version: version("1.3.0"),
                manager: InstallManager::MacAppStore,
            },
            "soundcheck",
        )
        .expect("banner");

        let message = banner.message.expect("message");
        assert!(!message.contains('`'), "{message}");
        assert!(message.contains("manages this installation"), "{message}");
    }

    #[test]
    fn being_ahead_of_the_channel_names_both_versions() {
        // Correct, and indistinguishable from a broken updater unless both
        // numbers are on screen.
        let banner = availability_banner(
            &UpdateAvailability::AheadOfChannel {
                installed: version("1.3.0-nightly.4"),
                channel: version("1.2.9"),
            },
            "app",
        )
        .expect("banner");

        let message = banner.message.expect("message");
        assert!(message.contains("1.3.0-nightly.4"), "{message}");
        assert!(message.contains("1.2.9"), "{message}");
    }

    #[test]
    fn a_withheld_rollout_says_no_action_is_needed() {
        let banner = availability_banner(
            &UpdateAvailability::WithheldByRollout {
                version: version("1.3.0"),
            },
            "app",
        )
        .expect("banner");

        assert_eq!(banner.tone, StatusTone::Info);
        assert!(banner.title.expect("title").contains("1.3.0"));
    }
}
