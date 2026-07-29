use std::sync::{Arc, atomic::Ordering};

use longhorn_core::{
    LiveWindowMetrics, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_tauri_windowing::{
    DesktopObservation, ManagedDesktopReadback, ManagedWebviewWindow, ManagedWindowRegistry,
    NativeWindowMutationError, NoWindowFactory, TauriObservationError, TauriWindowLifecycleHost,
    TauriWindowLifecycleServices, UniformWindowGeometryMapper, WindowMutationBackend,
    WindowRevealStatus, execute_tauri_window_apply,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, HostWindowHandle, LiveWindow, WindowDiffInput,
};
use tauri::{AppHandle, Runtime, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

use super::support::{
    FlushMode, TestCapture, TestClock, TestReporter, TestReveal, TestScheduler, TestSink,
    TestUserClose, id, placement, policy,
};

#[derive(Default)]
struct NativeSuccess;

impl<R: Runtime> WindowMutationBackend<R> for NativeSuccess {
    fn unmaximize(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn set_outer_position(
        &mut self,
        _window: &WebviewWindow<R>,
        _position: ScreenPoint,
    ) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn set_inner_size(
        &mut self,
        _window: &WebviewWindow<R>,
        _size: ScreenSize,
    ) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn maximize(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn show(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn hide(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn focus(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }

    fn close(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        Ok(())
    }
}

struct StaticReadback(DesktopObservation);

impl ManagedDesktopReadback<MockRuntime> for StaticReadback {
    fn readback(
        &mut self,
        _app: &AppHandle<MockRuntime>,
        _registry: &ManagedWindowRegistry<MockRuntime>,
    ) -> Result<DesktopObservation, TauriObservationError> {
        Ok(self.0.clone())
    }
}

fn live(window_id: WindowId, placement: WindowPlacement) -> LiveWindow {
    LiveWindow::new(
        Some(window_id),
        HostWindowHandle::new("reveal").unwrap(),
        LiveWindowMetrics::new(
            ScreenRect::new(placement.outer_origin(), placement.inner_size()),
            placement.inner_size(),
        ),
        false,
        false,
        false,
    )
}

#[test]
fn reveal_waits_for_page_ready_and_converged_hidden_placement() {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, "reveal", Default::default())
        .build()
        .unwrap();
    let window_id = id("window:reveal");
    let sink = Arc::new(TestSink::new(FlushMode::Succeed));
    let reveal = Arc::new(TestReveal::default());
    let host = Arc::new(TauriWindowLifecycleHost::new(
        policy(1_000),
        TauriWindowLifecycleServices::new(
            Arc::new(TestClock::default()),
            Arc::new(TestScheduler::default()),
            Arc::new(UniformWindowGeometryMapper::new(
                ScaleFactor::from_thousandths(1000).unwrap(),
            )),
            Arc::new(TestCapture::repeating(&window_id)),
            sink,
            Arc::new(TestUserClose::default()),
            Arc::new(TestReporter::default()),
            reveal.clone(),
        ),
    ));
    host.install_window(window_id.clone(), window.clone(), None)
        .unwrap();
    let waiting = host.mark_page_ready(&window_id).unwrap();
    assert!(matches!(
        waiting.status(),
        WindowRevealStatus::Waiting {
            page_ready: true,
            placement_ready: false
        }
    ));

    let target = placement(50, 60, 800, 600);
    let desired = DesiredWindow::new(window_id.clone(), target, false, false);
    let input = WindowDiffInput::new(
        [desired],
        [live(window_id.clone(), target)],
        HostCapabilities::none(),
        ApplyGeneration::new(3),
    )
    .for_hidden_restore();
    let registry = ManagedWindowRegistry::new(
        [ManagedWebviewWindow::new(Some(window_id.clone()), window)],
        None,
    )
    .unwrap()
    .with_apply_observer(host.clone());
    let outcome = execute_tauri_window_apply(
        app.handle(),
        input,
        registry,
        NoWindowFactory,
        NativeSuccess,
        StaticReadback(DesktopObservation::new(
            Vec::new(),
            vec![live(window_id, target)],
        )),
    )
    .unwrap();
    let receipts = host.mark_apply_converged(outcome.receipt()).unwrap();

    assert!(matches!(receipts[0].status(), WindowRevealStatus::Revealed));
    assert_eq!(reveal.0.load(Ordering::SeqCst), 1);
}
