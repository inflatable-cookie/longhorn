//! The controller's acceptance, from Card 196 and the 2026-09-22 staged
//! amendment.
//!
//! Every port is a fake here, which is the point: the controller performs no
//! work, so a test can substitute every side effect and still exercise the
//! whole sequence.

use std::cell::{Cell, RefCell};
use std::io::Cursor;

use longhorn_update::{
    AdmissionAuthority, AdmissionLease, AdmissionRefusal, Applied, Artifact, ArtifactFetch,
    ArtifactKey, BuildIdentity, Channel, ChannelManifest, CheckKind, DeferralCause, FetchError,
    FetchProgress, InstallFailure, InstallId, InstallManager, InstallProvenance, OutstandingWork,
    QuiescenceKind, QuiescenceProbe, SourceError, SourceRequest, TargetTriple, UpdateApplyCommand,
    UpdateCancelCommand, UpdateCheckCommand, UpdateController, UpdateDeferCommand, UpdateGate,
    UpdateInstaller, UpdateOutcomeProjection, UpdatePrepareCommand, UpdatePrepareStart,
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
        _limit: u64,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        *self.calls.borrow_mut() += 1;
        if let Some(progress) = self.report {
            report(progress);
        }
        Ok(self.bytes.clone())
    }
}

/// Reports several progress values from inside the transfer, and records
/// whether the observer ran while the transfer was on the stack.
struct Streaming<'flag> {
    bytes: Vec<u8>,
    reports: Vec<FetchProgress>,
    calls: RefCell<u32>,
    transferring: &'flag Cell<bool>,
}

impl ArtifactFetch for Streaming<'_> {
    fn fetch(
        &self,
        _request: &SourceRequest,
        _limit: u64,
        report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        *self.calls.borrow_mut() += 1;
        self.transferring.set(true);
        for progress in &self.reports {
            report(*progress);
        }
        self.transferring.set(false);
        Ok(self.bytes.clone())
    }
}

/// Fails the transfer itself, so a typed fetch failure is reachable.
struct Broken {
    error: FetchError,
    calls: RefCell<u32>,
}

impl ArtifactFetch for Broken {
    fn fetch(
        &self,
        _request: &SourceRequest,
        _limit: u64,
        _report: &mut dyn FnMut(FetchProgress),
    ) -> Result<Vec<u8>, FetchError> {
        *self.calls.borrow_mut() += 1;
        Err(self.error.clone())
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

/// An installer that refuses, to prove the staged artifact survives a failed
/// replacement.
struct Refusing;

impl UpdateInstaller for Refusing {
    fn apply(&self, _artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        Err(InstallFailure::NotWritable {
            detail: "read-only bundle".to_owned(),
        })
    }
}

/// Grants every lease. The sequencing tests are about the controller, not the
/// barrier; the barrier's own behaviour is proved in `interlock.rs` and in the
/// lease-lifetime tests at the end of this file.
struct Granting;

impl AdmissionAuthority for Granting {
    fn acquire(&self) -> Result<Box<dyn AdmissionLease + '_>, AdmissionRefusal> {
        Ok(Box::new(Granted))
    }
}

/// The lease `Granting` hands out. It blocks nothing, which is why the tests
/// that check the barrier do not use it.
struct Granted;

impl AdmissionLease for Granted {}

/// One shared instance, so a gate can borrow it for `'static` without a local
/// binding in every test.
const GRANTING: Granting = Granting;

/// Grants one exclusive lease at a time, so an acquisition from inside the
/// installer is refused while the controller holds the first.
struct ExclusiveAuthority {
    held: Cell<bool>,
    acquisitions: Cell<u32>,
    refusals: Cell<u32>,
    releases: Cell<u32>,
}

impl ExclusiveAuthority {
    fn new() -> Self {
        Self {
            held: Cell::new(false),
            acquisitions: Cell::new(0),
            refusals: Cell::new(0),
            releases: Cell::new(0),
        }
    }
}

impl AdmissionAuthority for ExclusiveAuthority {
    fn acquire(&self) -> Result<Box<dyn AdmissionLease + '_>, AdmissionRefusal> {
        if self.held.get() {
            self.refusals.set(self.refusals.get() + 1);
            return Err(AdmissionRefusal::new("a write is in flight"));
        }
        self.held.set(true);
        self.acquisitions.set(self.acquisitions.get() + 1);
        Ok(Box::new(ExclusiveLease { authority: self }))
    }
}

/// The one lease an [`ExclusiveAuthority`] grants; dropping it is the release.
struct ExclusiveLease<'authority> {
    authority: &'authority ExclusiveAuthority,
}

impl AdmissionLease for ExclusiveLease<'_> {}

impl Drop for ExclusiveLease<'_> {
    fn drop(&mut self) {
        self.authority.held.set(false);
        self.authority
            .releases
            .set(self.authority.releases.get() + 1);
    }
}

/// Never grants, so the unacquirable-lease path is reachable.
struct NoAdmission;

impl AdmissionAuthority for NoAdmission {
    fn acquire(&self) -> Result<Box<dyn AdmissionLease + '_>, AdmissionRefusal> {
        Err(AdmissionRefusal::new("an index rebuild is running"))
    }
}

/// Records whether replacement ran and what the barrier looked like while it
/// did. The acquisition attempt inside `apply` stands in for the new
/// conflicting work the lease exists to refuse.
struct LeaseWatchingInstaller<'authority> {
    authority: &'authority ExclusiveAuthority,
    calls: Cell<u32>,
    conflicting_work_refused: Cell<bool>,
    conflicting_work_started: Cell<bool>,
}

impl<'authority> LeaseWatchingInstaller<'authority> {
    fn new(authority: &'authority ExclusiveAuthority) -> Self {
        Self {
            authority,
            calls: Cell::new(0),
            conflicting_work_refused: Cell::new(false),
            conflicting_work_started: Cell::new(false),
        }
    }
}

impl UpdateInstaller for LeaseWatchingInstaller<'_> {
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        self.calls.set(self.calls.get() + 1);
        match self.authority.acquire() {
            Ok(_) => self.conflicting_work_started.set(true),
            Err(_) => self.conflicting_work_refused.set(true),
        }
        Ok(Applied {
            version: artifact.version().clone(),
            relaunched: false,
        })
    }
}

/// Counts calls, so a test can prove the installer was never reached.
struct CountingInstaller {
    calls: Cell<u32>,
}

impl CountingInstaller {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }
}

impl UpdateInstaller for CountingInstaller {
    fn apply(&self, artifact: &VerifiedArtifact) -> Result<Applied, InstallFailure> {
        self.calls.set(self.calls.get() + 1);
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
    fetch: &'port dyn ArtifactFetch,
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

fn prepare(version: &str) -> UpdatePrepareCommand {
    UpdatePrepareCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
        version: version.to_owned(),
    }
}

fn apply(version: &str) -> UpdateApplyCommand {
    UpdateApplyCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
        version: version.to_owned(),
    }
}

fn cancel() -> UpdateCancelCommand {
    UpdateCancelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
    }
}

/// Runs the whole one-call prepare with an observer that throws its reports
/// away, which is every test that is not about progress.
fn prepare_quietly(
    controller: &mut UpdateController<'_>,
    command: &UpdatePrepareCommand,
) -> UpdateOutcomeProjection {
    controller.prepare(command, &mut |_| {})
}

/// Starts a split transfer, or fails the test with the refusal.
fn transfer(
    controller: &mut UpdateController<'_>,
    command: &UpdatePrepareCommand,
) -> longhorn_update::PreparedTransfer {
    match controller.begin_prepare(command) {
        UpdatePrepareStart::Transfer(transfer) => transfer,
        UpdatePrepareStart::Refused(outcome) => panic!("refused as {outcome:?}"),
    }
}

#[test]
fn a_manifest_for_the_wrong_channel_is_refused_and_not_stored() {
    // The endpoint for the selected channel answered with a manifest claiming
    // another. Evaluating it would let a mislabel silently restage a rollout.
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, _) = check(&signing);
    let mut mislabeled = manifest(&signing.signature(ARTIFACT));
    mislabeled.channel = Channel::Nightly;

    let outcome = controller.check(&command, &mislabeled, CheckKind::UserInitiated);

    assert_eq!(rejection(&outcome), UpdateRejectionCode::ChannelMismatch);
    let snapshot = controller.snapshot();
    assert!(matches!(
        snapshot.availability,
        longhorn_update::UpdateAvailabilityProjection::UpToDate
    ));
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
fn a_stale_authority_epoch_is_refused_on_every_command() {
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
    let prepared = controller.prepare(
        &UpdatePrepareCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: stale,
            version: "1.4.0".to_owned(),
        },
        &mut |_| {},
    );
    let applied = controller.apply(
        &UpdateApplyCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: stale,
            version: "1.4.0".to_owned(),
        },
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );
    let cancelled = controller.cancel(&UpdateCancelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: stale,
    });

    for outcome in [
        &checked, &selected, &deferred, &prepared, &applied, &cancelled,
    ] {
        assert_eq!(rejection(outcome), UpdateRejectionCode::StaleAuthority);
    }
    assert_eq!(fetch.calls(), 0, "a stale caller must not start a transfer");
}

#[test]
fn the_whole_sequence_prepares_then_applies_and_leaves_the_new_version_up_to_date() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let prepared = prepare_quietly(&mut controller, &prepare("1.4.0"));
    let snapshot = committed(&prepared);
    assert_eq!(
        snapshot.progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        }
    );
    assert_eq!(
        snapshot.installed_version, "1.3.0",
        "prepare replaces nothing"
    );
    let staged = snapshot.staged.as_ref().expect("a staged artifact");
    assert_eq!(staged.version, "1.4.0");
    assert_eq!(staged.channel, Channel::Production);

    let applied = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );

    let snapshot = committed(&applied);
    assert_eq!(snapshot.installed_version, "1.4.0");
    assert_eq!(snapshot.progress, UpdateProgressProjection::Idle);
    assert!(snapshot.staged.is_none());
    assert_eq!(fetch.calls(), 1, "apply must not re-download");
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
    let prepared = prepare_quietly(&mut controller, &prepare("1.4.0"));

    // The offer survives. It is not an error state, and a surface that showed
    // it as one would be hiding a version the user can install themselves.
    assert!(matches!(
        committed(&checked).availability,
        longhorn_update::UpdateAvailabilityProjection::ManagedElsewhere { .. }
    ));
    assert_eq!(rejection(&prepared), UpdateRejectionCode::NoOffer);
    assert_eq!(fetch.calls(), 0);
}

/// The gate runs at apply, so a busy host pays for the transfer and then keeps
/// what it downloaded. The retained artifact is the whole point of the staged
/// protocol: "Later" must not mean "download again".
#[test]
fn a_deferred_install_retains_the_staged_artifact_and_applies_later() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let busy = Busy;
    let deferred = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(vec![&busy], &GRANTING),
        &Installer,
    );

    // Committed, not rejected: a refused install carries its reason, and the
    // deferral is the reason.
    let snapshot = committed(&deferred);
    assert_eq!(snapshot.installed_version, "1.3.0");
    assert!(snapshot.deferral.is_some());
    assert_eq!(
        snapshot.progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        },
        "a refused install is deferred, not cancelled"
    );
    assert!(
        snapshot.staged.is_some(),
        "the verified bytes survive the deferral"
    );

    let applied = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );

    assert_eq!(committed(&applied).installed_version, "1.4.0");
    assert_eq!(fetch.calls(), 1, "Later must not re-download");
}

#[test]
fn a_cancel_discards_the_staged_artifact_and_returns_to_idle() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let cancelled = controller.cancel(&cancel());

    let snapshot = committed(&cancelled);
    assert_eq!(snapshot.progress, UpdateProgressProjection::Idle);
    assert!(snapshot.staged.is_none());

    // Nothing is staged, so a later apply has nothing to consume.
    let applied = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );
    assert_eq!(rejection(&applied), UpdateRejectionCode::NoOffer);
}

/// A cancel that arrives while the host is running the transfer cannot stop
/// it; it discards what arrives instead of retaining it.
#[test]
fn a_cancel_during_a_split_transfer_discards_what_arrives() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let transfer = transfer(&mut controller, &prepare("1.4.0"));
    controller.cancel(&cancel());
    let completed = controller.complete_prepare(&transfer, Ok(ARTIFACT.to_vec()));

    let snapshot = committed(&completed);
    assert_eq!(snapshot.progress, UpdateProgressProjection::Idle);
    assert!(snapshot.staged.is_none());
}

/// A byte report is observed while the transfer runs, not only after the call
/// returns. The flag is the proof: every observer call happened with the
/// transfer still on the stack.
#[test]
fn a_byte_report_is_observed_during_the_transfer() {
    let signing = Signing::new();
    let (source, _) = (Source, Fetch::serving(ARTIFACT));
    let transferring = Cell::new(false);
    let fetch = Streaming {
        bytes: ARTIFACT.to_vec(),
        reports: vec![
            FetchProgress::of(9, 27),
            FetchProgress::of(18, 27),
            FetchProgress::of(27, 27),
        ],
        calls: RefCell::new(0),
        transferring: &transferring,
    };
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let mut observed = Vec::new();
    let outcome = controller.prepare(&prepare("1.4.0"), &mut |progress| {
        observed.push((progress.clone(), transferring.get()));
    });

    assert_eq!(observed.len(), 3, "every report reaches the observer");
    assert!(
        observed.iter().all(|(_, during)| *during),
        "a report observed outside the transfer is not live progress"
    );
    assert_eq!(
        observed.first().unwrap().0,
        UpdateProgressProjection::Downloading {
            received: 9,
            expected: Some(27),
            fraction: Some(1.0 / 3.0),
        }
    );
    assert_eq!(
        observed.last().unwrap().0,
        UpdateProgressProjection::Downloading {
            received: 27,
            expected: Some(27),
            fraction: Some(1.0),
        }
    );
    // The call still lands on the retained state, not on the last report.
    assert_eq!(
        committed(&outcome).progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        }
    );
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

    let mut observed = Vec::new();
    let outcome = controller.prepare(&prepare("1.4.0"), &mut |progress| {
        observed.push(progress.clone());
    });

    // A host that reports nothing never reaches the observer at all, which is
    // the same answer as a source with no content length.
    assert!(
        observed.is_empty(),
        "a silent host reports nothing to observe"
    );
    assert_eq!(
        committed(&outcome).progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        }
    );
    assert_eq!(FetchProgress::unbounded(0).fraction(), None);
}

/// A failure is typed on the prepare outcome, and nothing is retained.
#[test]
fn fetch_failures_are_typed_and_stage_nothing() {
    let signing = Signing::new();
    let (source, _) = (Source, Fetch::serving(ARTIFACT));
    let cases = [
        (
            FetchError::Interrupted {
                detail: "reset".to_owned(),
            },
            UpdateRejectionCode::Unreachable,
        ),
        (
            FetchError::Unavailable {
                detail: "404".to_owned(),
            },
            UpdateRejectionCode::Unavailable,
        ),
    ];

    for (error, code) in cases {
        let fetch = Broken {
            error,
            calls: RefCell::new(0),
        };
        let mut controller = controller(&signing, &source, &fetch, writable());
        let (command, manifest) = check(&signing);
        controller.check(&command, &manifest, CheckKind::UserInitiated);

        let outcome = prepare_quietly(&mut controller, &prepare("1.4.0"));

        assert_eq!(rejection(&outcome), code);
        assert_eq!(
            controller.snapshot().progress,
            UpdateProgressProjection::Idle
        );
        assert!(controller.snapshot().staged.is_none());
    }
}

#[test]
fn an_artifact_signed_by_another_key_is_refused_and_never_staged() {
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

    let outcome = prepare_quietly(&mut controller, &prepare("1.4.0"));

    assert_eq!(rejection(&outcome), UpdateRejectionCode::SignatureRejected);
    assert_eq!(
        controller.snapshot().progress,
        UpdateProgressProjection::Idle
    );
    assert!(
        controller.snapshot().staged.is_none(),
        "a failed verification is discarded, never retained"
    );

    // And there is nothing for an installer to reach.
    let applied = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );
    assert_eq!(rejection(&applied), UpdateRejectionCode::NoOffer);
}

/// Replacement failure is not verification failure: the bytes are still
/// verified, so they stay staged for a retry.
#[test]
fn a_failed_replacement_keeps_the_staged_artifact() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let outcome = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Refusing,
    );

    assert_eq!(rejection(&outcome), UpdateRejectionCode::NotWritable);
    assert!(controller.snapshot().staged.is_some());
    assert_eq!(
        controller.snapshot().progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        }
    );
}

/// An apply that names a version this controller did not stage refuses, rather
/// than replacing the application with a release the surface never showed.
#[test]
fn an_apply_naming_a_different_version_refuses_and_keeps_the_staged_one() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let outcome = controller.apply(
        &apply("1.9.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );

    assert_eq!(rejection(&outcome), UpdateRejectionCode::NoOffer);
    assert!(controller.snapshot().staged.is_some());

    let applied = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &GRANTING),
        &Installer,
    );
    assert_eq!(committed(&applied).installed_version, "1.4.0");
}

/// The split prepare is what lets a host release its shared-state lock across
/// the transfer, so the two halves have to agree about what they are doing.
#[test]
fn a_split_prepare_retains_what_the_transfer_delivered() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let transfer = transfer(&mut controller, &prepare("1.4.0"));
    assert_eq!(transfer.version(), &version("1.4.0"));
    assert_eq!(transfer.authority_epoch(), 1);
    let delivered = fetch.fetch(
        transfer.request(),
        longhorn_update::MAX_ARTIFACT_BYTES,
        &mut |_| {},
    );
    let outcome = controller.complete_prepare(&transfer, delivered);

    let snapshot = committed(&outcome);
    let staged = snapshot.staged.as_ref().expect("a staged artifact");
    assert_eq!(staged.digest.len(), 64);
}

/// A channel switch during a split transfer replaces the authority lifetime,
/// so the delivered bytes belong to a context that is gone.
#[test]
fn a_channel_switch_during_a_transfer_discards_the_delivery() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let transfer = transfer(&mut controller, &prepare("1.4.0"));
    controller.select_channel(&UpdateSelectChannelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
        channel: Channel::Beta,
    });
    let outcome = controller.complete_prepare(&transfer, Ok(ARTIFACT.to_vec()));

    assert_eq!(rejection(&outcome), UpdateRejectionCode::StaleAuthority);
    assert!(controller.snapshot().staged.is_none());
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

/// The epoch is real now: a channel switch replaces the authority context,
/// so commands issued against the pre-switch snapshot refuse as stale.
#[test]
fn a_channel_switch_advances_the_epoch_and_refuses_pre_switch_commands() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);

    let switched = controller.select_channel(&UpdateSelectChannelCommand {
        protocol_version: UpdateProtocolVersion::CURRENT,
        authority_epoch: 1,
        channel: Channel::Beta,
    });
    assert_eq!(committed(&switched).authority_epoch, 2);

    // A command still carrying the pre-switch epoch is stale...
    let outdated = controller.check(&command, &manifest, CheckKind::Automatic);
    assert_eq!(rejection(&outdated), UpdateRejectionCode::StaleAuthority);

    // ...and one carrying the new epoch commits. The manifest is Production
    // while the build now follows Beta — the Card 201 refusal — so use a
    // manifest matching the selected channel.
    let mut beta = manifest;
    beta.channel = Channel::Beta;
    let current = controller.check(
        &UpdateCheckCommand {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: 2,
        },
        &beta,
        CheckKind::Automatic,
    );
    committed(&current);
}

/// The lease's lifetime is the critical section. A new conflicting acquisition
/// from inside the installer must be refused, and the barrier must lift only
/// after `apply` has returned — not before the swap, and not later.
#[test]
fn the_admission_lease_is_held_through_apply_and_released_after() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let authority = ExclusiveAuthority::new();
    let installer = LeaseWatchingInstaller::new(&authority);
    let gate = UpdateGate::new(Vec::new(), &authority);

    let applied = controller.apply(&apply("1.4.0"), &gate, &installer);

    assert_eq!(committed(&applied).installed_version, "1.4.0");
    assert_eq!(installer.calls.get(), 1, "replacement ran");
    assert!(
        installer.conflicting_work_refused.get(),
        "new conflicting work must be refused while the barrier is held"
    );
    assert!(
        !installer.conflicting_work_started.get(),
        "nothing conflicting may start during the swap"
    );
    assert_eq!(authority.acquisitions.get(), 1);
    assert!(
        !authority.held.get(),
        "the barrier lifts once apply has returned"
    );
    assert_eq!(authority.releases.get(), 1);
}

/// No bypass: a lease the host will not grant defers, and the installer is
/// never reached. The staged artifact survives for a later attempt.
#[test]
fn an_unacquirable_lease_defers_and_nothing_installs() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let installer = CountingInstaller::new();
    let deferred = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &NoAdmission),
        &installer,
    );

    let snapshot = committed(&deferred);
    assert_eq!(
        installer.calls.get(),
        0,
        "an unacquired lease must install nothing"
    );
    assert_eq!(snapshot.installed_version, "1.3.0");
    assert_eq!(
        snapshot.deferral.as_ref().map(|deferral| &deferral.cause),
        Some(&DeferralCause::WorkInFlight {
            detail: "an index rebuild is running".to_owned(),
        }),
        "the host's reason travels with the refusal"
    );
    assert_eq!(
        snapshot.progress,
        UpdateProgressProjection::ReadyToInstall {
            version: "1.4.0".to_owned(),
        },
        "a refused lease is a deferral, not a cancellation"
    );
    assert!(snapshot.staged.is_some(), "the bytes survive for a retry");
}

/// The barrier is tied to the call rather than to success: a replacement that
/// fails still releases it, or the host would be wedged behind a lease nothing
/// holds.
#[test]
fn a_failed_replacement_still_releases_the_lease() {
    let signing = Signing::new();
    let (source, fetch) = (Source, Fetch::serving(ARTIFACT));
    let mut controller = controller(&signing, &source, &fetch, writable());
    let (command, manifest) = check(&signing);
    controller.check(&command, &manifest, CheckKind::UserInitiated);
    prepare_quietly(&mut controller, &prepare("1.4.0"));

    let authority = ExclusiveAuthority::new();
    let outcome = controller.apply(
        &apply("1.4.0"),
        &UpdateGate::new(Vec::new(), &authority),
        &Refusing,
    );

    assert_eq!(rejection(&outcome), UpdateRejectionCode::NotWritable);
    assert_eq!(authority.acquisitions.get(), 1);
    assert_eq!(authority.releases.get(), 1);
    assert!(!authority.held.get());
}
