use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferAbort,
    SurfaceTransferCommand, SurfaceTransferResponse,
};
use longhorn_transfer::TerminalTransferResolution;
use longhorn_windowing::HostWindowHandle;

use super::{TransferCallerAuthority, TransferHandlerAssembly, TransferSessionRegistrar};
use crate::{ManagedTransferRuntime, TransferHandlerError};

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: longhorn_transfer::MonotonicClock,
{
    /// Admits one whole-Surface transfer through the injected adapter.
    ///
    /// The adapter runs without holding the coordinator lock.
    pub fn start_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
        adapter: &impl SurfaceTransferAdapter,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError> {
        let request_id = request.request_id().clone();
        Ok(
            match self.admit(caller_handle, |caller, sessions| {
                adapter.start_surface(caller, request, sessions)
            })? {
                Ok(response) => response,
                Err(error) => SurfaceSessionResponse::Aborted {
                    abort: error.into_abort(
                        request_id,
                        SurfaceTransferAbort::host_transfer,
                        SurfaceTransferAbort::from_transfer,
                    ),
                },
            },
        )
    }

    /// Resolves and consumes one whole-Surface attempt using fresh host geometry.
    ///
    /// The adapter runs after the coordinator lock is released.
    pub fn commit_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceTransferCommand,
        adapter: &impl SurfaceTransferAdapter,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        let request_id = request.request_id().clone();
        let session_id = request.session_id();
        Ok(
            match self.resolve_commit(
                caller_handle,
                request.selector(),
                |coordinator, clock, selector, live_windows| {
                    coordinator.attempt_target_or_empty_display(
                        clock,
                        session_id,
                        selector,
                        live_windows,
                    )
                },
            )? {
                Ok((caller, resolution)) => adapter.commit_surface(caller, request, resolution),
                Err(error) => SurfaceTransferResponse::Aborted {
                    abort: error.into_abort(
                        request_id,
                        SurfaceTransferAbort::host_transfer,
                        SurfaceTransferAbort::from_transfer,
                    ),
                },
            },
        )
    }
}

/// Narrow consumer adapter for authoritative whole-Surface admission and commit.
///
/// Tauri command assemblies may call the same adapter concurrently.
pub trait SurfaceTransferAdapter {
    /// Admits one Surface using fresh caller authority and the narrow session
    /// registrar. Runs without holding the coordinator lock.
    fn start_surface(
        &self,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
        sessions: &mut TransferSessionRegistrar<'_>,
    ) -> SurfaceSessionResponse;

    /// Commits one terminal Surface attempt after the coordinator has resolved
    /// and consumed the target or empty-display destination. Runs without
    /// holding the coordinator lock.
    fn commit_surface(
        &self,
        caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        resolution: TerminalTransferResolution,
    ) -> SurfaceTransferResponse;
}
