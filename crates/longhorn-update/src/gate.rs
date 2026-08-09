use crate::{Deferral, QuiescenceProbe, QuiescenceReceipt};
use semver::Version;

/// Gates installation on Longhorn-owned work being settled.
///
/// This gate answers one question and no others — is it safe to install
/// right now. Installation is `longhorn-update-install`'s, on every host. The
/// application calls [`UpdateGate::authorize`] immediately before handing the
/// downloaded artifact to the installer.
///
/// The separation survives the 2026-08-09 amendment that moved execution into
/// Longhorn: authorization was always host-agnostic, and knowing what is in
/// flight is a different question from knowing how to replace a bundle. That
/// is why this lives in the pure policy crate — it decides, it does not act.
///
/// Reporting note: an install that reached disk but did not relaunch is not
/// a failed update. Tell the user to reopen the application; telling them
/// the update failed invites retrying an update they already have.
pub struct UpdateGate<'probe> {
    probes: Vec<&'probe dyn QuiescenceProbe>,
}

impl<'probe> UpdateGate<'probe> {
    /// Records a gate over a set of probes.
    pub fn new(probes: Vec<&'probe dyn QuiescenceProbe>) -> Self {
        Self { probes }
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

    /// Authorizes installing `version`, or defers with the reason.
    ///
    /// Quiescence is checked on every call, never cached from an earlier
    /// one: the user may have started a transfer between being offered the
    /// update and accepting it. A refused install is never a cancelled one —
    /// the reason travels with the refusal.
    pub fn authorize(&self, version: &Version) -> InstallAuthorization {
        let receipt = self.quiescence();
        if let Some(cause) = receipt.as_deferral_cause() {
            return InstallAuthorization::Deferred(Deferral::new(version.clone(), cause));
        }
        InstallAuthorization::Approved
    }
}

/// What an install attempt is permitted to do.
#[derive(Clone, Debug, PartialEq)]
pub enum InstallAuthorization {
    /// Nothing is in flight; the application may install.
    Approved,
    /// Something is in flight; do not install, and why.
    Deferred(Deferral),
}
