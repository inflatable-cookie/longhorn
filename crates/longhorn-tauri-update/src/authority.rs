use longhorn_update::{
    FetchError, PreparedTransfer, UpdateApplyCommand, UpdateCancelCommand, UpdateCheckCommand,
    UpdateDeferCommand, UpdateOutcomeProjection, UpdatePrepareCommand, UpdatePrepareStart,
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
///
/// # Why prepare is two calls
///
/// A staged prepare transfers the artifact, and a host that serializes its
/// authority behind one lock would hold that lock for the whole transfer if it
/// were one call. `begin_prepare` and `complete_prepare` are the seam that lets
/// the transfer run outside the lock: the only state touched while bytes arrive
/// is the progress report, which travels out of band.
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

    /// Validates the offer and composes the transfer.
    ///
    /// Returns the request the host transfers, or an ordinary refusal. Nothing
    /// is retained until [`Self::complete_prepare`].
    fn begin_prepare(
        &mut self,
        caller: &str,
        command: UpdatePrepareCommand,
    ) -> Result<UpdatePrepareStart, UpdateHostError>;

    /// Verifies and retains what the transfer delivered.
    ///
    /// Takes the transfer's outcome rather than its bytes so the controller
    /// maps a transport failure onto its own typed rejection. A verification
    /// failure discards; it never stages.
    fn complete_prepare(
        &mut self,
        caller: &str,
        transfer: PreparedTransfer,
        delivered: Result<Vec<u8>, FetchError>,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;

    /// Applies the retained staged artifact.
    ///
    /// Its own capability, separate from `check` and from `begin_prepare`.
    /// Staging verified bytes is not covered by permission to replace the
    /// running application, and vice versa.
    fn apply(
        &mut self,
        caller: &str,
        command: UpdateApplyCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;

    /// Discards the retained staged artifact.
    fn cancel(
        &mut self,
        caller: &str,
        command: UpdateCancelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError>;
}
