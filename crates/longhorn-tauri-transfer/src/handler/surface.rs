use longhorn_surface_transfer::{
    SurfaceSessionResponse, SurfaceSessionStartRequest, SurfaceTransferCommand,
    SurfaceTransferResponse,
};
use longhorn_transfer::{
    LiveTransferWindow, TargetSelector, TransferCommitSelector, TransferCoordinator,
    TransferErrorCode,
};
use longhorn_windowing::HostWindowHandle;

use super::{TransferCallerAuthority, TransferHandlerAssembly, current_caller};
use crate::{ManagedTransferRuntime, TransferHandlerError};

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: longhorn_transfer::MonotonicClock,
{
    /// Admits one whole-Surface transfer through the injected optional adapter.
    pub fn start_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceSessionStartRequest,
        adapter: &mut impl SurfaceTransferAdapter,
    ) -> Result<SurfaceSessionResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let mut state = self.lock_active()?;
        let Some(caller) = current_caller(&state, &runtime) else {
            return Ok(SurfaceSessionResponse::Aborted {
                abort: longhorn_surface_transfer::SurfaceTransferAbort::host_transfer(
                    request_id,
                    TransferErrorCode::UnknownClientEpoch,
                    "caller has no current registered transfer client",
                ),
            });
        };
        Ok(adapter.start_surface(
            &mut state.coordinator,
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
        ))
    }

    /// Commits one whole-Surface transfer through fresh checked host projection.
    pub fn commit_surface(
        &self,
        caller_handle: &HostWindowHandle,
        request: SurfaceTransferCommand,
        adapter: &mut impl SurfaceTransferAdapter,
    ) -> Result<SurfaceTransferResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let selector = match request.selector() {
            TransferCommitSelector::ExplicitZone { drop_zone_id } => {
                TargetSelector::ExplicitZone(drop_zone_id.clone())
            }
            TransferCommitSelector::ScreenPoint { point } => TargetSelector::ScreenPoint(*point),
        };
        let live_windows = runtime.live_windows();
        let mut state = self.lock_active()?;
        let Some(caller) = current_caller(&state, &runtime) else {
            return Ok(SurfaceTransferResponse::Aborted {
                abort: longhorn_surface_transfer::SurfaceTransferAbort::host_transfer(
                    request_id,
                    TransferErrorCode::UnknownClientEpoch,
                    "caller has no current registered transfer client",
                ),
            });
        };
        Ok(adapter.commit_surface(
            &mut state.coordinator,
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
            selector,
            live_windows,
        ))
    }
}

/// Narrow consumer adapter for authoritative whole-Surface admission and commit.
pub trait SurfaceTransferAdapter {
    /// Admits one Surface using fresh caller authority and the shared coordinator.
    fn start_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
    ) -> SurfaceSessionResponse;

    /// Commits one terminal Surface attempt using fresh host target evidence.
    fn commit_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        selector: TargetSelector,
        live_windows: Vec<LiveTransferWindow>,
    ) -> SurfaceTransferResponse;
}
