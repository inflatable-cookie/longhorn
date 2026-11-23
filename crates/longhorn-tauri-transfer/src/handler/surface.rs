use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferCommand, SurfaceTransferResponse,
};
use longhorn_transfer::{TerminalTransferResolution, TransferErrorCode};
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
    /// Admits one whole-Surface transfer through the injected optional adapter.
    ///
    /// Releases the coordinator lock before calling the adapter so consumer
    /// configuration I/O never blocks concurrent transfer calls.
    pub fn start_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
        adapter: &mut impl SurfaceTransferAdapter,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let caller = {
            let state = self.lock_active()?;
            current_caller(&state, &runtime)
        };
        let Some(caller) = caller else {
            return Ok(SurfaceSessionResponse::Aborted {
                abort: SurfaceTransferAbort::host_transfer(
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
        Ok(adapter.start_surface(
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
            &mut registrar,
        ))
    }

    /// Commits one whole-Surface transfer through fresh checked host projection.
    ///
    /// Resolves and consumes the terminal target or empty-display attempt under
    /// the coordinator lock, then releases the lock before calling the adapter
    /// so configuration I/O and window provisioning never run while holding the
    /// coordinator mutex.
    pub fn commit_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceTransferCommand,
        adapter: &mut impl SurfaceTransferAdapter,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        let (selector, runtime) = self.project_selector(caller_handle, request.selector())?;
        let request_id = request.request_id().clone();
        let live_windows = runtime.live_windows();
        let (caller, resolution) = {
            let mut state = self.lock_active()?;
            let Some(caller) = current_caller(&state, &runtime) else {
                return Ok(SurfaceTransferResponse::Aborted {
                    abort: SurfaceTransferAbort::host_transfer(
                        request_id,
                        TransferErrorCode::UnknownClientEpoch,
                        "caller has no current registered transfer client",
                    ),
                });
            };
            let resolution = state.coordinator.attempt_target_or_empty_display(
                &self.clock,
                request.session_id(),
                selector,
                &live_windows,
            );
            (caller, resolution)
        };
        let resolution = match resolution {
            Ok(resolution) => resolution,
            Err(error) => {
                return Ok(SurfaceTransferResponse::Aborted {
                    abort: SurfaceTransferAbort::host_transfer(
                        request_id,
                        error.code(),
                        error.detail(),
                    ),
                });
            }
        };
        Ok(adapter.commit_surface(
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
            resolution,
        ))
    }
}

/// Narrow consumer adapter for authoritative whole-Surface admission and commit.
pub trait SurfaceTransferAdapter {
    /// Admits one Surface using fresh caller authority and the narrow session
    /// registrar. Runs without holding the coordinator lock.
    fn start_surface(
        &mut self,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
        sessions: &mut TransferSessionRegistrar<'_>,
    ) -> SurfaceSessionResponse;

    /// Commits one terminal Surface attempt after the coordinator has resolved
    /// and consumed the target or empty-display destination. Runs without
    /// holding the coordinator lock.
    fn commit_surface(
        &mut self,
        caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        resolution: TerminalTransferResolution,
    ) -> SurfaceTransferResponse;
}
