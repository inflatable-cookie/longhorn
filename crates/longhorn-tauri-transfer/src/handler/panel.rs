use longhorn_transfer::{
    LiveTransferWindow, PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse,
    TargetSelector, TransferAbort, TransferCommitSelector, TransferCoordinator, TransferErrorCode,
    TransferSessionResponse,
};
use longhorn_windowing::HostWindowHandle;

use super::{TransferCallerAuthority, TransferHandlerAssembly, current_caller};
use crate::{ManagedTransferRuntime, TransferHandlerError};

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: longhorn_transfer::MonotonicClock,
{
    /// Admits one panel transfer through the injected domain adapter.
    pub fn start_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelSessionStartRequest,
        adapter: &mut impl PanelTransferAdapter,
    ) -> Result<TransferSessionResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let mut state = self.lock_active()?;
        let Some(caller) = current_caller(&state, &runtime) else {
            return Ok(TransferSessionResponse::Aborted {
                abort: TransferAbort::host_transfer(
                    request_id,
                    TransferErrorCode::UnknownClientEpoch,
                    "caller has no current registered transfer client",
                ),
            });
        };
        Ok(adapter.start_panel(
            &mut state.coordinator,
            TransferCallerAuthority::new(
                runtime.caller().window_id().clone(),
                caller,
                self.session_lifetime,
            ),
            request,
        ))
    }

    /// Commits one panel transfer through fresh checked host projection.
    pub fn commit_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelTransferCommand,
        adapter: &mut impl PanelTransferAdapter,
    ) -> Result<PanelTransferResponse, TransferHandlerError> {
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
            return Ok(PanelTransferResponse::Aborted {
                abort: TransferAbort::host_transfer(
                    request_id,
                    TransferErrorCode::UnknownClientEpoch,
                    "caller has no current registered transfer client",
                ),
            });
        };
        Ok(adapter.commit_panel(
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

/// Narrow consumer adapter for authoritative panel admission and commit.
pub trait PanelTransferAdapter {
    /// Admits one panel using fresh caller authority and the shared coordinator.
    fn start_panel(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
    ) -> TransferSessionResponse;

    /// Commits one terminal panel attempt using fresh host target evidence.
    fn commit_panel(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        selector: TargetSelector,
        live_windows: Vec<LiveTransferWindow>,
    ) -> PanelTransferResponse;
}
