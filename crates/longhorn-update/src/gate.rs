use semver::Version;

use crate::{
    AdmissionAuthority, AdmissionLease, Deferral, DeferralCause, QuiescenceProbe,
    QuiescenceReceipt, UpdateInstallAuthorizationProjection,
};

/// Gates installation on Longhorn-owned work being settled and on a held
/// exclusive admission lease.
///
/// This gate answers one question and no others — is it safe to install right
/// now. Installation is `longhorn-update-install`'s, on every host. The
/// application calls [`UpdateGate::authorize`] immediately before handing the
/// downloaded artifact to the installer.
///
/// The separation survives the 2026-08-09 amendment that moved execution into
/// Longhorn: authorization was always host-agnostic, and knowing what is in
/// flight is a different question from knowing how to replace a bundle. That
/// is why this lives in the pure policy crate — it decides, it does not act.
///
/// # Two answers, one critical section
///
/// The 2026-09-22 amendment makes authorization a *held* answer. Quiescence is
/// still checked on every call and is the precondition; the exclusive
/// admission lease is the barrier that keeps the answer true until the
/// replacement returns. The receipt says "nothing is in flight now"; the lease
/// says "nothing conflicting starts until I am dropped". Both are needed, and
/// the lease is released by dropping the [`InstallAuthorization`] that
/// carries it.
///
/// Reporting note: an install that reached disk but did not relaunch is not
/// a failed update. Tell the user to reopen the application; telling them
/// the update failed invites retrying an update they already have.
pub struct UpdateGate<'probe> {
    probes: Vec<&'probe dyn QuiescenceProbe>,
    admission: &'probe dyn AdmissionAuthority,
}

impl<'probe> UpdateGate<'probe> {
    /// Records a gate over a set of probes and an admission authority.
    ///
    /// The authority is host-supplied and required: a gate with no way to take
    /// the barrier would be the silent bypass contract 018 forbids. The
    /// application wires both once and keeps them.
    pub fn new(
        probes: Vec<&'probe dyn QuiescenceProbe>,
        admission: &'probe dyn AdmissionAuthority,
    ) -> Self {
        Self { probes, admission }
    }

    /// Returns what is currently outstanding.
    ///
    /// Every probe runs; the receipt is the union, so a surface reporting
    /// the reason states all of it rather than whichever probe happened to
    /// be first.
    #[must_use]
    pub fn quiescence(&self) -> QuiescenceReceipt {
        QuiescenceReceipt::collect(self.probes.iter().copied())
    }

    /// Authorizes installing `version` by taking the exclusive admission
    /// lease, or defers with the reason.
    ///
    /// Quiescence is checked on every call, never cached from an earlier
    /// one: the user may have started a transfer between being offered the
    /// update and accepting it. A refused install is never a cancelled one —
    /// the reason travels with the refusal.
    ///
    /// Quiescence is checked first, because it is the precondition; the lease
    /// is acquired second and must be held until the replacement returns. The
    /// returned [`InstallAuthorization::Held`] owns the lease, so a caller
    /// that drops it before `apply` returns has reopened the window. A lease
    /// that cannot be acquired is a deferral carrying the host's reason, never
    /// a failure and never an install.
    pub fn authorize(&self, version: &Version) -> InstallAuthorization<'probe> {
        let receipt = self.quiescence();
        if let Some(cause) = receipt.as_deferral_cause() {
            return InstallAuthorization::Deferred(Deferral::new(version.clone(), cause));
        }
        match self.admission.acquire() {
            Ok(lease) => InstallAuthorization::Held(lease),
            // The host refused to hold the barrier. It is the same deferral
            // cause as Longhorn's own probes because it is the same answer —
            // something conflicting is in flight — and the host's detail is
            // what says which. Longhorn relays it and learns nothing about the
            // application's operations.
            Err(refusal) => InstallAuthorization::Deferred(Deferral::new(
                version.clone(),
                DeferralCause::WorkInFlight {
                    detail: refusal.detail,
                },
            )),
        }
    }
}

/// What an install attempt is permitted to do.
///
/// The `Held` variant owns the exclusive admission lease. Keeping the value
/// alive for the whole critical section is the caller's obligation, and it is
/// why this type is not `Clone`: there is exactly one lease, and duplicating
/// it would be duplicating the barrier.
pub enum InstallAuthorization<'lease> {
    /// The exclusive admission lease is held; nothing conflicting can start
    /// until this value is dropped.
    Held(Box<dyn AdmissionLease + 'lease>),
    /// Something is in flight, or the host refused the lease; do not install,
    /// and why.
    Deferred(Deferral),
}

impl InstallAuthorization<'_> {
    /// Projects the authorization for the wire.
    ///
    /// The lease itself never crosses a boundary — it is a local capability —
    /// so the projection reports that one is held rather than trying to
    /// serialise it.
    #[must_use]
    pub fn projection(&self) -> UpdateInstallAuthorizationProjection {
        match self {
            Self::Held(_) => UpdateInstallAuthorizationProjection::Held,
            Self::Deferred(deferral) => UpdateInstallAuthorizationProjection::Deferred {
                cause: deferral.cause.clone(),
            },
        }
    }
}

impl core::fmt::Debug for InstallAuthorization<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Held(_) => formatter.write_str("Held(<admission lease>)"),
            Self::Deferred(deferral) => formatter.debug_tuple("Deferred").field(deferral).finish(),
        }
    }
}
