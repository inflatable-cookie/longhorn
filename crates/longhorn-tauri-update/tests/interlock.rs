//! Restart interlock evidence.
//!
//! The claim under test: no update installs while Longhorn-owned work is in
//! flight, and an update that reaches disk is never reported as a failure.

use std::sync::atomic::{AtomicUsize, Ordering};

use longhorn_tauri_update::{
    CountingProbe, InstallError, InstallOutcome, UpdateGate, UpdateInstaller, operation_probe,
    transfer_session_probe,
};
use longhorn_update::{DeferralCause, QuiescenceKind, QuiescenceProbe};
use semver::Version;

fn version() -> Version {
    Version::parse("1.3.0").unwrap()
}

/// Records what the gate asked of the installer.
#[derive(Default)]
struct RecordingInstaller {
    installs: AtomicUsize,
    relaunches: AtomicUsize,
    install_result: Option<InstallError>,
    relaunch_result: Option<InstallError>,
}

impl RecordingInstaller {
    fn failing_install(error: InstallError) -> Self {
        Self {
            install_result: Some(error),
            ..Self::default()
        }
    }

    fn failing_relaunch(error: InstallError) -> Self {
        Self {
            relaunch_result: Some(error),
            ..Self::default()
        }
    }
}

impl UpdateInstaller for RecordingInstaller {
    fn install(&self) -> Result<(), InstallError> {
        self.installs.fetch_add(1, Ordering::Relaxed);
        self.install_result.clone().map_or(Ok(()), Err)
    }

    fn relaunch(&self) -> Result<(), InstallError> {
        self.relaunches.fetch_add(1, Ordering::Relaxed);
        self.relaunch_result.clone().map_or(Ok(()), Err)
    }
}

#[test]
fn a_quiescent_host_installs_and_relaunches() {
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(RecordingInstaller::default(), probes);

    assert_eq!(gate.install(&version()), InstallOutcome::Installed);
}

#[test]
fn an_open_transfer_session_refuses_the_install_entirely() {
    // The destructive case. Not "install and hope", not "install and warn"
    // -- the installer must never be reached.
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let installer = RecordingInstaller::default();
    let gate = UpdateGate::new(installer, probes);

    let outcome = gate.install(&version());

    let InstallOutcome::Deferred(deferral) = outcome else {
        panic!("an open session must defer, found {outcome:?}");
    };
    assert_eq!(
        deferral.cause,
        DeferralCause::WorkInFlight {
            detail: "1 open transfer session".to_owned()
        }
    );
    assert_eq!(deferral.version, version());
}

#[test]
fn the_deferral_reason_names_everything_outstanding() {
    // A surface that said "1 open transfer session" while three operations
    // were also running would understate what the user is interrupting.
    let sessions = transfer_session_probe(|| 2);
    let operations = operation_probe(|| 3);
    let flushes = CountingProbe::new(QuiescenceKind::PendingFlush, || 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&flushes, &sessions, &operations];
    let gate = UpdateGate::new(RecordingInstaller::default(), probes);

    let InstallOutcome::Deferred(deferral) = gate.install(&version()) else {
        panic!("outstanding work must defer");
    };

    assert_eq!(
        deferral.cause,
        DeferralCause::WorkInFlight {
            detail: "1 pending flush, 2 open transfer sessions, 3 running operations".to_owned()
        }
    );
}

#[test]
fn quiescence_is_rechecked_at_install_time() {
    // The user may start a transfer between being offered the update and
    // accepting it. A receipt taken when the offer was made is not an answer
    // to "is it safe now".
    let sessions = AtomicUsize::new(0);
    let probe = transfer_session_probe(|| sessions.load(Ordering::Relaxed));
    let probes: Vec<&dyn QuiescenceProbe> = vec![&probe];
    let gate = UpdateGate::new(RecordingInstaller::default(), probes);

    assert!(gate.quiescence().is_quiescent());

    sessions.store(1, Ordering::Relaxed);
    assert!(matches!(
        gate.install(&version()),
        InstallOutcome::Deferred(_)
    ));
}

#[test]
fn a_non_writable_installation_defers_with_its_own_cause() {
    // Homebrew casks and administrator-installed copies. The remedy is a
    // manual download, and the cause has to say so rather than looking like
    // transient work in flight.
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(
        RecordingInstaller::failing_install(InstallError::NotWritable {
            detail: "/Applications/Example.app".into(),
        }),
        probes,
    );

    let InstallOutcome::Deferred(deferral) = gate.install(&version()) else {
        panic!("a non-writable installation must defer");
    };

    assert_eq!(
        deferral.cause,
        DeferralCause::InstallationNotWritable {
            detail: "/Applications/Example.app".to_owned()
        }
    );
    assert!(
        !deferral.cause.is_retryable(),
        "a non-writable installation cannot resolve itself"
    );
}

#[test]
fn a_failed_relaunch_is_not_reported_as_a_failed_update() {
    // tauri#11392: the update lands and the application does not come back.
    // Telling the user the update failed would be false, and would invite
    // them to retry an update they already have.
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(
        RecordingInstaller::failing_relaunch(InstallError::RelaunchFailed {
            detail: "process did not start".into(),
        }),
        probes,
    );

    let outcome = gate.install(&version());

    assert!(matches!(
        outcome,
        InstallOutcome::InstalledAwaitingRelaunch { .. }
    ));
    assert!(
        !matches!(outcome, InstallOutcome::Deferred(_)),
        "the update reached disk; it must not be reported as deferred"
    );
}

#[test]
fn an_install_error_reports_that_the_update_did_not_land() {
    assert!(
        !InstallError::Failed {
            detail: "network".into()
        }
        .update_landed()
    );
    assert!(
        InstallError::RelaunchFailed {
            detail: "process did not start".into()
        }
        .update_landed()
    );
}

#[test]
fn a_failed_install_defers_as_a_failed_install_not_as_work_in_flight() {
    // Nothing Longhorn-owned was running when the install failed; reporting
    // "work in flight" would tell the user the wrong story about why the
    // update did not happen.
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(
        RecordingInstaller::failing_install(InstallError::Failed {
            detail: "network unreachable".into(),
        }),
        probes,
    );

    let InstallOutcome::Deferred(deferral) = gate.install(&version()) else {
        panic!("a failed install must defer");
    };

    assert_eq!(
        deferral.cause,
        DeferralCause::InstallFailed {
            detail: "update install failed: network unreachable".to_owned()
        }
    );
    assert!(
        deferral.cause.is_retryable(),
        "a failed install can succeed on retry"
    );
}
