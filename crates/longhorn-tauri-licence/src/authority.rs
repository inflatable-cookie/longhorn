use longhorn_licence::{
    LicenceActivateCommand, LicenceDeactivateCommand, LicenceOutcomeProjection,
    LicenceRefreshCommand, LicenceReleaseSeatCommand, LicenceRenameSeatCommand, LicenceSnapshot,
};

use crate::LicenceHostError;

/// Consumer-injected caller authorization over the licence authority.
///
/// The consumer holds the composition — the verified licence, the credential
/// store, the activation sources, the clock guard. This trait is the caller
/// check in front of it, not a second authority.
pub trait LicenceHostAuthority: Send {
    /// Returns the caller-authorized licence state.
    fn snapshot(&mut self, caller: &str) -> Result<LicenceSnapshot, LicenceHostError>;

    /// Presents a credential and asks for a licence.
    ///
    /// Its own capability. This is the one command that carries credential
    /// material inward and the one that writes the platform keychain; a
    /// window that may display licence state has not thereby been trusted
    /// with either.
    fn activate(
        &mut self,
        caller: &str,
        command: LicenceActivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;

    /// Releases this machine's seat.
    fn deactivate(
        &mut self,
        caller: &str,
        command: LicenceDeactivateCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;

    /// Re-checks the lease now.
    ///
    /// Its own capability: it reaches the network on the operator's behalf,
    /// which displaying state does not.
    fn refresh(
        &mut self,
        caller: &str,
        command: LicenceRefreshCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;

    /// Releases a named machine's seat.
    fn release_seat(
        &mut self,
        caller: &str,
        command: LicenceReleaseSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;

    /// Renames a machine's seat.
    fn rename_seat(
        &mut self,
        caller: &str,
        command: LicenceRenameSeatCommand,
    ) -> Result<LicenceOutcomeProjection, LicenceHostError>;
}
