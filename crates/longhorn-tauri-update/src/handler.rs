use std::sync::{Arc, Mutex};

use longhorn_update::{
    ArtifactFetch, MAX_ARTIFACT_BYTES, UpdateApplyCommand, UpdateCancelCommand, UpdateCheckCommand,
    UpdateDeferCommand, UpdateOutcomeProjection, UpdatePrepareCommand, UpdatePrepareStart,
    UpdateProgressEvent, UpdateProtocolVersion, UpdateSelectChannelCommand, UpdateSnapshot,
};

use crate::{UpdateHostAuthority, UpdateHostError};

/// Shared injected assembly used by Tauri and conformance tests.
///
/// Holds the transfer port as well as the authority so the authoritative state
/// is not locked while a transfer runs: `prepare` takes the lock to compose the
/// request, releases it, transfers, and takes it again to retain the result.
///
/// The port is an `Arc` rather than a borrow because Tauri managed state has to
/// be `'static`. A consumer builds one fetch, hands a clone here, and lends the
/// same instance to the `UpdateController` it injects as authority.
pub struct UpdateHandlerAssembly<A: ?Sized> {
    fetch: Arc<dyn ArtifactFetch + Send + Sync>,
    // Commands serialize changes to the slot. A report holds only a Weak so
    // invalidation never waits for the transfer or the progress sink.
    active_progress: Mutex<Option<Arc<()>>>,
    authority: Mutex<A>,
}

impl<A> UpdateHandlerAssembly<A> {
    /// Binds one explicitly injected consumer authority and the transfer port
    /// the assembly drives on its behalf.
    #[must_use]
    pub fn new(authority: A, fetch: Arc<dyn ArtifactFetch + Send + Sync>) -> Self {
        Self {
            fetch,
            active_progress: Mutex::new(None),
            authority: Mutex::new(authority),
        }
    }
}

impl<A: ?Sized> UpdateHandlerAssembly<A> {
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

impl<A> UpdateHandlerAssembly<A>
where
    A: UpdateHostAuthority + ?Sized,
{
    /// Returns the caller-authorized update state.
    pub fn snapshot(&self, caller: &str) -> Result<UpdateSnapshot, UpdateHostError> {
        self.with_authority(|authority| authority.snapshot(caller))?
    }

    /// Asks the source for the channel's current manifest and records it.
    pub fn check(
        &self,
        caller: &str,
        command: UpdateCheckCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.check(caller, command))?
    }

    /// Follows a different channel from now on.
    pub fn select_channel(
        &self,
        caller: &str,
        command: UpdateSelectChannelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.invalidate_progress_on_commit(|authority| authority.select_channel(caller, command))
    }

    /// Declines a version for now.
    pub fn defer(
        &self,
        caller: &str,
        command: UpdateDeferCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.defer(caller, command))?
    }

    /// Fetches, verifies and retains, reporting progress as bytes arrive.
    ///
    /// The authority lock is released for the download. The active transfer
    /// alone may hand progress to the sink.
    pub fn prepare(
        &self,
        caller: &str,
        command: UpdatePrepareCommand,
        progress: &mut dyn FnMut(UpdateProgressEvent),
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        let mut progress_slot = self
            .active_progress
            .lock()
            .map_err(|_| UpdateHostError::state_unavailable())?;
        let start = self.with_authority(|authority| authority.begin_prepare(caller, command))??;
        let transfer = match start {
            UpdatePrepareStart::Refused(outcome) => return Ok(outcome),
            UpdatePrepareStart::Transfer(transfer) => transfer,
        };
        let active = Arc::new(());
        let report_token = Arc::downgrade(&active);
        *progress_slot = Some(active);
        drop(progress_slot);

        let epoch = transfer.authority_epoch();
        let delivered = self
            .fetch
            .fetch(transfer.request(), MAX_ARTIFACT_BYTES, &mut |report| {
                if report_token.strong_count() == 0 {
                    return;
                }
                progress(UpdateProgressEvent {
                    protocol_version: UpdateProtocolVersion::CURRENT,
                    authority_epoch: epoch,
                    progress: report.into(),
                });
            });

        let mut progress_slot = self
            .active_progress
            .lock()
            .map_err(|_| UpdateHostError::state_unavailable())?;
        if progress_slot
            .as_ref()
            .is_some_and(|active| Arc::as_ptr(active) == report_token.as_ptr())
        {
            *progress_slot = None;
        }
        self.with_authority(|authority| authority.complete_prepare(caller, transfer, delivered))?
    }

    /// Applies the retained staged artifact.
    pub fn apply(
        &self,
        caller: &str,
        command: UpdateApplyCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.with_authority(|authority| authority.apply(caller, command))?
    }

    /// Discards the retained staged artifact.
    pub fn cancel(
        &self,
        caller: &str,
        command: UpdateCancelCommand,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        self.invalidate_progress_on_commit(|authority| authority.cancel(caller, command))
    }

    fn invalidate_progress_on_commit(
        &self,
        action: impl FnOnce(&mut A) -> Result<UpdateOutcomeProjection, UpdateHostError>,
    ) -> Result<UpdateOutcomeProjection, UpdateHostError> {
        let mut progress_slot = self
            .active_progress
            .lock()
            .map_err(|_| UpdateHostError::state_unavailable())?;
        let outcome = self.with_authority(action)??;
        if matches!(outcome, UpdateOutcomeProjection::Committed { .. }) {
            *progress_slot = None;
        }
        Ok(outcome)
    }
}
