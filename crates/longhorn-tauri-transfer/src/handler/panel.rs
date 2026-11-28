use longhorn_transfer::{
    PanelSessionStartRequest, PanelTransferCommand, PanelTransferResponse, TerminalTransferAttempt,
    TransferAbort, TransferSessionResponse,
};
use longhorn_windowing::HostWindowHandle;

use super::{TransferCallerAuthority, TransferHandlerAssembly, TransferSessionRegistrar};
use crate::{ManagedTransferRuntime, TransferHandlerError};

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: longhorn_transfer::MonotonicClock,
{
    /// Admits one panel transfer through the injected adapter.
    ///
    /// The adapter runs without holding the coordinator lock.
    pub fn start_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelSessionStartRequest,
        adapter: &impl PanelTransferAdapter,
    ) -> Result<TransferSessionResponse, TransferHandlerError> {
        let request_id = request.request_id().clone();
        Ok(
            match self.admit(caller_handle, |caller, sessions| {
                adapter.start_panel(caller, request, sessions)
            })? {
                Ok(response) => response,
                Err(error) => TransferSessionResponse::Aborted {
                    abort: error.into_abort(
                        request_id,
                        TransferAbort::host_transfer,
                        TransferAbort::from_transfer,
                    ),
                },
            },
        )
    }

    /// Resolves and consumes one panel attempt using fresh host geometry.
    ///
    /// The adapter runs after the coordinator lock is released.
    pub fn commit_panel(
        &self,
        caller_handle: &HostWindowHandle,
        request: PanelTransferCommand,
        adapter: &impl PanelTransferAdapter,
    ) -> Result<PanelTransferResponse, TransferHandlerError> {
        let request_id = request.request_id().clone();
        let session_id = request.session_id();
        Ok(
            match self.resolve_commit(
                caller_handle,
                request.selector(),
                |coordinator, clock, selector, live_windows| {
                    coordinator.attempt_target_resolution(clock, session_id, selector, live_windows)
                },
            )? {
                Ok((caller, attempt)) => adapter.commit_panel(caller, request, attempt),
                Err(error) => PanelTransferResponse::Aborted {
                    abort: error.into_abort(
                        request_id,
                        TransferAbort::host_transfer,
                        TransferAbort::from_transfer,
                    ),
                },
            },
        )
    }
}

/// Narrow consumer adapter for authoritative panel admission and commit.
///
/// Tauri command assemblies may call the same adapter concurrently.
pub trait PanelTransferAdapter {
    /// Admits one panel using fresh caller authority and the narrow session
    /// registrar. Runs without holding the coordinator lock.
    fn start_panel(
        &self,
        caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
        sessions: &mut TransferSessionRegistrar<'_>,
    ) -> TransferSessionResponse;

    /// Commits one terminal panel attempt after the coordinator has resolved
    /// and consumed the target. Runs without holding the coordinator lock.
    fn commit_panel(
        &self,
        caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        attempt: TerminalTransferAttempt,
    ) -> PanelTransferResponse;
}
