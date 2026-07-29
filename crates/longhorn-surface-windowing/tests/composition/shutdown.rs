use std::{cell::RefCell, fmt};

use longhorn_surface_windowing::{SurfaceWindowShutdownError, shutdown_surface_window_host};
use longhorn_windowing::WindowLifecycleDuration;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Failure(&'static str);

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for Failure {}

#[test]
fn surface_flush_precedes_window_shutdown_and_receipts_are_complete() {
    let order = RefCell::new(Vec::new());
    let receipt = shutdown_surface_window_host(
        WindowLifecycleDuration::from_millis(750),
        |timeout| {
            order.borrow_mut().push("surface");
            assert_eq!(timeout.as_millis(), 750);
            Ok::<_, Failure>(17_u64)
        },
        || {
            order.borrow_mut().push("window");
            Ok::<_, Failure>(3_usize)
        },
    )
    .unwrap();
    assert_eq!(order.into_inner(), ["surface", "window"]);
    assert_eq!(*receipt.surface(), 17);
    assert_eq!(*receipt.window(), 3);
}

#[test]
fn surface_failure_blocks_teardown_and_window_failure_keeps_flush_evidence() {
    let window_called = RefCell::new(false);
    let surface_error = shutdown_surface_window_host(
        WindowLifecycleDuration::from_millis(500),
        |_| Err::<u64, _>(Failure("flush failed")),
        || {
            *window_called.borrow_mut() = true;
            Ok::<_, Failure>(())
        },
    )
    .unwrap_err();
    assert_eq!(
        surface_error,
        SurfaceWindowShutdownError::Surface(Failure("flush failed"))
    );
    assert!(!window_called.into_inner());

    let window_error = shutdown_surface_window_host(
        WindowLifecycleDuration::from_millis(500),
        |_| Ok::<_, Failure>(23_u64),
        || Err::<(), _>(Failure("teardown failed")),
    )
    .unwrap_err();
    assert_eq!(
        window_error,
        SurfaceWindowShutdownError::Window {
            surface: 23,
            source: Failure("teardown failed"),
        }
    );
}
