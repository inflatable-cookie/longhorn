use longhorn_update::{Deferral, DeferralCause, QuiescenceProbe, QuiescenceReceipt};
use semver::Version;

use crate::{InstallError, UpdateInstaller};

/// What happened when an install was attempted.
#[derive(Clone, Debug, PartialEq)]
pub enum InstallOutcome {
    /// Installed and relaunching.
    Installed,
    /// Installed, but the application did not relaunch itself.
    ///
    /// The update landed. Telling the user to reopen the application is
    /// correct; telling them the update failed is not.
    InstalledAwaitingRelaunch {
        /// Why the relaunch did not happen.
        detail: String,
    },
    /// Not installed, and why.
    Deferred(Deferral),
}

/// Gates installation on Longhorn-owned work being settled.
///
/// The order is quiesce, install, relaunch. macOS separates install from
/// relaunch — the plugin replaces the bundle and returns without relaunching
/// — which is what makes that ordering ours to choose rather than something
/// we have to work around.
pub struct UpdateGate<'probe, I> {
    probes: Vec<&'probe dyn QuiescenceProbe>,
    installer: I,
}

impl<'probe, I> UpdateGate<'probe, I>
where
    I: UpdateInstaller,
{
    /// Records a gate over a set of probes.
    pub fn new(installer: I, probes: Vec<&'probe dyn QuiescenceProbe>) -> Self {
        Self { probes, installer }
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

    /// Installs `version` if nothing is in flight.
    ///
    /// Quiescence is checked immediately before handing over, not cached
    /// from an earlier call: the user may have started a transfer between
    /// being offered the update and accepting it.
    pub fn install(&self, version: &Version) -> InstallOutcome {
        let receipt = self.quiescence();
        if let Some(cause) = receipt.as_deferral_cause() {
            return InstallOutcome::Deferred(Deferral::new(version.clone(), cause));
        }

        match self.installer.install() {
            Ok(()) => {}
            Err(InstallError::NotWritable { detail }) => {
                return InstallOutcome::Deferred(Deferral::new(
                    version.clone(),
                    DeferralCause::InstallationNotWritable { detail },
                ));
            }
            Err(error) => {
                return InstallOutcome::Deferred(Deferral::new(
                    version.clone(),
                    DeferralCause::WorkInFlight {
                        detail: error.to_string(),
                    },
                ));
            }
        }

        // From here the update is on disk. Nothing below may report a
        // failure that reads as "the update did not happen", because it did.
        match self.installer.relaunch() {
            Ok(()) => InstallOutcome::Installed,
            Err(error) => InstallOutcome::InstalledAwaitingRelaunch {
                detail: error.to_string(),
            },
        }
    }
}
