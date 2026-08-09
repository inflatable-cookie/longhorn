//! Update policy acceptance evidence.

use longhorn_update::{
    BuildIdentity, Channel, ChannelManifest, CheckKind, Deferral, DeferralCause, InstallId,
    InstallManager, InstallProvenance, OfferReason, Rollout, RolloutFraction, UpdateAvailability,
    evaluate,
};
use semver::Version;

fn version(value: &str) -> Version {
    Version::parse(value).unwrap()
}

fn install(value: &str) -> InstallId {
    InstallId::new(value).unwrap()
}

fn fraction(value: f64) -> RolloutFraction {
    RolloutFraction::new(value).unwrap()
}

fn build(channel: Channel, value: &str) -> BuildIdentity {
    BuildIdentity::new(channel, version(value))
}

/// An install that a half rollout of `1.3.0` excludes, found rather than
/// assumed so the withheld cases test what they claim to.
fn excluded_install(rollout: &Rollout) -> InstallId {
    (0..1_000)
        .map(|index| install(&format!("install-{index}")))
        .find(|candidate| !rollout.includes(candidate))
        .expect("a half rollout must exclude someone")
}

fn included_install(rollout: &Rollout) -> InstallId {
    (0..1_000)
        .map(|index| install(&format!("install-{index}")))
        .find(|candidate| rollout.includes(candidate))
        .expect("a half rollout must include someone")
}

#[test]
fn a_matching_version_is_up_to_date() {
    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &ChannelManifest::new(Channel::Production, version("1.2.9")),
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::SelfManaged,
    );

    assert_eq!(availability, UpdateAvailability::UpToDate);
}

#[test]
fn a_newer_channel_version_is_offered() {
    let manifest =
        ChannelManifest::new(Channel::Production, version("1.3.0")).with_notes("fixes things");

    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &manifest,
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::SelfManaged,
    );

    let UpdateAvailability::Offer(offer) = availability else {
        panic!("a newer version must be offered, found {availability:?}");
    };
    assert_eq!(offer.version, version("1.3.0"));
    assert_eq!(offer.reason, OfferReason::Staged);
    assert_eq!(offer.notes.as_deref(), Some("fixes things"));
}

#[test]
fn an_install_ahead_of_its_channel_is_distinct_from_up_to_date() {
    // The channel-rejoin case: a nightly install selects production, which is
    // still behind it. Reporting UpToDate here would be a lie, and reporting
    // nothing at all reads as a broken updater.
    let availability = evaluate(
        &build(Channel::Production, "1.3.0-nightly.4"),
        &ChannelManifest::new(Channel::Production, version("1.2.9")),
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::SelfManaged,
    );

    assert_eq!(
        availability,
        UpdateAvailability::AheadOfChannel {
            installed: version("1.3.0-nightly.4"),
            channel: version("1.2.9"),
        }
    );
    assert_ne!(availability, UpdateAvailability::UpToDate);
}

#[test]
fn a_prerelease_rejoins_its_channel_when_the_release_lands() {
    // Semver prerelease ordering places 1.3.0-nightly.4 before 1.3.0, so the
    // rejoin needs no special handling at all -- it is the ordinary offer path.
    let availability = evaluate(
        &build(Channel::Production, "1.3.0-nightly.4"),
        &ChannelManifest::new(Channel::Production, version("1.3.0")),
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::SelfManaged,
    );

    let UpdateAvailability::Offer(offer) = availability else {
        panic!("the release must supersede its own prereleases");
    };
    assert_eq!(offer.version, version("1.3.0"));
}

#[test]
fn rollout_withholds_from_installs_outside_the_stage() {
    let rollout = Rollout::new(fraction(0.5), "1.3.0");
    let excluded = excluded_install(&rollout);
    let manifest =
        ChannelManifest::new(Channel::Production, version("1.3.0")).with_rollout(rollout.clone());

    assert_eq!(
        evaluate(
            &build(Channel::Production, "1.2.9"),
            &manifest,
            &excluded,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged,
        ),
        UpdateAvailability::WithheldByRollout {
            version: version("1.3.0")
        }
    );

    let included = included_install(&rollout);
    assert!(matches!(
        evaluate(
            &build(Channel::Production, "1.2.9"),
            &manifest,
            &included,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged,
        ),
        UpdateAvailability::Offer(_)
    ));
}

#[test]
fn a_user_initiated_check_bypasses_rollout() {
    let rollout = Rollout::new(fraction(0.5), "1.3.0");
    let excluded = excluded_install(&rollout);
    let manifest =
        ChannelManifest::new(Channel::Production, version("1.3.0")).with_rollout(rollout);

    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &manifest,
        &excluded,
        CheckKind::UserInitiated,
        InstallProvenance::SelfManaged,
    );

    let UpdateAvailability::Offer(offer) = availability else {
        panic!("asking explicitly must not be answered with silence");
    };
    assert_eq!(offer.reason, OfferReason::UserInitiated);
}

#[test]
fn the_minimum_version_floor_overrides_rollout() {
    // The security-fix lever: an install below the floor updates whether or
    // not the stage has reached it.
    let rollout = Rollout::new(fraction(0.5), "1.3.0");
    let excluded = excluded_install(&rollout);
    let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"))
        .with_rollout(rollout)
        .with_minimum_version(version("1.2.4"));

    let availability = evaluate(
        &build(Channel::Production, "1.2.3"),
        &manifest,
        &excluded,
        CheckKind::Automatic,
        InstallProvenance::SelfManaged,
    );

    let UpdateAvailability::Offer(offer) = availability else {
        panic!("a mandatory update must never be withheld by rollout");
    };
    assert_eq!(offer.reason, OfferReason::BelowMinimumVersion);
    assert!(!offer.reason.is_optional());
}

#[test]
fn an_install_at_the_floor_is_still_subject_to_rollout() {
    let rollout = Rollout::new(fraction(0.5), "1.3.0");
    let excluded = excluded_install(&rollout);
    let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"))
        .with_rollout(rollout)
        .with_minimum_version(version("1.2.4"));

    assert_eq!(
        evaluate(
            &build(Channel::Production, "1.2.4"),
            &manifest,
            &excluded,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged,
        ),
        UpdateAvailability::WithheldByRollout {
            version: version("1.3.0")
        }
    );
}

#[test]
fn faster_channels_are_never_staged() {
    // Beta and nightly exist to receive releases early. Staging them would
    // defeat the only reason to be on them.
    let rollout = Rollout::new(fraction(0.5), "1.3.0");
    let excluded = excluded_install(&rollout);

    for channel in [Channel::Beta, Channel::Nightly] {
        let manifest =
            ChannelManifest::new(channel, version("1.3.0")).with_rollout(rollout.clone());

        assert!(
            matches!(
                evaluate(
                    &build(channel, "1.2.9"),
                    &manifest,
                    &excluded,
                    CheckKind::Automatic,
                    InstallProvenance::SelfManaged,
                ),
                UpdateAvailability::Offer(_)
            ),
            "{channel} must not stage"
        );
    }
}

#[test]
fn evaluation_is_deterministic_for_the_same_inputs() {
    let rollout = Rollout::new(fraction(0.37), "1.3.0");
    let manifest =
        ChannelManifest::new(Channel::Production, version("1.3.0")).with_rollout(rollout);
    let build = build(Channel::Production, "1.2.9");

    for index in 0..200 {
        let install = install(&format!("install-{index}"));
        let first = evaluate(
            &build,
            &manifest,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged,
        );
        let second = evaluate(
            &build,
            &manifest,
            &install,
            CheckKind::Automatic,
            InstallProvenance::SelfManaged,
        );
        assert_eq!(first, second);
    }
}

#[test]
fn a_deferral_does_not_suppress_a_later_offer() {
    let deferral = Deferral::new(version("1.3.0"), DeferralCause::UserPostponed);

    assert!(deferral.applies_to(&version("1.3.0")));
    assert!(
        !deferral.applies_to(&version("1.3.1")),
        "declining one version must not silently strand the install on every later one"
    );
}

#[test]
fn availability_round_trips_through_its_wire_form() {
    let cases = [
        UpdateAvailability::UpToDate,
        UpdateAvailability::AheadOfChannel {
            installed: version("1.3.0-nightly.4"),
            channel: version("1.2.9"),
        },
        UpdateAvailability::WithheldByRollout {
            version: version("1.3.0"),
        },
    ];

    for case in cases {
        let json = serde_json::to_string(&case).unwrap();
        assert_eq!(
            serde_json::from_str::<UpdateAvailability>(&json).unwrap(),
            case
        );
    }
}

#[test]
fn an_externally_managed_install_is_told_where_to_update_not_offered_one() {
    // Card 168. Not `UpToDate` — that would be false, there *is* an update.
    // Not an offer either, because installing it would leave the package
    // manager's database describing a version no longer on disk.
    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &ChannelManifest::new(Channel::Production, version("1.3.0")),
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::ExternallyManaged {
            manager: InstallManager::HomebrewCask,
        },
    );

    assert_eq!(
        availability,
        UpdateAvailability::ManagedElsewhere {
            version: version("1.3.0"),
            manager: InstallManager::HomebrewCask,
        }
    );
}

#[test]
fn even_a_mandatory_release_is_not_self_installed_over_a_package_manager() {
    // The floor exists so a security release is never withheld. It still is
    // not withheld — the user is told where to get it — but urgency does not
    // make it safe to desync the manager, so the answer is not an offer.
    let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"))
        .with_minimum_version(version("1.2.10"));

    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &manifest,
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::ExternallyManaged {
            manager: InstallManager::LinuxDistribution,
        },
    );

    assert!(matches!(
        availability,
        UpdateAvailability::ManagedElsewhere { .. }
    ));
}

#[test]
fn an_undetermined_provenance_still_offers_so_ordinary_installs_do_not_regress() {
    // Every Windows layout today classifies as `Undetermined`. If that
    // blocked updates, this card would have broken more than it fixed.
    let availability = evaluate(
        &build(Channel::Production, "1.2.9"),
        &ChannelManifest::new(Channel::Production, version("1.3.0")),
        &install("any"),
        CheckKind::Automatic,
        InstallProvenance::Undetermined,
    );

    assert!(matches!(availability, UpdateAvailability::Offer(_)));
}

#[test]
fn an_external_deferral_is_not_retryable_and_names_the_command() {
    // A client surface must not say "we will try again" here. It will never
    // succeed, and the user has something to do instead.
    let cause = DeferralCause::ExternallyManaged {
        manager: InstallManager::HomebrewCask,
        command: InstallManager::HomebrewCask.upgrade_command("soundcheck"),
    };

    assert!(!cause.is_retryable());
    assert_eq!(
        cause.to_string(),
        "installed by Homebrew; update with `brew upgrade --cask soundcheck`"
    );
}
