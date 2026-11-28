use std::{
    collections::BTreeSet,
    sync::{Condvar, Mutex},
};

use longhorn_core::{TransferClientId, TransferRequestId, WindowId};
use longhorn_transfer::{
    ClientEpoch, DragSessionIdAllocator, DropZone, LeasePublication, LiveTransferWindow,
    MonotonicClock, SessionCreationReceipt, TargetSelector, TransferAbort, TransferCancelReceipt,
    TransferCancelRequest, TransferCancelResponse, TransferClientSnapshot, TransferCommitSelector,
    TransferCoordinator, TransferDuration, TransferError, TransferErrorCode, TransferLeaseReceipt,
    TransferLeaseRequest, TransferLeaseResponse, TransferLimits, TransferSessionRequest,
};
use longhorn_windowing::HostWindowHandle;

use crate::{
    ManagedTransferRuntime, ManagedTransferSnapshot, TransferHandlerError, TransferRuntimeError,
    project_client_rect,
};

mod panel;
#[cfg(feature = "surface-transfer")]
mod surface;
mod types;

pub use panel::PanelTransferAdapter;
#[cfg(feature = "surface-transfer")]
pub use surface::SurfaceTransferAdapter;
pub use types::{
    TransferCallerAuthority, TransferHandlerTeardownReceipt, TransferHandlerTeardownStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct CurrentClient {
    client_id: TransferClientId,
    epoch: ClientEpoch,
}

#[derive(Debug)]
struct HandlerState {
    coordinator: TransferCoordinator,
    next_client: u64,
    publishing: BTreeSet<HostWindowHandle>,
    active: bool,
}

enum AttemptRejection {
    UnknownCaller,
    Coordination(TransferError),
}

impl AttemptRejection {
    fn into_abort<A>(
        self,
        request_id: TransferRequestId,
        host_abort: impl FnOnce(TransferRequestId, TransferErrorCode, &'static str) -> A,
        transfer_abort: impl FnOnce(TransferRequestId, &TransferError) -> A,
    ) -> A {
        match self {
            Self::UnknownCaller => host_abort(
                request_id,
                TransferErrorCode::UnknownClientEpoch,
                "caller has no current registered transfer client",
            ),
            Self::Coordination(error) => transfer_abort(request_id, &error),
        }
    }
}

type CreateSessionFn<'a> = dyn FnMut(
        &mut dyn DragSessionIdAllocator,
        TransferSessionRequest,
    ) -> Result<Result<SessionCreationReceipt, TransferError>, TransferHandlerError>
    + 'a;

/// Narrow coordinator handle passed to domain admission adapters so they can
/// register one bounded session without holding the handler lock across I/O
/// or receiving raw `&mut TransferCoordinator` access.
pub struct TransferSessionRegistrar<'a> {
    create: &'a mut CreateSessionFn<'a>,
}

impl<'a> TransferSessionRegistrar<'a> {
    pub(super) fn new(create: &'a mut CreateSessionFn<'a>) -> Self {
        Self { create }
    }

    /// Registers one bounded transfer session under the handler's coordinator lock.
    pub fn create_session(
        &mut self,
        allocator: &mut impl DragSessionIdAllocator,
        request: TransferSessionRequest,
    ) -> Result<Result<SessionCreationReceipt, TransferError>, TransferHandlerError> {
        (self.create)(allocator, request)
    }
}

struct DynAllocator<'a>(&'a mut dyn DragSessionIdAllocator);

impl DragSessionIdAllocator for DynAllocator<'_> {
    fn allocate(&mut self) -> Result<[u8; 16], longhorn_transfer::DragSessionIdAllocationError> {
        self.0.allocate()
    }
}

/// Shared transfer handler core used by real and mock managed-window runtimes.
pub struct TransferHandlerAssembly<R, C> {
    runtime: R,
    clock: C,
    session_lifetime: TransferDuration,
    lease_lifetime: TransferDuration,
    state: Mutex<HandlerState>,
    publication_finished: Condvar,
}

struct SnapshotPublication<'a, R, C> {
    handler: &'a TransferHandlerAssembly<R, C>,
    caller: &'a HostWindowHandle,
}

impl<R, C> Drop for SnapshotPublication<'_, R, C> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.handler.state.lock() {
            state.publishing.remove(self.caller);
        }
        self.handler.publication_finished.notify_all();
    }
}

impl<R, C> TransferHandlerAssembly<R, C>
where
    R: ManagedTransferRuntime,
    C: MonotonicClock,
{
    /// Constructs active finite transfer handler state.
    #[must_use]
    pub fn new(runtime: R, clock: C, limits: TransferLimits) -> Self {
        Self {
            runtime,
            clock,
            session_lifetime: limits.maximum_session_lifetime(),
            lease_lifetime: limits.maximum_lease_lifetime(),
            state: Mutex::new(HandlerState {
                coordinator: TransferCoordinator::new(limits),
                next_client: 1,
                publishing: BTreeSet::new(),
                active: true,
            }),
            publication_finished: Condvar::new(),
        }
    }

    /// Registers a fresh renderer epoch and returns current caller authority.
    pub fn snapshot(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        self.snapshot_for_window(caller_handle)
            .map(|(_, snapshot)| snapshot)
    }

    pub(crate) fn snapshot_and_publish(
        &self,
        caller_handle: &HostWindowHandle,
        publish: impl FnOnce(&TransferClientSnapshot),
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        let (window_id, snapshot) = self.snapshot_for_window(caller_handle)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        loop {
            if state.coordinator.current_client(&window_id)
                != Some((snapshot.client_id(), snapshot.client_epoch()))
            {
                return Ok(snapshot);
            }
            if !state.publishing.contains(caller_handle) {
                break;
            }
            state = self
                .publication_finished
                .wait(state)
                .map_err(|_| TransferHandlerError::StateUnavailable)?;
        }
        // Order delivery by webview label, including across a destroy and rebind.
        // The permit lives only for this emission, with no mutex held by the emitter.
        state.publishing.insert(caller_handle.clone());
        drop(state);
        let _publication = SnapshotPublication {
            handler: self,
            caller: caller_handle,
        };
        publish(&snapshot);
        Ok(snapshot)
    }

    fn snapshot_for_window(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<(WindowId, TransferClientSnapshot), TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let window_id = runtime.caller().window_id().clone();
        let client = {
            let mut state = self.lock_active()?;
            let client = issue_client(&mut state)?;
            state
                .coordinator
                .bind_client_epoch(
                    &self.clock,
                    window_id.clone(),
                    client.client_id.clone(),
                    client.epoch,
                )
                .map_err(|error| {
                    TransferHandlerError::ClientBinding(error.code(), error.detail().to_owned())
                })?;
            client
        };
        // Close the probe/bind race with destroy_window: a destroy ordered
        // before this recheck either already removed the binding or is undone
        // here; one ordered after it removes the binding itself. Without this,
        // a window destroyed between the unlocked probe and the bind would
        // leak a client slot until teardown.
        let recheck = self.runtime.snapshot(caller_handle).and_then(|runtime| {
            if runtime.caller().window_id() == &window_id {
                Ok(())
            } else {
                Err(TransferRuntimeError::UnmanagedCaller(caller_handle.clone()))
            }
        });
        if let Err(error) = recheck {
            let mut state = self.lock_active()?;
            if state
                .coordinator
                .current_client(&window_id)
                .is_some_and(|(current_id, _)| *current_id == client.client_id)
            {
                state.coordinator.destroy_window(&window_id);
            }
            return Err(error.into());
        }
        Ok((
            window_id,
            TransferClientSnapshot::new(client.client_id, client.epoch),
        ))
    }

    /// Projects and atomically publishes one complete caller-window lease.
    pub fn publish_lease(
        &self,
        caller_handle: &HostWindowHandle,
        request: TransferLeaseRequest,
    ) -> Result<TransferLeaseResponse, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let zones = match project_zones(&runtime, &request) {
            Ok(zones) => zones,
            Err(error) => {
                return Ok(TransferLeaseResponse::Aborted {
                    abort: TransferAbort::invalid_lease(request_id, error.to_string()),
                });
            }
        };
        let publication = LeasePublication::new(
            runtime.caller().window_id().clone(),
            request.client_id().clone(),
            request.client_epoch(),
            request.generation(),
            self.lease_lifetime,
            runtime.caller().outer_bounds(),
            zones,
        );
        let mut state = self.lock_active()?;
        match state.coordinator.publish_lease(&self.clock, publication) {
            Ok(receipt) => Ok(TransferLeaseResponse::Published {
                lease: TransferLeaseReceipt::new(
                    request_id,
                    request.client_id().clone(),
                    request.client_epoch(),
                    receipt.generation(),
                    receipt.zone_count(),
                ),
            }),
            Err(error) => Ok(TransferLeaseResponse::Aborted {
                abort: TransferAbort::from_transfer(request_id, &error),
            }),
        }
    }

    /// Cancels one bounded session after confirming the caller is managed.
    pub fn cancel(
        &self,
        caller_handle: &HostWindowHandle,
        request: TransferCancelRequest,
    ) -> Result<TransferCancelResponse, TransferHandlerError> {
        self.runtime.snapshot(caller_handle)?;
        let request_id = request.request_id().clone();
        let mut state = self.lock_active()?;
        match state
            .coordinator
            .cancel_session(&self.clock, request.session_id())
        {
            Ok(receipt) => Ok(TransferCancelResponse::Cancelled {
                cancellation: TransferCancelReceipt::from_domain(request_id, receipt),
            }),
            Err(error) => Ok(TransferCancelResponse::Aborted {
                abort: TransferAbort::from_transfer(request_id, &error),
            }),
        }
    }

    fn admit<T>(
        &self,
        caller_handle: &HostWindowHandle,
        start: impl FnOnce(TransferCallerAuthority, &mut TransferSessionRegistrar<'_>) -> T,
    ) -> Result<Result<T, AttemptRejection>, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let caller = {
            let state = self.lock_active()?;
            self.caller_authority(&state, &runtime)
        };
        let Some(caller) = caller else {
            return Ok(Err(AttemptRejection::UnknownCaller));
        };
        let mut create = |allocator: &mut dyn DragSessionIdAllocator,
                          request: TransferSessionRequest| {
            self.create_session_dyn(allocator, request)
        };
        let mut registrar = TransferSessionRegistrar::new(&mut create);
        Ok(Ok(start(caller, &mut registrar)))
    }

    fn resolve_commit<A>(
        &self,
        caller_handle: &HostWindowHandle,
        selector: &TransferCommitSelector,
        resolve: impl FnOnce(
            &mut TransferCoordinator,
            &C,
            TargetSelector,
            &[LiveTransferWindow],
        ) -> Result<A, TransferError>,
    ) -> Result<Result<(TransferCallerAuthority, A), AttemptRejection>, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let selector = match selector {
            TransferCommitSelector::ExplicitZone { drop_zone_id } => {
                TargetSelector::ExplicitZone(drop_zone_id.clone())
            }
            TransferCommitSelector::ScreenPoint { point } => TargetSelector::ScreenPoint(*point),
        };
        let live_windows = runtime.live_windows();
        let mut state = self.lock_active()?;
        let Some(caller) = self.caller_authority(&state, &runtime) else {
            return Ok(Err(AttemptRejection::UnknownCaller));
        };
        Ok(
            resolve(&mut state.coordinator, &self.clock, selector, &live_windows)
                .map(|attempt| (caller, attempt))
                .map_err(AttemptRejection::Coordination),
        )
    }

    fn caller_authority(
        &self,
        state: &HandlerState,
        runtime: &ManagedTransferSnapshot,
    ) -> Option<TransferCallerAuthority> {
        let window_id = runtime.caller().window_id();
        let (client_id, epoch) = state.coordinator.current_client(window_id)?;
        Some(TransferCallerAuthority::new(
            window_id.clone(),
            CurrentClient {
                client_id: client_id.clone(),
                epoch,
            },
            self.session_lifetime,
        ))
    }

    /// Invalidates process-local authority after a trusted managed-window destroy.
    pub fn destroy_window(&self, window_id: &WindowId) -> Result<(), TransferHandlerError> {
        let mut state = self.lock_active()?;
        state.coordinator.destroy_window(window_id);
        Ok(())
    }

    /// Discards all process-local authority once and remains idempotent.
    pub fn teardown(&self) -> Result<TransferHandlerTeardownReceipt, TransferHandlerError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        if !state.active {
            return Ok(TransferHandlerTeardownReceipt {
                status: TransferHandlerTeardownStatus::AlreadyTornDown,
                sessions: 0,
                client_windows: 0,
                leases: 0,
            });
        }
        let discarded = state.coordinator.discard_all();
        state.active = false;
        Ok(TransferHandlerTeardownReceipt {
            status: TransferHandlerTeardownStatus::TornDown,
            sessions: discarded.sessions(),
            client_windows: discarded.client_windows(),
            leases: discarded.leases(),
        })
    }

    fn lock_active(&self) -> Result<std::sync::MutexGuard<'_, HandlerState>, TransferHandlerError> {
        let state = self
            .state
            .lock()
            .map_err(|_| TransferHandlerError::StateUnavailable)?;
        if !state.active {
            return Err(TransferHandlerError::Inactive);
        }
        Ok(state)
    }

    pub(super) fn create_session_dyn(
        &self,
        allocator: &mut dyn DragSessionIdAllocator,
        request: TransferSessionRequest,
    ) -> Result<Result<SessionCreationReceipt, TransferError>, TransferHandlerError> {
        let mut state = self.lock_active()?;
        let mut dyn_allocator = DynAllocator(allocator);
        Ok(state
            .coordinator
            .create_session(&self.clock, &mut dyn_allocator, request))
    }
}

fn issue_client(state: &mut HandlerState) -> Result<CurrentClient, TransferHandlerError> {
    let value = state.next_client;
    state.next_client = value
        .checked_add(1)
        .ok_or(TransferHandlerError::IdentityExhausted)?;
    let client_id = TransferClientId::new(format!("client:{value:016x}"))
        .map_err(|_| TransferHandlerError::InvalidIssuedClientId)?;
    Ok(CurrentClient {
        client_id,
        epoch: ClientEpoch::new(value),
    })
}

fn project_zones(
    runtime: &ManagedTransferSnapshot,
    request: &TransferLeaseRequest,
) -> Result<Vec<DropZone>, crate::TransferProjectionError> {
    request
        .zones()
        .iter()
        .map(|zone| {
            Ok(DropZone::new(
                zone.id().clone(),
                project_client_rect(runtime.caller(), zone.bounds())?,
                zone.insertion_position(),
                zone.target().clone(),
            ))
        })
        .collect()
}
