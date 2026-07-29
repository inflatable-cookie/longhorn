use std::{cell::Cell, collections::VecDeque};

use longhorn_config::ConfigStore;
use longhorn_core::{
    DomainId, DropZoneId, LayoutContainerId, RegionId, ScreenPoint, ScreenRect, ScreenSize,
    TransferHostBindingId, WindowId,
};
use longhorn_transfer::{
    ClientEpoch, DragSessionId, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone,
    InsertionPosition, LeaseGeneration, LeasePublication, LiveTransferWindow, MonotonicClock,
    PanelHostBinding, PanelHostBindingKind, PanelHostBindings, PanelSessionAdmission,
    PanelTransferCommitRequest, PanelTransferOperation, TargetSelector, TransferCapability,
    TransferClientId, TransferCoordinator, TransferDuration, TransferInstant, TransferLimits,
    TransferRevision, TransferTargetBinding, admit_panel_session,
};

use super::{TestDomain, domain_id, main_region, source_container, target_container, tool_panel};

pub const SOURCE_WINDOW: &str = "window:source";
pub const TARGET_WINDOW: &str = "window:target";
pub const SOURCE_CLIENT: &str = "client:source";
pub const TARGET_CLIENT: &str = "client:target";
pub const SOURCE_BINDING: &str = "binding:source";
pub const TARGET_BINDING: &str = "binding:target";
pub const TARGET_ZONE: &str = "zone:target";

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

pub struct Allocator {
    values: VecDeque<[u8; 16]>,
    calls: usize,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            values: VecDeque::from([[7; 16], [8; 16], [9; 16]]),
            calls: 0,
        }
    }

    pub const fn calls(&self) -> usize {
        self.calls
    }
}

impl DragSessionIdAllocator for Allocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        self.calls += 1;
        self.values.pop_front().ok_or(DragSessionIdAllocationError)
    }
}

pub struct Runtime {
    pub coordinator: TransferCoordinator,
    pub clock: FakeClock,
    pub bindings: PanelHostBindings,
    pub session_id: DragSessionId,
}

impl Runtime {
    pub fn admit(store: &ConfigStore, domain: &TestDomain, kind: PanelHostBindingKind) -> Self {
        let clock = FakeClock::new(10);
        let mut coordinator = coordinator(&clock);
        let bindings = bindings(kind, domain.descriptor().id().clone());
        let mut allocator = Allocator::new();
        let receipt = admit_panel_session(
            store,
            domain,
            &mut coordinator,
            &clock,
            &mut allocator,
            &bindings,
            PanelSessionAdmission::new(
                window(SOURCE_WINDOW),
                client(SOURCE_CLIENT),
                ClientEpoch::new(1),
                tool_panel(),
                binding_id(SOURCE_BINDING),
                TransferDuration::new(40),
            ),
        )
        .unwrap();
        Self {
            coordinator,
            clock,
            bindings,
            session_id: receipt.payload().session_id(),
        }
    }

    pub fn publish_zone(
        &mut self,
        document_id: DomainId,
        revision: u64,
        container_id: LayoutContainerId,
        region_id: RegionId,
        insertion: Option<u32>,
    ) {
        self.coordinator
            .publish_lease(
                &self.clock,
                LeasePublication::new(
                    window(TARGET_WINDOW),
                    client(TARGET_CLIENT),
                    ClientEpoch::new(1),
                    LeaseGeneration::new(1),
                    TransferDuration::new(30),
                    target_bounds(),
                    vec![DropZone::new(
                        DropZoneId::new(TARGET_ZONE).unwrap(),
                        target_bounds(),
                        insertion.map(InsertionPosition::new),
                        TransferCapability::MovePanel,
                        TransferTargetBinding::PanelRegion {
                            host_binding_id: binding_id(TARGET_BINDING),
                            document_id,
                            revision: TransferRevision::new(revision),
                            container_id,
                            region_id,
                        },
                    )],
                ),
            )
            .unwrap();
    }

    pub fn publish_default_zone(&mut self) {
        self.publish_zone(domain_id(), 7, target_container(), main_region(), None);
    }

    pub fn commit_request(&self, operation: PanelTransferOperation) -> PanelTransferCommitRequest {
        PanelTransferCommitRequest::new(
            self.session_id,
            TargetSelector::ExplicitZone(DropZoneId::new(TARGET_ZONE).unwrap()),
            [
                LiveTransferWindow::new(window(SOURCE_WINDOW), source_bounds()),
                LiveTransferWindow::new(window(TARGET_WINDOW), target_bounds()),
            ],
            operation,
        )
    }
}

pub fn coordinator(clock: &FakeClock) -> TransferCoordinator {
    let mut coordinator = TransferCoordinator::new(
        TransferLimits::new(
            8,
            8,
            8,
            8,
            32,
            TransferDuration::new(100),
            TransferDuration::new(50),
        )
        .unwrap(),
    );
    for (window_id, client_id) in [
        (SOURCE_WINDOW, SOURCE_CLIENT),
        (TARGET_WINDOW, TARGET_CLIENT),
    ] {
        coordinator
            .bind_client_epoch(
                clock,
                window(window_id),
                client(client_id),
                ClientEpoch::new(1),
            )
            .unwrap();
    }
    coordinator
}

pub fn bindings(kind: PanelHostBindingKind, document_id: DomainId) -> PanelHostBindings {
    let make = |id, window_id, container_id| match kind {
        PanelHostBindingKind::DirectWindow => {
            PanelHostBinding::direct_window(id, window_id, document_id.clone(), container_id)
        }
        PanelHostBindingKind::SurfaceContainer => {
            PanelHostBinding::surface_container(id, window_id, document_id.clone(), container_id)
        }
    };
    PanelHostBindings::new([
        make(
            binding_id(SOURCE_BINDING),
            window(SOURCE_WINDOW),
            source_container(),
        ),
        make(
            binding_id(TARGET_BINDING),
            window(TARGET_WINDOW),
            target_container(),
        ),
    ])
    .unwrap()
}

pub fn window(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn client(value: &str) -> TransferClientId {
    TransferClientId::new(value).unwrap()
}

pub fn binding_id(value: &str) -> TransferHostBindingId {
    TransferHostBindingId::new(value).unwrap()
}

pub fn source_bounds() -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(0, 0), ScreenSize::new(800, 600))
}

pub fn target_bounds() -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(800, 0), ScreenSize::new(800, 600))
}
