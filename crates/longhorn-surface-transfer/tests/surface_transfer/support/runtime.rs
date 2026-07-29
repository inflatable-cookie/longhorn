use std::{cell::Cell, collections::VecDeque};

use longhorn_core::{
    DomainId, ScreenPoint, ScreenRect, ScreenSize, TransferClientId, TransferHostBindingId,
};
use longhorn_surface_transfer::{
    SurfaceHostBinding, SurfaceHostBindings, SurfaceSessionAdmission, admit_surface_session,
};
use longhorn_transfer::{
    ClientEpoch, DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone,
    DropZoneId, LeaseGeneration, LeasePublication, LiveTransferWindow, MonotonicClock,
    TransferCapability, TransferCoordinator, TransferDuration, TransferInstant, TransferLimits,
    TransferRevision, TransferTargetBinding,
};

use super::{TestDomain, window_id};

pub struct FakeClock(Cell<u64>);

impl FakeClock {
    pub const fn new(now: u64) -> Self {
        Self(Cell::new(now))
    }
}

impl MonotonicClock for FakeClock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(self.0.get())
    }
}

pub struct SequenceAllocator(VecDeque<[u8; 16]>);

impl SequenceAllocator {
    pub fn new(values: impl IntoIterator<Item = [u8; 16]>) -> Self {
        Self(values.into_iter().collect())
    }
}

impl DragSessionIdAllocator for SequenceAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        self.0.pop_front().ok_or(DragSessionIdAllocationError)
    }
}

pub struct RuntimeFixture {
    pub clock: FakeClock,
    pub coordinator: TransferCoordinator,
    pub allocator: SequenceAllocator,
    pub bindings: SurfaceHostBindings,
}

impl RuntimeFixture {
    pub fn new() -> Self {
        let clock = FakeClock::new(0);
        let mut coordinator = TransferCoordinator::new(
            TransferLimits::new(
                8,
                8,
                8,
                8,
                20,
                TransferDuration::new(100),
                TransferDuration::new(50),
            )
            .unwrap(),
        );
        bind(&mut coordinator, &clock, "window:main", "client:main");
        bind(&mut coordinator, &clock, "window:target", "client:target");
        let target_bounds = ScreenRect::new(ScreenPoint::new(100, 100), ScreenSize::new(600, 500));
        coordinator
            .publish_lease(
                &clock,
                LeasePublication::new(
                    window_id("window:target"),
                    client_id("client:target"),
                    ClientEpoch::new(1),
                    LeaseGeneration::new(1),
                    TransferDuration::new(40),
                    target_bounds,
                    vec![DropZone::new(
                        DropZoneId::new("zone:surface-target").unwrap(),
                        ScreenRect::new(ScreenPoint::new(120, 120), ScreenSize::new(300, 200)),
                        None,
                        TransferCapability::MoveSurface,
                        TransferTargetBinding::SurfaceWindow {
                            host_binding_id: binding_id("binding:target"),
                            document_id: domain_id(),
                            revision: TransferRevision::new(7),
                        },
                    )],
                ),
            )
            .unwrap();
        Self {
            clock,
            coordinator,
            allocator: SequenceAllocator::new([[1; 16], [2; 16], [3; 16], [4; 16]]),
            bindings: SurfaceHostBindings::new([
                SurfaceHostBinding::new(
                    binding_id("binding:source"),
                    window_id("window:main"),
                    domain_id(),
                ),
                SurfaceHostBinding::new(
                    binding_id("binding:target"),
                    window_id("window:target"),
                    domain_id(),
                ),
            ])
            .unwrap(),
        }
    }

    pub fn admit(
        &mut self,
        store: &longhorn_config::ConfigStore,
        domain: &TestDomain,
    ) -> Result<DragSessionId, longhorn_surface_transfer::SurfaceTransferError> {
        admit_surface_session(
            store,
            domain,
            &mut self.coordinator,
            &self.clock,
            &mut self.allocator,
            &self.bindings,
            SurfaceSessionAdmission::new(
                window_id("window:main"),
                client_id("client:main"),
                ClientEpoch::new(1),
                super::surface_id("surface:a"),
                binding_id("binding:source"),
                TransferDuration::new(40),
            ),
        )
        .map(|receipt| receipt.payload().session_id())
    }

    pub fn live_target(&self) -> LiveTransferWindow {
        LiveTransferWindow::new(
            window_id("window:target"),
            ScreenRect::new(ScreenPoint::new(100, 100), ScreenSize::new(600, 500)),
        )
    }
}

pub fn binding_id(value: &str) -> TransferHostBindingId {
    TransferHostBindingId::new(value).unwrap()
}

fn bind(coordinator: &mut TransferCoordinator, clock: &FakeClock, window: &str, client: &str) {
    coordinator
        .bind_client_epoch(
            clock,
            window_id(window),
            client_id(client),
            ClientEpoch::new(1),
        )
        .unwrap();
}

fn client_id(value: &str) -> TransferClientId {
    TransferClientId::new(value).unwrap()
}

fn domain_id() -> DomainId {
    DomainId::new("surfaces.workspace").unwrap()
}
