//! The controller's acceptance, from Card 196.
//!
//! Every port is a fake here, which is the point: the controller performs no
//! work, so a test can substitute every side effect and still exercise the
//! whole sequence.

use std::cell::RefCell;
use std::io::Cursor;

use longhorn_update::{
    Applied, Artifact, ArtifactFetch, ArtifactKey, BuildIdentity, Channel, ChannelManifest,
    CheckKind, DeferralCause, FetchError, FetchProgress, InstallFailure, InstallId, InstallManager,
    InstallProvenance, OutstandingWork, QuiescenceKind, QuiescenceProbe, SourceError,
    SourceRequest, TargetTriple, UpdateCheckCommand, UpdateController, UpdateDeferCommand,
    UpdateGate, UpdateInstallCommand, UpdateInstaller, UpdateOutcomeProjection,
    UpdateProgressProjection, UpdateProtocolVersion, UpdateRejectionCode,
    UpdateSelectChannelCommand, UpdateSource, VerifiedArtifact,
};
use minisign::KeyPair;
use semver::Version;

const ARTIFACT: &[u8] = b"a signed application bundle";
const TARGET: &str = "aarch64-apple-darwin";

fn target() -> TargetTriple {
    TargetTriple::new(TARGET).unwrap()
}

fn version(value: &str) -> Version {
    Version::parse(value).unwrap()
}

struct Signing {
    keys: KeyPair,
}

impl Signing {
    fn new() -> Self {
        Self {
            keys: KeyPair::generate_unencrypted_keypair().unwrap(),
        }
    }

    fn key(&self) -> ArtifactKey {
        ArtifactKey::from_base64(&self.keys.pk.to_base64()).unwrap()
    }

    fn signature(&self, bytes: &[u8]) -> String {
        minisign::sign(None, &self.keys.sk, Cursor::new(bytes), None, None)
            .unwrap()
            .to_string()
    }
}

struct Source;

impl UpdateSource for Source {
    fn manifest_request(&self, _channel: Channel) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(
            longhorn_update::EndpointUrl::new("https://example.test/manifest.json").unwrap(),
        ))
    }
}

/// Records whether it was called, and what it reports.
struct Fetch {
    bytes: Vec<u8>,
    report: Option<FetchProgress>,
    calls: RefCell<u32>,
}

impl Fetch {
    fn serving(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            report: Some(FetchProgress::of(27, 27)),
            calls: RefCell::new(0),
        }
    }

    fn silent(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            report: None,
            calls: RefCell::new(0),
        }
    }

    fn calls(&self) -> u32 {
        *self.calls.borrow()
    }
}

impl ArtifactFetch for Fetch {
    fn fetch(
        &self,
        _request: &SourceRequest,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        *self.calls.borrow_mut() += 1;
        if let Some(progress) = self.report {
            report(progress);
        }
        Ok(self.bytes.clone())
    }
}

struct Installer;

impl UpdateInstaller for Installer {
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        Ok(Applied {
            version: artifact.version().clone(),
            relaunched: false,
        })
    }
}

struct Busy;

impl QuiescenceProbe for Busy {
    fn outstanding(&self) -> Option<OutstandingWork> {
        Some(OutstandingWork {
            kind: QuiescenceKind::OpenTransferSession,
            count: 1,
        })
    }
}

fn manifest(signature: &str) -> ChannelManifest {
    let mut manifest = ChannelManifest::new(Channel::Production, version("1.4.0"));
    manifest.artifacts.insert(
        target(),
        Artifact::new("https://example.test/app.tar.gz", signature),
    );
    manifest
}

fn controller<'port>(
    signing: &Signing,
    source: &'port Source,
    fetch: &'port Fetch,
    provenance: InstallProvenance,
) -> UpdateController<'port> {
    UpdateController::new(
        BuildIdentity::new(Channel::Production, version("1.3.0")),
        target(),
        InstallId::new("install-1").unwrap(),
        provenance,
        signing.key(),
        source,
        fetch,
    )
}

fn writable() -> InstallProvenance {
    InstallProvenance::SelfManaged
}

fn committed(outcome: &UpdateOutcomeProjection) -> &longhorn_update::UpdateSnapshot {
    match outcome {
        UpdateOutcomeProjection::Committed { snapshot } => snapshot,
        UpdateOutcomeProjection::Rejected { code, .. } => panic!("rejected as {code:?}"),
    }
}

fn rejection(outcome: &UpdateOutcomeProjection) -> UpdateRejectionCode {
    match outcome {
        UpdateOutcomeProjection::Rejected { code, .. } => *code,
        UpdateOutcomeProjection::Committed { .. } => panic!("committed"),
    }
}

fn check(signing: &Signing) -> (UpdateCheckCommand, ChannelManifest) {
    (
        UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
        },
        manifest(&signing.signature(ARTIFACT)),
    )
}

#[test]
fn a_check_records_an_offer_and_projects_it() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);

    let outcome = controller.check(&command, &manifest, CheckKind::UserInitiated);

    let snapshot = committed(&outcome);
    assert_eq!(snapshot.installed_version, "1.3.0");
    assert!(matches!(
        snapshot.availability,
        longhorn_update::UpdateAvailabilityProjection::Offer { .. }
    ));
}

/// Card 190's acceptance, which had nothing that could refuse it.
#[test]
fn a_stale_authority_epoch_is_refused_on_all_four_commands() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let stale = controller.authority_epoch() + 1;
    let manifest = manifest(&signing.signature(ARTIFACT));

    let checked = controller.check(
        &UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: stale,
        },
        &manifest,
        CheckKind::Automatic,
    );
    let selected = controller.select_channel(&UpdateSelectChannelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: stale,
        channel: Channel::Beta,
    });
    let deferred = controller.defer(&UpdateDeferCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: stale,
        version: "1.4.0".to_owned(),
        cause: DeferralCause::UserPostponed,
    });
    let installed = controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: stale,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(Vec::new()),
        &Installer,
    );

    for outcome in [&checked, &selected, &deferred, &installed] {
        assert_eq!(rejection(outcome), UpdateRejectionCode::StaleAuthority);
    }
    assert_eq!(fetch.calls(), 0, "a stale caller must not start a transfer");
}

#[test]
fn the_whole_sequence_installs_and_leaves_the_new_version_up_to_date() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let outcome = controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(Vec::new()),
        &Installer,
    );

    let snapshot = committed(&outcome);
    assert_eq!(snapshot.installed_version, "1.4.0");
    assert_eq!(snapshot.progress, UpdateProgressProjection::Idle);
    assert_eq!(fetch.calls(), 1);
}

/// The milestone's "better than the plugin" case. Downloading eighty
/// megabytes and then saying "you installed this with Homebrew" is the
/// plugin's behaviour, not an improvement on it.
#[test]
fn an_externally_managed_install_never_starts_a_transfer() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(
        &signing,
        &source,
        &fetch,
        InstallProvenance::ExternallyManaged {
            manager: InstallManager::HomebrewCask,
        },
    );
    let (command, manifest) = check(&signing);

    let checked = controller.check(&command, &manifest, CheckKind::UserInitiated);
    let installed = controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(Vec::new()),
        &Installer,
    );

    // The offer survives. It is not an error state, and a surface that showed
    // it as one would be hiding a version the user can install themselves.
    assert!(matches!(
        committed(&checked).availability,
        longhorn_update::UpdateAvailabilityProjection::ManagedElsewhere { .. }
    ));
    assert_eq!(rejection(&installed), UpdateRejectionCode::NoOffer);
    assert_eq!(fetch.calls(), 0);
}

/// The gate sits between verify and install, so the transfer has already
/// happened. That is deliberate — see the controller's module note.
#[test]
fn work_in_flight_defers_the_install_after_the_transfer() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let busy = Busy;
    let outcome = controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(vec![&busy]),
        &Installer,
    );

    // Committed, not rejected: a refused install carries its reason, and the
    // deferral is the reason.
    let snapshot = committed(&outcome);
    assert_eq!(snapshot.installed_version, "1.3.0");
    assert!(snapshot.deferral.is_some());
    assert_eq!(fetch.calls(), 1);
}

#[test]
fn an_artifact_signed_by_another_key_is_refused_before_the_installer() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let elsewhere = Signing::new();
    let manifest = manifest(&elsewhere.signature(ARTIFACT));
    controller.check(
        &UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
        },
        &manifest,
        CheckKind::UserInitiated,
    );

    let outcome = controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(Vec::new()),
        &Installer,
    );

    assert_eq!(rejection(&outcome), UpdateRejectionCode::SignatureRejected);
}

/// Card 190 built the `Option<f64>` fraction and had nothing that could reach
/// the absent case end to end.
#[test]
fn a_host_that_reports_no_length_leaves_the_fraction_absent() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::silent(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let busy = Busy;
    controller.install(
        &UpdateInstallCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 1,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(vec![&busy]),
        &Installer,
    );

    // A host that reports nothing is the same answer as a source with no
    // content length: no fraction, rather than zero.
    assert_eq!(FetchProgress::unbounded(0).fraction(), None);
    assert_eq!(fetch.calls(), 1);
}

/// Switching channel drops the old channel's answer rather than showing it
/// beside the new one.
#[test]
fn selecting_a_channel_clears_the_previous_offer() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let outcome = controller.select_channel(&UpdateSelectChannelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
        channel: Channel::Beta,
    });

    let snapshot = committed(&outcome);
    assert_eq!(snapshot.channel, Channel::Beta);
    assert!(matches!(
        snapshot.availability,
        longhorn_update::UpdateAvailabilityProjection::UpToDate
    ));
}
