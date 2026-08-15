use std::error::Error;

use longhorn_update::{
    Channel, DeferralCause, InstallManager, OfferReason, UpdateAvailabilityProjection,
    UpdateChangedEvent, UpdateChangedKind, UpdateCheckCommand, UpdateDeferCommand,
    UpdateDeferralProjection, UpdateInstallCommand, UpdateOutcomeProjection,
    UpdateProgressProjection, UpdateProtocolVersion, UpdateRejectionCode,
    UpdateSelectChannelCommand, UpdateSnapshot,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let epoch = 7;
    let snapshot = UpdateSnapshot {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: epoch,
        channel: Channel::Nightly,
        installed_version: "1.3.0-nightly.4".into(),
        availability: UpdateAvailabilityProjection::Offer {
            version: "1.3.0-nightly.5".into(),
            reason: OfferReason::UserInitiated,
            notes: Some("Fixture release notes.".into()),
        },
        deferral: Some(UpdateDeferralProjection {
            version: "1.3.0-nightly.2".into(),
            cause: DeferralCause::UserPostponed,
        }),
        progress: UpdateProgressProjection::Downloading {
            fraction: Some(0.5),
        },
    };
    // The two availability states a surface most needs to tell apart from a
    // broken updater, plus the idle and waiting progress states.
    let managed_snapshot = UpdateSnapshot {
        channel: Channel::Production,
        installed_version: "1.2.9".into(),
        availability: UpdateAvailabilityProjection::ManagedElsewhere {
            version: "1.3.0".into(),
            manager: InstallManager::HomebrewCask,
        },
        deferral: None,
        progress: UpdateProgressProjection::Idle,
        ..snapshot.clone()
    };
    let ahead_snapshot = UpdateSnapshot {
        availability: UpdateAvailabilityProjection::AheadOfChannel {
            installed: "1.3.0-nightly.4".into(),
            channel: "1.2.9".into(),
        },
        deferral: None,
        progress: UpdateProgressProjection::ReadyToInstall {
            version: "1.3.0-nightly.5".into(),
        },
        ..snapshot.clone()
    };
    let withheld_snapshot = UpdateSnapshot {
        availability: UpdateAvailabilityProjection::WithheldByRollout {
            version: "1.3.0".into(),
        },
        deferral: None,
        progress: UpdateProgressProjection::Verifying,
        ..snapshot.clone()
    };
    let outcomes = [
        UpdateOutcomeProjection::Committed {
            snapshot: snapshot.clone(),
        },
        // Every code, one rejection each -- channelMismatch included. A code
        // with no fixture is a code the TypeScript side could stop recognising
        // without anything failing.
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::StaleAuthority,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::NoOffer,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::Unavailable,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::ChannelMismatch,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::Unreachable,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::SignatureRejected,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::NotWritable,
            snapshot: snapshot.clone(),
        },
        UpdateOutcomeProjection::Rejected {
            code: UpdateRejectionCode::InstallFailed,
            snapshot: snapshot.clone(),
        },
    ];
    let fixture = json!({
        "protocolVersion": 1,
        "snapshot": to_value(&snapshot)?,
        "managedSnapshot": to_value(managed_snapshot)?,
        "aheadSnapshot": to_value(ahead_snapshot)?,
        "withheldSnapshot": to_value(withheld_snapshot)?,
        "upToDateSnapshot": to_value(UpdateSnapshot {
            availability: UpdateAvailabilityProjection::UpToDate,
            deferral: None,
            progress: UpdateProgressProjection::Installing {
                version: "1.3.0".into(),
            },
            ..snapshot.clone()
        })?,
        "checkCommand": to_value(UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: epoch,
        })?,
        "selectChannelCommand": to_value(UpdateSelectChannelCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: epoch,
            channel: Channel::Beta,
        })?,
        "deferCommand": to_value(UpdateDeferCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: epoch,
            version: "1.3.0-nightly.5".into(),
            cause: DeferralCause::WorkInFlight {
                detail: "transfer session open".into(),
            },
        })?,
        "installCommand": to_value(UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: epoch,
            version: "1.3.0-nightly.5".into(),
        })?,
        "outcomes": outcomes.map(to_value).into_iter().collect::<Result<Vec<_>, _>>()?,
        "changedEvent": to_value(UpdateChangedEvent {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: epoch,
            kind: UpdateChangedKind::Checked,
        })?,
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownChannel": "canary",
            "unknownOfferReason": "mandatory",
            "unknownInstallManager": "macports",
            "unknownDeferralCause": "postponedByPolicy",
            "unknownAvailabilityState": "pending",
            "unknownProgressState": "seeding",
            "unknownRejectionCode": "futureRejection",
            "unknownOutcomeStatus": "uncertain",
            "unknownChangedKind": "futureKind"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}
