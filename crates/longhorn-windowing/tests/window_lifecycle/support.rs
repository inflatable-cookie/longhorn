use longhorn_core::{ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_windowing::{
    ApplyGeneration, HostWindowHandle, MonotonicMillis, WindowLifecycleCoordinator,
    WindowLifecycleDirective, WindowLifecycleDuration, WindowLifecycleEvent, WindowLifecyclePolicy,
    WindowOperation,
};

pub(super) fn id() -> WindowId {
    WindowId::new("main").unwrap()
}

pub(super) const fn at(value: u64) -> MonotonicMillis {
    MonotonicMillis::new(value)
}

pub(super) const fn duration(value: u64) -> WindowLifecycleDuration {
    WindowLifecycleDuration::from_millis(value)
}

pub(super) const fn generation(value: u64) -> ApplyGeneration {
    ApplyGeneration::new(value)
}

pub(super) const fn donor_policy() -> WindowLifecyclePolicy {
    WindowLifecyclePolicy::new(
        duration(3_000),
        duration(5_000),
        duration(300),
        duration(250),
        duration(1_000),
    )
}

pub(super) fn coordinator() -> WindowLifecycleCoordinator {
    WindowLifecycleCoordinator::new(donor_policy())
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn move_resize(x: i32, y: i32, width: u32, height: u32) -> WindowOperation {
    WindowOperation::MoveResize {
        window_id: id(),
        transport_handle: None,
        placement: placement(x, y, width, height),
    }
}

pub(super) fn close() -> WindowOperation {
    WindowOperation::Close {
        window_id: id(),
        transport_handle: HostWindowHandle::new("main-host").unwrap(),
    }
}

pub(super) fn moved(x: i32, y: i32) -> WindowLifecycleEvent {
    WindowLifecycleEvent::Moved {
        window_id: id(),
        outer_origin: ScreenPoint::new(x, y),
    }
}

pub(super) fn resized(width: u32, height: u32) -> WindowLifecycleEvent {
    WindowLifecycleEvent::Resized {
        window_id: id(),
        inner_size: ScreenSize::new(width, height),
    }
}

pub(super) fn only(directives: Vec<WindowLifecycleDirective>) -> WindowLifecycleDirective {
    assert_eq!(directives.len(), 1);
    directives.into_iter().next().unwrap()
}
