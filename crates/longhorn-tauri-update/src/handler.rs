use std::sync::{Arc, Mutex};

use longhorn_update::{
    ArtifactFetch, MAX_ARTIFACT_BYTES, UpdateApplyCommand, UpdateCancelCommand, UpdateCheckCommand,
    UpdateDeferCommand, UpdateOutcomeProjection, UpdatePrepareCommand, UpdatePrepareStart,
    UpdateProgressEvent, UpdateProtocolVersion, UpdateSelectChannelCommand, UpdateSnapshot,
};

use crate::{UpdateHostAuthority, UpdateHostError, UpdateHostService};

/// Shared injected assembly used by Tauri and conformance tests.
///
/// Holds the transfer port as well as the authority so the authoritative state
/// is not locked while a transfer runs: `prepare` takes the lock to compose the
/// request, releases it, transfers, and takes it again to retain the result.
///
/// The port is an `Arc` rather than a borrow because Tauri managed state has to
/// be `'static`. A consumer builds one fetch, hands a clone here, and lends the
/// same instance to the `UpdateController` it injects as authority.
pub struct UpdateHandlerAssembly<A> {
    authority: Mutex<A>,
    fetch: Arc<dyn ArtifactFetch + Send + Sync>,
}

impl<A> UpdateHandlerAssembly<A> {
    /// Binds one explicitly injected consumer authority and the transfer port
    /// the assembly drives on its behalf.
    #[must_use]
    pub fn new(authority: A, fetch: Arc<dyn ArtifactFetch + Send + Sync>) -> Self {
        Self {
            authority: Mutex::new(authority),
            fetch,
        }
    }

    /// Runs trusted host work against injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, UpdateHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| UpdateHostError::state_unavailable())
    }
}

impl<A> UpdateHostService for UpdateHandlerAssembly<A>
where
    A: UpdateHostAuthority,
{
    fn snapshot(&self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    fn check(
        &self,
        caller: &str,
        command: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.check(caller, command))?
    }

    fn select_channel(
        &self,
        caller: &str,
        command: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.select_channel(caller, command))?
    }

    fn defer(
        &self,
        caller: &str,
        command: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.defer(caller, command))?
    }

    fn prepare(
        &self,
        caller: &str,
        command: UpdatePrepareCommand,
        progress: &mut dyn FnMut(UpdateProgressEvent),
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        // The lock is released here. Everything between this and the second
        // `with_authority` is the transfer, which is why `begin_prepare` and
        // `complete_prepare` exist as two calls at all.
        let start = self.with_authority(|authority| authority.begin_prepare(caller, command))??;
        let transfer = match start {
            UpdatePrepareStart::Refused(outcome) => return Ok(outcome),
            UpdatePrepareStart::Transfer(transfer) => transfer,
        };

        let epoch = transfer.authority_epoch();
        let delivered = self
            .fetch
            .fetch(transfer.request(), MAX_ARTIFACT_BYTES, &mut |report| {
                progress(UpdateProgressEvent {
                    protocol_version: UpdateProtocolVersion::CURRENT,
                    authority_epoch: epoch,
                    progress: report.into(),
                });
            });

        self.with_authority(|authority| authority.complete_prepare(caller, transfer, delivered))?
    }

    fn apply(
        &self,
        caller: &str,
        command: UpdateApplyCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.apply(caller, command))?
    }

    fn cancel(
        &self,
        caller: &str,
        command: UpdateCancelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.cancel(caller, command))?
    }
}
