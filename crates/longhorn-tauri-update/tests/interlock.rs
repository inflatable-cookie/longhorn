//! Restart interlock evidence.
//!
//! The claim under test: no install is authorized while Longhorn-owned work
//! is in flight, the reason travels with the refusal, and authorization is
//! rechecked rather than reused from an earlier offer.

use std::sync::atomic::{AtomicUsize, Ordering};

use longhorn_tauri_update::{
    CountingProbe, InstallAuthorization, UpdateGate, operation_probe, transfer_session_probe,
};
use longhorn_update::{DeferralCause, QuiescenceKind, QuiescenceProbe};
use semver::Version;

fn version() -> Version {
    Version::parse("1.3.0").unwrap()
}

#[test]
fn a_quiescent_host_authorizes_the_install() {
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(probes);

    assert_eq!(gate.authorize(&version()), InstallAuthorization::Approved);
}

#[test]
fn an_open_transfer_session_refuses_the_install_entirely() {
    // The destructive case. Not "install and hope", not "install and warn"
    // -- the application must never install while the session is open.
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(probes);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!("an open session must defer, found {}", gate.quiescence().detail());
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
    let gate = UpdateGate::new(probes);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
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
fn quiescence_is_rechecked_at_authorization_time() {
    // The user may start a transfer between being offered the update and
    // accepting it. A receipt taken when the offer was made is not an answer
    // to "is it safe now".
    let sessions = AtomicUsize::new(0);
    let probe = transfer_session_probe(|| sessions.load(Ordering::Relaxed));
    let probes: Vec<&dyn QuiescenceProbe> = vec![&probe];
    let gate = UpdateGate::new(probes);

    assert!(gate.quiescence().is_quiescent());
    assert_eq!(gate.authorize(&version()), InstallAuthorization::Approved);

    sessions.store(1, Ordering::Relaxed);
    assert!(matches!(
        gate.authorize(&version()),
        InstallAuthorization::Deferred(_)
    ));
}

#[test]
fn a_refused_install_never_looks_like_a_cancelled_one() {
    // The deferral carries the version it was taken against, so the surface
    // can distinguish "refused for a reason" from "nothing was refused".
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(probes);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!("an open session must defer");
    };

    assert_eq!(deferral.version, version());
    assert!(
        matches!(deferral.cause, DeferralCause::WorkInFlight { .. }),
        "the refusal must carry the reason"
    );
}
