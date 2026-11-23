use std::{collections::BTreeMap, sync::Mutex};

use longhorn_core::{TransferClientId, WindowId};
use longhorn_transfer::{
    ClientEpoch, DragSessionIdAllocator, DropZone, LeasePublication, MonotonicClock,
    SessionCreationReceipt, TargetSelector, TransferAbort, TransferCancelReceipt,
    TransferCancelRequest, TransferCancelResponse, TransferClientSnapshot, TransferCommitSelector,
    TransferCoordinator, TransferDuration, TransferError, TransferLeaseReceipt,
    TransferLeaseRequest, TransferLeaseResponse, TransferLimits, TransferSessionRequest,
};
use longhorn_windowing::HostWindowHandle;

use crate::{
    ManagedTransferRuntime, ManagedTransferSnapshot, TransferHandlerError, project_client_rect,
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
    epoch_high_water: BTreeMap<WindowId, ClientEpoch>,
    next_client: u64,
    active: bool,
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
                epoch_high_water: BTreeMap::new(),
                next_client: 1,
                active: true,
            }),
        }
    }

    /// Registers a fresh renderer epoch and returns current caller authority.
    pub fn snapshot(
        &self,
        caller_handle: &HostWindowHandle,
    ) -> Result<TransferClientSnapshot, TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let window_id = runtime.caller().window_id().clone();
        let client_id = {
            let mut state = self.lock_active()?;
            let epoch = state.epoch_high_water.get(&window_id).map_or(
                Ok(ClientEpoch::new(1)),
                |current| {
                    current
                        .get()
                        .checked_add(1)
                        .map(ClientEpoch::new)
                        .ok_or(TransferHandlerError::IdentityExhausted)
                },
            )?;
            let client_id = issue_client_id(&mut state)?;
            state
                .coordinator
                .bind_client_epoch(&self.clock, window_id.clone(), client_id.clone(), epoch)
                .map_err(|error| {
                    TransferHandlerError::ClientBinding(error.code(), error.detail().to_owned())
                })?;
            state.epoch_high_water.insert(window_id.clone(), epoch);
            CurrentClient { client_id, epoch }
        };
        // Close the probe/bind race with destroy_window: a destroy ordered
        // before this recheck either already removed the binding or is undone
        // here; one ordered after it removes the binding itself. Without this,
        // a window destroyed between the unlocked probe and the bind would
        // leak a client slot until teardown.
        if let Err(error) = self.runtime.snapshot(caller_handle) {
            let mut state = self.lock_active()?;
            if state
                .coordinator
                .current_client(&window_id)
                .is_some_and(|(current_id, _)| *current_id == client_id.client_id)
            {
                state.coordinator.destroy_window(&window_id);
            }
            return Err(error.into());
        }
        Ok(TransferClientSnapshot::new(
            client_id.client_id,
            client_id.epoch,
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

    /// Converts one renderer selector using fresh caller and live-window evidence.
    pub(crate) fn project_selector(
        &self,
        caller_handle: &HostWindowHandle,
        selector: &TransferCommitSelector,
    ) -> Result<(TargetSelector, ManagedTransferSnapshot), TransferHandlerError> {
        let runtime = self.runtime.snapshot(caller_handle)?;
        let selector = match selector {
            TransferCommitSelector::ExplicitZone { drop_zone_id } => {
                TargetSelector::ExplicitZone(drop_zone_id.clone())
            }
            TransferCommitSelector::ScreenPoint { point } => TargetSelector::ScreenPoint(*point),
        };
        Ok((selector, runtime))
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
        state.epoch_high_water.clear();
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

fn current_caller(
    state: &HandlerState,
    runtime: &ManagedTransferSnapshot,
) -> Option<CurrentClient> {
    state
        .coordinator
        .current_client(runtime.caller().window_id())
        .map(|(client_id, epoch)| CurrentClient {
            client_id: client_id.clone(),
            epoch,
        })
}

fn issue_client_id(state: &mut HandlerState) -> Result<TransferClientId, TransferHandlerError> {
    let value = state.next_client;
    state.next_client = value
        .checked_add(1)
        .ok_or(TransferHandlerError::IdentityExhausted)?;
    TransferClientId::new(format!("client:{value:016x}"))
        .map_err(|_| TransferHandlerError::InvalidIssuedClientId)
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
