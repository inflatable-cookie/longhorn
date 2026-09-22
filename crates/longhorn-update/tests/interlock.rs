//! Restart interlock evidence.
//!
//! The claim under test: no install is authorized while Longhorn-owned work
//! is in flight, the reason travels with the refusal, and authorization is
//! rechecked rather than reused from an earlier offer. Since the 2026-09-22
//! amendment the approval is a *held* exclusive admission lease, so the
//! further claim is that a held lease refuses new work until it is dropped.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use longhorn_update::{
    AdmissionAuthority, AdmissionLease, AdmissionRefusal, CountingProbe, DeferralCause,
    InstallAuthorization, QuiescenceKind, QuiescenceProbe, UpdateGate,
    UpdateInstallAuthorizationProjection, operation_probe, transfer_session_probe,
};
use semver::Version;

fn version() -> Version {
    Version::parse("1.3.0").unwrap()
}

/// Grants one exclusive lease at a time; a second acquire while one is held is
/// refused with the host's reason. This is the minimal admission authority the
/// contract describes, and it is the gate's fake: the barrier lives here, not
/// in Longhorn.
struct ExclusiveAuthority {
    held: Cell<bool>,
    acquisitions: Cell<u32>,
    refusals: Cell<u32>,
}

impl ExclusiveAuthority {
    fn new() -> Self {
        Self {
            held: Cell::new(false),
            acquisitions: Cell::new(0),
            refusals: Cell::new(0),
        }
    }

    fn held(&self) -> bool {
        self.held.get()
    }

    fn acquisitions(&self) -> u32 {
        self.acquisitions.get()
    }

    fn refusals(&self) -> u32 {
        self.refusals.get()
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

/// The lease the exclusive authority hands out. Dropping it is the release.
struct ExclusiveLease<'authority> {
    authority: &'authority ExclusiveAuthority,
}

impl AdmissionLease for ExclusiveLease<'_> {}

impl Drop for ExclusiveLease<'_> {
    fn drop(&mut self) {
        self.authority.held.set(false);
    }
}

/// Refuses every acquisition, so the refusal path is reachable.
struct RefusingAuthority;

impl AdmissionAuthority for RefusingAuthority {
    fn acquire(&self) -> Result<Box<dyn AdmissionLease + '_>, AdmissionRefusal> {
        Err(AdmissionRefusal::new("another window is saving"))
    }
}

#[test]
fn a_quiescent_host_holds_an_exclusive_lease() {
    let authority = ExclusiveAuthority::new();
    let idle = transfer_session_probe(|| 0);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&idle];
    let gate = UpdateGate::new(probes, &authority);

    let authorization = gate.authorize(&version());

    assert!(matches!(&authorization, InstallAuthorization::Held(_)));
    assert!(authority.held(), "the barrier is taken, not just promised");
    assert_eq!(authority.acquisitions(), 1);
}

#[test]
fn a_held_lease_refuses_new_work_until_it_is_dropped() {
    // "Conflicting work" is the host's concept, so the fake models it as a
    // second acquisition attempt. While the first lease is alive the authority
    // refuses, and the gate turns that into a deferral carrying the reason.
    let authority = ExclusiveAuthority::new();
    let gate = UpdateGate::new(Vec::new(), &authority);
    let held = gate.authorize(&version());
    assert!(matches!(&held, InstallAuthorization::Held(_)));

    let refused = gate.authorize(&version());

    let InstallAuthorization::Deferred(deferral) = &refused else {
        panic!("a held lease must refuse new work");
    };
    assert_eq!(
        deferral.cause,
        DeferralCause::WorkInFlight {
            detail: "a write is in flight".to_owned(),
        }
    );
    assert_eq!(authority.refusals(), 1);

    drop(held);

    assert!(!authority.held(), "dropping the lease is the release");
    let again = gate.authorize(&version());
    assert!(matches!(&again, InstallAuthorization::Held(_)));
    assert_eq!(authority.acquisitions(), 2);
}

#[test]
fn an_open_transfer_session_refuses_the_install_entirely() {
    // The destructive case. Not "install and hope", not "install and warn"
    // -- the application must never install while the session is open.
    let authority = ExclusiveAuthority::new();
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(probes, &authority);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!(
            "an open session must defer, found {}",
            gate.quiescence().detail()
        );
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
    let authority = ExclusiveAuthority::new();
    let sessions = transfer_session_probe(|| 2);
    let operations = operation_probe(|| 3);
    let flushes = CountingProbe::new(QuiescenceKind::PendingFlush, || 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&flushes, &sessions, &operations];
    let gate = UpdateGate::new(probes, &authority);

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
    let authority = ExclusiveAuthority::new();
    let sessions = AtomicUsize::new(0);
    let probe = transfer_session_probe(|| sessions.load(Ordering::Relaxed));
    let probes: Vec<&dyn QuiescenceProbe> = vec![&probe];
    let gate = UpdateGate::new(probes, &authority);

    assert!(gate.quiescence().is_quiescent());
    let held = gate.authorize(&version());
    assert!(matches!(&held, InstallAuthorization::Held(_)));
    drop(held);

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
    let authority = ExclusiveAuthority::new();
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(probes, &authority);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!("an open session must defer");
    };

    assert_eq!(deferral.version, version());
    assert!(
        matches!(deferral.cause, DeferralCause::WorkInFlight { .. }),
        "the refusal must carry the reason"
    );
}

/// Quiescence is the precondition and the lease is the barrier, so a receipt
/// that is not clear must stop before the authority is asked at all: taking a
/// lease and holding it while Longhorn's own work is mid-commit is worse than
/// an ordinary deferral.
#[test]
fn quiescence_is_checked_before_the_lease_is_acquired() {
    let authority = ExclusiveAuthority::new();
    let busy = transfer_session_probe(|| 1);
    let probes: Vec<&dyn QuiescenceProbe> = vec![&busy];
    let gate = UpdateGate::new(probes, &authority);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!("outstanding work must defer");
    };

    assert!(matches!(deferral.cause, DeferralCause::WorkInFlight { .. }));
    assert_eq!(
        authority.acquisitions(),
        0,
        "a non-quiescent host must not take the barrier"
    );
}

/// The wire projection states which of the two answers the gate gave. The
/// lease itself cannot cross the boundary, so this is how a generated client
/// reads a held barrier.
#[test]
fn the_authorization_projects_as_held_or_deferred() {
    let authority = ExclusiveAuthority::new();
    let gate = UpdateGate::new(Vec::new(), &authority);

    let held = gate.authorize(&version());
    assert_eq!(
        held.projection(),
        UpdateInstallAuthorizationProjection::Held
    );
    drop(held);

    let refusing = RefusingAuthority;
    let refusing_gate = UpdateGate::new(Vec::new(), &refusing);
    let refused = refusing_gate.authorize(&version());
    assert_eq!(
        refused.projection(),
        UpdateInstallAuthorizationProjection::Deferred {
            cause: DeferralCause::WorkInFlight {
                detail: "another window is saving".to_owned(),
            },
        }
    );
}

/// A lease the host will not grant is a deferral carrying the host's reason,
/// never a failure and never an install. The wire cause is the same
/// `WorkInFlight` a busy Longhorn probe produces — the answer is the same — and
/// the host's detail is what distinguishes it, without Longhorn learning what
/// the conflicting work is.
#[test]
fn an_unacquirable_lease_defers_with_the_hosts_reason() {
    let authority = RefusingAuthority;
    let gate = UpdateGate::new(Vec::new(), &authority);

    let InstallAuthorization::Deferred(deferral) = gate.authorize(&version()) else {
        panic!("a refused lease must defer");
    };

    assert_eq!(
        deferral.cause,
        DeferralCause::WorkInFlight {
            detail: "another window is saving".to_owned(),
        }
    );
    assert_eq!(
        deferral.cause.to_string(),
        "work in flight: another window is saving"
    );
}
