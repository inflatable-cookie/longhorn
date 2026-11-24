use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, ThreadId};

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
    progress: ProgressDelivery,
    authority: Mutex<A>,
}

#[derive(Default)]
struct ProgressDelivery {
    state: Mutex<ProgressState>,
    finished: Condvar,
}

#[derive(Default)]
struct ProgressState {
    active: Option<Arc<()>>,
    publisher: Option<ThreadId>,
}

impl ProgressDelivery {
    fn lock(&self) -> Result<MutexGuard<'_, ProgressState>, UpdateHostError> {
        let thread = thread::current().id();
        let state = self
            .state
            .lock()
            .map_err(|_| UpdateHostError::state_unavailable())?;
        self.finished
            .wait_while(state, |state| {
                state.publisher.is_some_and(|owner| owner != thread)
            })
            .map_err(|_| UpdateHostError::state_unavailable())
    }

    fn publish(&self, transfer: &Arc<()>, emit: impl FnOnce()) {
        let Ok(mut state) = self.lock() else {
            return;
        };
        if !state
            .active
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, transfer))
        {
            return;
        }
        // A synchronous sink may call back into the assembly. Other threads
        // wait for the outermost publication before changing authority.
        let _publication = if state.publisher.is_none() {
            state.publisher = Some(thread::current().id());
            Some(Publication(self))
        } else {
            None
        };
        drop(state);
        emit();
    }
}

struct Publication<'a>(&'a ProgressDelivery);

impl Drop for Publication<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.publisher = None;
            self.0.finished.notify_all();
        }
    }
}

impl<A> UpdateHandlerAssembly<A> {
    /// Binds one explicitly injected consumer authority and the transfer port
    /// the assembly drives on its behalf.
    #[must_use]
    pub fn new(authority: A, fetch: Arc<dyn ArtifactFetch + Send + Sync>) -> Self {
        Self {
            fetch,
            progress: ProgressDelivery::default(),
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
        let mut progress_state = self.progress.lock()?;
        let start = self.with_authority(|authority| authority.begin_prepare(caller, command))??;
        let transfer = match start {
            UpdatePrepareStart::Refused(outcome) => return Ok(outcome),
            UpdatePrepareStart::Transfer(transfer) => transfer,
        };
        let report_token = Arc::new(());
        progress_state.active = Some(Arc::clone(&report_token));
        drop(progress_state);

        let epoch = transfer.authority_epoch();
        let delivered = self
            .fetch
            .fetch(transfer.request(), MAX_ARTIFACT_BYTES, &mut |report| {
                self.progress.publish(&report_token, || {
                    progress(UpdateProgressEvent {
                        protocol_version: UpdateProtocolVersion::CURRENT,
                        authority_epoch: epoch,
                        progress: report.into(),
                    });
                });
            });

        let mut progress_state = self.progress.lock()?;
        if progress_state
            .active
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, &report_token))
        {
            progress_state.active = None;
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
        let mut progress_state = self.progress.lock()?;
        let outcome = self.with_authority(action)??;
        if matches!(outcome, UpdateOutcomeProjection::Committed { .. }) {
            progress_state.active = None;
        }
        Ok(outcome)
    }
}
