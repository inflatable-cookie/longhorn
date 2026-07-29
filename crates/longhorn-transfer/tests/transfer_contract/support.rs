use std::{cell::Cell, collections::VecDeque};

use longhorn_core::{
    DomainId, LayoutContainerId, RegionId, ScreenPoint, ScreenRect, ScreenSize, WindowId,
};
use longhorn_transfer::{
    ClientEpoch, DragSessionIdAllocationError, DragSessionIdAllocator, DropZone, DropZoneId,
    InsertionPosition, LeaseGeneration, LeasePublication, LiveTransferWindow, MonotonicClock,
    TransferCapability, TransferClientId, TransferCoordinator, TransferDuration,
    TransferHostBindingId, TransferInstant, TransferLimits, TransferRevision,
    TransferSourceAuthority, TransferSubjectId, TransferTargetBinding,
};

pub struct FakeClock(Cell<u64>);

impl FakeClock {
    pub const fn new(now: u64) -> Self {
        Self(Cell::new(now))
    }

    pub fn set(&self, now: u64) {
        self.0.set(now);
    }
}

impl MonotonicClock for FakeClock {
    fn now(&self) -> TransferInstant {
        TransferInstant::new(self.0.get())
    }
}

pub struct SequenceAllocator {
    values: VecDeque<Result<[u8; 16], DragSessionIdAllocationError>>,
    calls: usize,
}

impl SequenceAllocator {
    pub fn new(values: impl IntoIterator<Item = [u8; 16]>) -> Self {
        Self {
            values: values.into_iter().map(Ok).collect(),
            calls: 0,
        }
    }

    pub fn failing() -> Self {
        Self {
            values: VecDeque::from([Err(DragSessionIdAllocationError)]),
            calls: 0,
        }
    }

    pub const fn calls(&self) -> usize {
        self.calls
    }
}

impl DragSessionIdAllocator for SequenceAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        self.calls += 1;
        self.values
            .pop_front()
            .unwrap_or(Err(DragSessionIdAllocationError))
    }
}

pub fn limits(sessions: usize, clients: usize, zones: usize, insertion: u32) -> TransferLimits {
    TransferLimits::new(
        sessions,
        clients,
        clients,
        zones,
        insertion,
        TransferDuration::new(100),
        TransferDuration::new(50),
    )
    .unwrap()
}

pub fn coordinator() -> TransferCoordinator {
    TransferCoordinator::new(limits(8, 8, 8, 20))
}

pub fn window(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub fn client(value: &str) -> TransferClientId {
    TransferClientId::new(value).unwrap()
}

pub fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub fn live(window_id: &str, bounds: ScreenRect) -> LiveTransferWindow {
    LiveTransferWindow::new(window(window_id), bounds)
}

pub fn bind(
    coordinator: &mut TransferCoordinator,
    clock: &FakeClock,
    window_id: &str,
    client_id: &str,
    epoch: u64,
) {
    coordinator
        .bind_client_epoch(
            clock,
            window(window_id),
            client(client_id),
            ClientEpoch::new(epoch),
        )
        .unwrap();
}

pub fn panel_source(window_id: &str, client_id: &str, epoch: u64) -> TransferSourceAuthority {
    TransferSourceAuthority::Panel {
        client_id: client(client_id),
        client_epoch: ClientEpoch::new(epoch),
        source_window_id: window(window_id),
        subject_id: TransferSubjectId::new("panel:inspector").unwrap(),
        host_binding_id: TransferHostBindingId::new("host:source").unwrap(),
        document_id: DomainId::new("layout.workspace").unwrap(),
        revision: TransferRevision::new(7),
        container_id: LayoutContainerId::new("container:source").unwrap(),
        region_id: RegionId::new("region:tools").unwrap(),
    }
}

pub fn surface_source(window_id: &str, client_id: &str, epoch: u64) -> TransferSourceAuthority {
    TransferSourceAuthority::Surface {
        client_id: client(client_id),
        client_epoch: ClientEpoch::new(epoch),
        source_window_id: window(window_id),
        subject_id: TransferSubjectId::new("surface:workspace").unwrap(),
        host_binding_id: TransferHostBindingId::new("host:source").unwrap(),
        document_id: DomainId::new("surface.workspace").unwrap(),
        revision: TransferRevision::new(11),
    }
}

pub fn panel_zone(id: &str, bounds: ScreenRect, insertion: Option<u32>) -> DropZone {
    DropZone::new(
        DropZoneId::new(id).unwrap(),
        bounds,
        insertion.map(InsertionPosition::new),
        TransferCapability::MovePanel,
        TransferTargetBinding::PanelRegion {
            host_binding_id: TransferHostBindingId::new("host:target").unwrap(),
            document_id: DomainId::new("layout.workspace").unwrap(),
            revision: TransferRevision::new(9),
            container_id: LayoutContainerId::new("container:target").unwrap(),
            region_id: RegionId::new("region:main").unwrap(),
        },
    )
}

pub fn surface_zone(id: &str, bounds: ScreenRect) -> DropZone {
    DropZone::new(
        DropZoneId::new(id).unwrap(),
        bounds,
        None,
        TransferCapability::MoveSurface,
        TransferTargetBinding::SurfaceWindow {
            host_binding_id: TransferHostBindingId::new("host:target").unwrap(),
            document_id: DomainId::new("surface.workspace").unwrap(),
            revision: TransferRevision::new(13),
        },
    )
}

pub fn publication(
    window_id: &str,
    client_id: &str,
    epoch: u64,
    generation: u64,
    bounds: ScreenRect,
    zones: Vec<DropZone>,
) -> LeasePublication {
    LeasePublication::new(
        window(window_id),
        client(client_id),
        ClientEpoch::new(epoch),
        LeaseGeneration::new(generation),
        TransferDuration::new(30),
        bounds,
        zones,
    )
}
