//! Card 190. Exact wire evidence for the update protocol.

use longhorn_update::{
    Channel, Deferral, DeferralCause, InstallManager, OfferReason, UPDATE_PROTOCOL_VERSION,
    UpdateApplyCommand, UpdateAvailability, UpdateAvailabilityProjection, UpdateCancelCommand,
    UpdateChangedEvent, UpdateChangedKind, UpdateCheckCommand, UpdateDeferCommand,
    UpdateDeferralProjection, UpdateInstallAuthorizationProjection, UpdateOffer,
    UpdatePrepareCommand, UpdateProgressEvent, UpdateProgressProjection, UpdateProtocolVersion,
    UpdateSelectChannelCommand, UpdateSnapshot, UpdateStagedArtifactProjection,
};
use semver::Version;

fn version(value: &str) -> Version {
    Version::parse(value).expect("fixture version")
}

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value).expect("encode")).expect("decode")
}

/// The state that reads as a broken updater unless it is said out loud, and
/// the reason it is a variant rather than a flag on up-to-date.
#[test]
fn ahead_of_channel_projects_distinctly_from_up_to_date() {
    let ahead =
        UpdateAvailabilityProjection::from_availability(&UpdateAvailability::AheadOfChannel {
            installed: version("1.3.0-nightly.4"),
            channel: version("1.2.9"),
        });
    let current = UpdateAvailabilityProjection::from_availability(&UpdateAvailability::UpToDate);
    assert_ne!(ahead, current);

    let encoded = serde_json::to_value(&ahead).expect("encode");
    assert_eq!(encoded["state"], "aheadOfChannel");
    assert_eq!(encoded["installed"], "1.3.0-nightly.4");
    assert_eq!(encoded["channel"], "1.2.9");
    assert_eq!(
        serde_json::to_value(&current).expect("encode")["state"],
        "upToDate"
    );
    assert_eq!(round_trip(&ahead), ahead);
}

/// A source with no content length cannot produce a fraction. Absent, not
/// zero: a bar at zero says "nothing has happened", which is a different and
/// wrong claim.
#[test]
fn a_download_without_a_content_length_reports_no_fraction() {
    let unknown = UpdateProgressProjection::Downloading {
        received: 4_096,
        expected: None,
        fraction: None,
    };
    let known = UpdateProgressProjection::Downloading {
        received: 0,
        expected: Some(8_192),
        fraction: Some(0.0),
    };
    assert_ne!(unknown, known);

    let encoded = serde_json::to_value(&unknown).expect("encode");
    assert_eq!(encoded["state"], "downloading");
    assert_eq!(encoded["received"], 4_096);
    assert!(encoded["expected"].is_null());
    assert!(encoded["fraction"].is_null());
    assert_eq!(round_trip(&unknown), unknown);
}

#[test]
fn every_availability_variant_round_trips() {
    for availability in [
        UpdateAvailability::Offer(UpdateOffer {
            version: version("1.3.0"),
            reason: OfferReason::UserInitiated,
            notes: Some("Fixes the thing".to_owned()),
        }),
        UpdateAvailability::UpToDate,
        UpdateAvailability::AheadOfChannel {
            installed: version("1.3.0"),
            channel: version("1.2.9"),
        },
        UpdateAvailability::WithheldByRollout {
            version: version("1.3.0"),
        },
        UpdateAvailability::ManagedElsewhere {
            version: version("1.3.0"),
            manager: InstallManager::HomebrewCask,
        },
    ] {
        let projected = UpdateAvailabilityProjection::from_availability(&availability);
        assert_eq!(round_trip(&projected), projected);
    }
}

#[test]
fn every_progress_state_round_trips() {
    for progress in [
        UpdateProgressProjection::Idle,
        UpdateProgressProjection::Downloading {
            received: 27,
            expected: Some(54),
            fraction: Some(0.5),
        },
        UpdateProgressProjection::Downloading {
            received: 4_096,
            expected: None,
            fraction: None,
        },
        UpdateProgressProjection::Verifying,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.3.0".to_owned(),
        },
        UpdateProgressProjection::Installing {
            version: "1.3.0".to_owned(),
        },
    ] {
        assert_eq!(round_trip(&progress), progress);
    }
}

/// The channel that carries byte progress while the transfer runs. It is not
/// an invalidation hint: it carries the report itself, because a snapshot read
/// during the transfer still shows the retained state.
#[test]
fn a_live_progress_report_round_trips_with_its_epoch() {
    let event = UpdateProgressEvent {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 7,
        progress: UpdateProgressProjection::Downloading {
            received: 512,
            expected: Some(1_024),
            fraction: Some(0.5),
        },
    };
    assert_eq!(round_trip(&event), event);
    let encoded = serde_json::to_value(&event).expect("encode");
    assert_eq!(encoded["authorityEpoch"], 7);
    assert_eq!(encoded["progress"]["state"], "downloading");
}

#[test]
fn every_command_round_trips_and_carries_the_protocol_line() {
    let epoch = 7;
    let check = UpdateCheckCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
    };
    let select = UpdateSelectChannelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
        channel: Channel::Beta,
    };
    let defer = UpdateDeferCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
        version: "1.3.0".to_owned(),
        cause: DeferralCause::UserPostponed,
    };
    let prepared = UpdatePrepareCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
        version: "1.3.0".to_owned(),
    };
    let applied = UpdateApplyCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
        version: "1.3.0".to_owned(),
    };
    let cancelled = UpdateCancelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
    };
    assert_eq!(round_trip(&check), check);
    assert_eq!(round_trip(&select), select);
    assert_eq!(round_trip(&defer), defer);
    assert_eq!(round_trip(&prepared), prepared);
    assert_eq!(round_trip(&applied), applied);
    assert_eq!(round_trip(&cancelled), cancelled);
    assert_eq!(
        serde_json::to_value(&check).expect("encode")["protocolVersion"],
        UPDATE_PROTOCOL_VERSION
    );
}

/// A refused install is not a failure, and the reason is the whole point.
#[test]
fn a_refused_install_carries_its_cause() {
    let refused = UpdateInstallAuthorizationProjection::Deferred {
        cause: DeferralCause::WorkInFlight {
            detail: "one transfer session".to_owned(),
        },
    };
    let encoded = serde_json::to_value(&refused).expect("encode");
    assert_eq!(encoded["status"], "deferred");
    assert_eq!(encoded["cause"]["cause"], "workInFlight");
    assert_eq!(round_trip(&refused), refused);
}

#[test]
fn the_snapshot_round_trips_with_and_without_a_deferral() {
    let base = UpdateSnapshot {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 7,
        channel: Channel::Production,
        installed_version: "1.2.9".to_owned(),
        availability: UpdateAvailabilityProjection::UpToDate,
        deferral: None,
        staged: None,
        progress: UpdateProgressProjection::Idle,
    };
    assert_eq!(round_trip(&base), base);

    let deferred = UpdateSnapshot {
        deferral: Some(UpdateDeferralProjection::from_deferral(&Deferral::new(
            version("1.3.0"),
            DeferralCause::UserPostponed,
        ))),
        ..base
    };
    assert_eq!(round_trip(&deferred), deferred);
    assert_eq!(
        deferred.deferral.as_ref().expect("deferral").version,
        "1.3.0"
    );
}

/// The retained artifact is visible as its own identity, so a surface can say
/// what is waiting and `apply` can be held to it.
#[test]
fn a_staged_snapshot_projects_its_identity() {
    let staged = UpdateStagedArtifactProjection {
        version: "1.3.0".to_owned(),
        channel: Channel::Beta,
        digest: "ab".repeat(32),
    };
    let snapshot = UpdateSnapshot {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 7,
        channel: Channel::Production,
        installed_version: "1.2.9".to_owned(),
        availability: UpdateAvailabilityProjection::UpToDate,
        deferral: None,
        staged: Some(staged.clone()),
        progress: UpdateProgressProjection::ReadyToInstall {
            version: "1.3.0".to_owned(),
        },
    };

    let encoded = serde_json::to_value(&snapshot).expect("encode");
    assert_eq!(encoded["staged"]["version"], "1.3.0");
    assert_eq!(encoded["staged"]["channel"], "beta");
    assert_eq!(encoded["progress"]["state"], "readyToInstall");
    assert_eq!(round_trip(&snapshot), snapshot);
}

#[test]
fn the_changed_event_round_trips_every_kind() {
    for kind in [
        UpdateChangedKind::Checked,
        UpdateChangedKind::ChannelSelected,
        UpdateChangedKind::Deferred,
        UpdateChangedKind::Progressed,
    ] {
        let event = UpdateChangedEvent {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 7,
            kind,
        };
        assert_eq!(round_trip(&event), event);
    }
}
