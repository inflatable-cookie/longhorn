use longhorn_update::{
    UpdateCheckCommand, UpdateDeferCommand, UpdateInstallCommand, UpdateOutcomeProjection,
    UpdateSelectChannelCommand, UpdateSnapshot,
};

use crate::UpdateHostError;

/// Consumer-injected caller authorization over the update controller.
///
/// The consumer holds the `UpdateController` and the ports it needs — a
/// source, a fetch, quiescence probes, an installer. This trait is the caller
/// check in front of it, not a second controller.
///
/// `check` takes no manifest: retrieving one is the consumer's, because the
/// consumer is where the transport is. `UpdateController::manifest_request`
/// composes the request it should use.
pub trait UpdateHostAuthority: Send {
    /// Returns the caller-authorized update state.
    fn snapshot(&mut self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError>;

    /// Asks the source for the channel's current manifest and records it.
    ///
    /// Its own capability. A window that may display update state has not
    /// thereby been given permission to reach the network.
    fn check(
        &mut self,
        caller: &str,
        command: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;

    /// Follows a different channel from now on.
    fn select_channel(
        &mut self,
        caller: &str,
        command: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;

    /// Declines a version for now.
    fn defer(
        &mut self,
        caller: &str,
        command: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;

    /// Fetches, verifies, gates and installs.
    ///
    /// Its own capability, separate from `check`. Authorizing an install is
    /// not covered by permission to look for one: the first reads, the second
    /// replaces the running application.
    fn install(
        &mut self,
        caller: &str,
        command: UpdateInstallCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
}
