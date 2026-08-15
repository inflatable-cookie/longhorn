use std::error::Error;

use longhorn_licence::{
    HeldLicenceProjection, LicenceActivateCommand, LicenceChangedEvent, LicenceChangedKind,
    LicenceCredentialProjection, LicenceDeactivateCommand, LicenceEntitlementProjection,
    LicenceOutcomeProjection, LicenceProtocolVersion, LicenceRefreshCommand, LicenceRejectionCode,
    LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSeatProjection, LicenceSnapshot,
    LicenceTrustBasisProjection, LicenceUsabilityProjection, Timestamp,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let epoch = 7;
    let held = HeldLicenceProjection {
        product: "longhorn".into(),
        usability: LicenceUsabilityProjection::Active,
        trust_basis: LicenceTrustBasisProjection::OfflineSignature,
        entitlements: vec![
            LicenceEntitlementProjection {
                id: "pro".into(),
                at_most: None,
            },
            LicenceEntitlementProjection {
                id: "seats".into(),
                at_most: Some(5),
            },
        ],
        use_until: Some(Timestamp::from_unix_seconds(1_800_000_000)),
        update_until: Some(Timestamp::from_unix_seconds(1_790_000_000)),
        seats: vec![
            LicenceSeatProjection {
                machine_id: "m-fixture-this-machine".into(),
                label: Some("Studio iMac".into()),
                this_machine: true,
            },
            LicenceSeatProjection {
                machine_id: "m-fixture-old-macbook".into(),
                label: None,
                this_machine: false,
            },
        ],
    };
    let snapshot = LicenceSnapshot {
        protocol_version: LicenceProtocolVersion::CURRENT,
        authority_epoch: epoch,
        licence: Some(held),
    };
    // Grace with a remote trust basis: the timestamped variants and the second
    // trust-basis kind need a fixture as much as the unit ones do.
    let grace_snapshot = LicenceSnapshot {
        licence: Some(HeldLicenceProjection {
            usability: LicenceUsabilityProjection::InGrace {
                until: Timestamp::from_unix_seconds(1_785_000_000),
            },
            trust_basis: LicenceTrustBasisProjection::RemoteAssertion {
                checked: Timestamp::from_unix_seconds(1_784_999_400),
            },
            seats: Vec::new(),
            ..snapshot.licence.clone().expect("held")
        }),
        ..snapshot.clone()
    };
    let unlicensed_snapshot = LicenceSnapshot::unlicensed(epoch);
    let activate_commands = [
        LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            credential: LicenceCredentialProjection::Key {
                key: "ABCDE-12345-FGHJK-6789X".into(),
            },
            label: Some("Studio iMac".into()),
        },
        LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            credential: LicenceCredentialProjection::AccountToken {
                token: "account-token-fixture".into(),
            },
            label: None,
        },
        LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            credential: LicenceCredentialProjection::LicenceFile {
                contents_base64: "AAECAwQ=".into(),
            },
            label: None,
        },
    ];
    let outcomes = [
        LicenceOutcomeProjection::Committed {
            snapshot: snapshot.clone(),
        },
        // Every code, one rejection each. A code with no fixture is a code the
        // TypeScript side could stop recognising without anything failing.
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::Malformed,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::NotRecognised,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::NoSeatsFree,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::Revoked,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::ClockRefused,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::StaleAuthority,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::SeatNotFound,
            snapshot: snapshot.clone(),
        },
        LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::Unreachable,
            snapshot: snapshot.clone(),
        },
    ];
    let fixture = json!({
        "protocolVersion": 1,
        "snapshot": to_value(&snapshot)?,
        "graceSnapshot": to_value(grace_snapshot)?,
        "unlicensedSnapshot": to_value(unlicensed_snapshot)?,
        "activateCommands": activate_commands.map(to_value).into_iter().collect::<Result<Vec<_>, _>>()?,
        "deactivateCommand": to_value(LicenceDeactivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
        })?,
        "refreshCommand": to_value(LicenceRefreshCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
        })?,
        "releaseSeatCommand": to_value(LicenceReleaseSeatCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            machine_id: "m-fixture-old-macbook".into(),
        })?,
        "renameSeatCommand": to_value(LicenceRenameSeatCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            machine_id: "m-fixture-old-macbook".into(),
            label: Some("The old MacBook".into()),
        })?,
        "outcomes": outcomes.map(to_value).into_iter().collect::<Result<Vec<_>, _>>()?,
        "changedEvent": to_value(LicenceChangedEvent {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: epoch,
            kind: LicenceChangedKind::Activated,
        })?,
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownUsabilityState": "suspended",
            "unknownTrustBasisKind": "notarized",
            "unknownCredentialKind": "passkey",
            "unknownRejectionCode": "futureRejection",
            "unknownOutcomeStatus": "uncertain",
            "unknownChangedKind": "futureKind"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}
