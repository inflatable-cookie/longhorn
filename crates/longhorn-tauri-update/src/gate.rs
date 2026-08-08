use longhorn_update::{Deferral, QuiescenceProbe, QuiescenceReceipt};
use semver::Version;

/// Gates installation on Longhorn-owned work being settled.
///
/// Longhorn does not install. The Tauri updater plugin performs check,
/// download, verification, and bundle replacement; this gate answers the one
/// question Longhorn is entitled to answer — is it safe to install right
/// now. The application calls [`UpdateGate::authorize`] immediately before
/// handing the downloaded artifact to the plugin.
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
