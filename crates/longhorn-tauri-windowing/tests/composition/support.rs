use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc,
};

use longhorn_core::{
    LiveWindowMetrics, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_tauri_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, DesktopObservation,
    ManagedDesktopReadback, ManagedWindowRegistry, NativeWindowCall, NativeWindowMutationError,
    ScheduledWindowLifecycleWake, TauriObservationError, TauriWindowLifecycleServices,
    UniformWindowGeometryMapper, WindowCaptureBackend, WindowLifecycleClock, WindowLifecycleReport,
    WindowLifecycleReporter, WindowLifecycleScheduler, WindowMutationBackend,
    WindowPlacementFlushCompletion, WindowPlacementFlushTicket, WindowPlacementSink,
    WindowRevealBackend, WindowUserCloseHandler,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, HostWindowHandle, LiveWindow,
    MonotonicMillis, WindowDiffInput, WindowLifecycleDuration, WindowLifecyclePolicy,
};
use tauri::{AppHandle, Runtime, WebviewWindow, test::MockRuntime};

pub(super) fn id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub(super) fn handle(value: &str) -> HostWindowHandle {
    HostWindowHandle::new(value).unwrap()
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn desired(window_id: &str, placement: WindowPlacement, visible: bool) -> DesiredWindow {
    DesiredWindow::new(id(window_id), placement, false, visible)
}

pub(super) fn live(
    window_id: &str,
    transport_handle: &str,
    placement: WindowPlacement,
    visible: bool,
) -> LiveWindow {
    LiveWindow::new(
        Some(id(window_id)),
        handle(transport_handle),
        LiveWindowMetrics::new(
            ScreenRect::new(
                placement.outer_origin(),
                ScreenSize::new(
                    placement.inner_size().width() + 20,
                    placement.inner_size().height() + 40,
                ),
            ),
            placement.inner_size(),
        ),
        false,
        visible,
        false,
    )
}

pub(super) fn input(
    desired_windows: impl IntoIterator<Item = DesiredWindow>,
    live_windows: impl IntoIterator<Item = LiveWindow>,
    generation: u64,
) -> WindowDiffInput {
    WindowDiffInput::new(
        desired_windows,
        live_windows,
        HostCapabilities::none(),
        ApplyGeneration::new(generation),
    )
}

pub(super) fn policy(flush_millis: u64) -> WindowLifecyclePolicy {
    WindowLifecyclePolicy::new(
        WindowLifecycleDuration::from_millis(100),
        WindowLifecycleDuration::from_millis(100),
        WindowLifecycleDuration::from_millis(300),
        WindowLifecycleDuration::from_millis(400),
        WindowLifecycleDuration::from_millis(flush_millis),
    )
}

#[derive(Default)]
struct TestClock(AtomicU64);

impl WindowLifecycleClock for TestClock {
    fn now(&self) -> MonotonicMillis {
        MonotonicMillis::new(self.0.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
struct TestScheduler;

impl WindowLifecycleScheduler for TestScheduler {
    fn schedule(&self, _wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct TestCapture;

impl WindowCaptureBackend<MockRuntime> for TestCapture {
    fn capture(
        &self,
        window_id: &WindowId,
        _window: &WebviewWindow<MockRuntime>,
        retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, String> {
        Ok(CapturedWindowPlacement::new(
            window_id.clone(),
            retained_normal.unwrap_or_else(|| placement(20, 30, 800, 600)),
            false,
            CapturedDisplayAssociation::Unresolved,
        ))
    }
}

#[derive(Clone)]
pub(super) enum SinkMode {
    Succeed,
    Fail(String),
    RequestFail(String),
}

pub(super) struct TestSink {
    mode: SinkMode,
    staged: Mutex<Vec<CapturedWindowPlacement>>,
    requests: Mutex<Vec<longhorn_tauri_windowing::WindowFlushRequest>>,
}

impl TestSink {
    fn new(mode: SinkMode) -> Self {
        Self {
            mode,
            staged: Mutex::default(),
            requests: Mutex::default(),
        }
    }

    pub(super) fn staged_count(&self) -> usize {
        self.staged.lock().unwrap().len()
    }

    pub(super) fn requests(&self) -> Vec<longhorn_tauri_windowing::WindowFlushRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl WindowPlacementSink for TestSink {
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.staged.lock().unwrap().push(placement.clone());
        Ok(())
    }

    fn request_flush(
        &self,
        request: &longhorn_tauri_windowing::WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        self.requests.lock().unwrap().push(request.clone());
        if let SinkMode::RequestFail(detail) = &self.mode {
            return Err(detail.clone());
        }
        let (sender, receiver) = mpsc::channel();
        let completion = match &self.mode {
            SinkMode::Succeed => WindowPlacementFlushCompletion::Succeeded,
            SinkMode::Fail(detail) => WindowPlacementFlushCompletion::Failed(detail.clone()),
            SinkMode::RequestFail(_) => unreachable!(),
        };
        sender.send(completion).unwrap();
        Ok(WindowPlacementFlushTicket::new(receiver))
    }
}

#[derive(Default)]
pub(super) struct TestUserClose(pub(super) AtomicUsize);

impl WindowUserCloseHandler for TestUserClose {
    fn user_close(&self, _window_id: &WindowId) -> Result<(), String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Default)]
struct TestReporter;

impl WindowLifecycleReporter for TestReporter {
    fn report(&self, _report: WindowLifecycleReport) {}
}

#[derive(Default)]
pub(super) struct TestReveal(pub(super) AtomicUsize);

impl WindowRevealBackend<MockRuntime> for TestReveal {
    fn reveal(&self, _window: &WebviewWindow<MockRuntime>) -> Result<(), String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub(super) struct ServiceFixture {
    pub(super) services: TauriWindowLifecycleServices<MockRuntime>,
    pub(super) sink: Arc<TestSink>,
    pub(super) user_close: Arc<TestUserClose>,
    pub(super) reveal: Arc<TestReveal>,
}

pub(super) fn services(mode: SinkMode) -> ServiceFixture {
    let sink = Arc::new(TestSink::new(mode));
    let user_close = Arc::new(TestUserClose::default());
    let reveal = Arc::new(TestReveal::default());
    ServiceFixture {
        services: TauriWindowLifecycleServices::new(
            Arc::new(TestClock::default()),
            Arc::new(TestScheduler),
            Arc::new(UniformWindowGeometryMapper::new(
                ScaleFactor::from_thousandths(1000).unwrap(),
            )),
            Arc::new(TestCapture),
            sink.clone(),
            user_close.clone(),
            Arc::new(TestReporter),
            reveal.clone(),
        ),
        sink,
        user_close,
        reveal,
    }
}

#[derive(Clone)]
pub(super) struct StaticReadback {
    result: Result<DesktopObservation, TauriObservationError>,
}

impl StaticReadback {
    pub(super) fn complete(windows: impl IntoIterator<Item = LiveWindow>) -> Self {
        Self {
            result: Ok(DesktopObservation::new(
                Vec::new(),
                windows.into_iter().collect(),
            )),
        }
    }

    pub(super) fn failed(error: TauriObservationError) -> Self {
        Self { result: Err(error) }
    }
}

impl ManagedDesktopReadback<MockRuntime> for StaticReadback {
    fn readback(
        &mut self,
        _app: &AppHandle<MockRuntime>,
        _registry: &ManagedWindowRegistry<MockRuntime>,
    ) -> Result<DesktopObservation, TauriObservationError> {
        self.result.clone()
    }
}

#[derive(Clone, Default)]
pub(super) struct RecordingBackend {
    calls: Arc<Mutex<Vec<NativeWindowCall>>>,
    fail_on: Option<NativeWindowCall>,
}

impl RecordingBackend {
    pub(super) fn failing(call: NativeWindowCall) -> Self {
        Self {
            calls: Arc::default(),
            fail_on: Some(call),
        }
    }

    fn call(&self, call: NativeWindowCall) -> Result<(), NativeWindowMutationError> {
        self.calls.lock().unwrap().push(call);
        if self.fail_on == Some(call) {
            Err(NativeWindowMutationError::new("injected native failure"))
        } else {
            Ok(())
        }
    }
}

impl<R: Runtime> WindowMutationBackend<R> for RecordingBackend {
    fn unmaximize(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Unmaximize)
    }

    fn set_outer_position(
        &mut self,
        _: &WebviewWindow<R>,
        _: ScreenPoint,
    ) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::SetOuterPosition)
    }

    fn set_inner_size(
        &mut self,
        _: &WebviewWindow<R>,
        _: ScreenSize,
    ) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::SetInnerSize)
    }

    fn maximize(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Maximize)
    }

    fn show(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Show)
    }

    fn hide(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Hide)
    }

    fn focus(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Focus)
    }

    fn close(&mut self, _: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Close)
    }
}
