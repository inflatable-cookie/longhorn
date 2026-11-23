use longhorn_transfer::{
    PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse, TerminalTransferAttempt,
    TransferAbort, TransferErrorCode, TransferSessionResponse,
};
use longhorn_windowing::HostWindowHandle;

use super::{
    TransferCallerAuthority, TransferHandlerAssembly, TransferSessionRegistrar, current_caller,
};
use crate::{ManagedTransferRuntime, TransferHandlerError};

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: longhorn_transfer::MonotonicClock,
{
    /// Admits one panel transfer through the injected domain adapter.
    ///
    /// Releases the coordinator lock before calling the adapter so consumer
    /// configuration I/O never blocks or deadlocks concurrent transfer calls.
    pub fn start_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelSessionStartRequest,
        adapter: &mut impl PanelTransferAdapter,
    ) -> Result<TransferSessionResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let caller = {
            let state = self.lock_active()?;
            current_caller(&state, &runtime)
        };
        let Some(caller) = caller else {
            return Ok(TransferSessionResponse::Aborted {
                abort: TransferAbort::host_transfer(
                    request_id,
                    TransferErrorCode::UnknownClientEpoch,
                    "caller has no current registered transfer client",
                ),
            });
        };
        let mut create =
            |allocator: &mut dyn longhorn_transfer::DragSessionIdAllocator,
             session_request: longhorn_transfer::TransferSessionRequest| {
                self.create_session_dyn(allocator, session_request)
            };
        let mut registrar = TransferSessionRegistrar::new(&mut create);
        Ok(adapter.start_panel(
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
            &mut registrar,
        ))
    }

    /// Commits one panel transfer through fresh checked host projection.
    ///
    /// Resolves and consumes the terminal target attempt under the coordinator
    /// lock, then releases the lock before calling the adapter so configuration
    /// I/O never runs while holding the coordinator mutex.
    pub fn commit_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelTransferCommand,
        adapter: &mut impl PanelTransferAdapter,
    ) -> Result<PanelTransferResponse, TransferHandlerError> {
        let (selector, runtime) = self.project_selector(caller_handle, request.selector())?;
        let request_id = request.request_id().clone();
        let live_windows = runtime.live_windows();
        let (caller, attempt) = {
            let mut state = self.lock_active()?;
            let Some(caller) = current_caller(&state, &runtime) else {
                return Ok(PanelTransferResponse::Aborted {
                    abort: TransferAbort::host_transfer(
                        request_id,
                        TransferErrorCode::UnknownClientEpoch,
                        "caller has no current registered transfer client",
                    ),
                });
            };
            let attempt = state.coordinator.attempt_target_resolution(
                &self.clock,
                request.session_id(),
                selector,
                &live_windows,
            );
            (caller, attempt)
        };
        let attempt = match attempt {
            Ok(attempt) => attempt,
            Err(error) => {
                return Ok(PanelTransferResponse::Aborted {
                    abort: TransferAbort::from_transfer(request_id, &error),
                });
            }
        };
        Ok(adapter.commit_panel(
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
            attempt,
        ))
    }
}

/// Narrow consumer adapter for authoritative panel admission and commit.
pub trait PanelTransferAdapter {
    /// Admits one panel using fresh caller authority and the narrow session
    /// registrar. Runs without holding the coordinator lock.
    fn start_panel(
        &mut self,
        caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
        sessions: &mut TransferSessionRegistrar<'_>,
    ) -> TransferSessionResponse;

    /// Commits one terminal panel attempt after the coordinator has resolved
    /// and consumed the target. Runs without holding the coordinator lock.
    fn commit_panel(
        &mut self,
        caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        attempt: TerminalTransferAttempt,
    ) -> PanelTransferResponse;
}
