use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
};

use longhorn_core::{ScaleFactor, ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_tauri_windowing::{
    CapturedDisplayAssociation, CapturedWindowPlacement, ScheduledWindowLifecycleWake,
    TauriWindowLifecycleHost, TauriWindowLifecycleServices, UniformWindowGeometryMapper,
    WindowCaptureBackend, WindowLifecycleClock, WindowLifecycleReport, WindowLifecycleReporter,
    WindowLifecycleScheduler, WindowPlacementFlushCompletion, WindowPlacementFlushTicket,
    WindowPlacementSink, WindowRevealBackend, WindowUserCloseHandler,
};
use longhorn_windowing::{MonotonicMillis, WindowLifecycleDuration, WindowLifecyclePolicy};
use tauri::{App, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

pub(super) fn id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
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
pub(super) struct TestClock(AtomicU64);

impl TestClock {
    pub(super) fn set(&self, value: u64) {
        self.0.store(value, Ordering::SeqCst);
    }
}

impl WindowLifecycleClock for TestClock {
    fn now(&self) -> MonotonicMillis {
        MonotonicMillis::new(self.0.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
pub(super) struct TestScheduler(Mutex<Vec<ScheduledWindowLifecycleWake>>);

impl TestScheduler {
    pub(super) fn wakes(&self) -> Vec<ScheduledWindowLifecycleWake> {
        self.0.lock().unwrap().clone()
    }
}

impl WindowLifecycleScheduler for TestScheduler {
    fn schedule(&self, wake: ScheduledWindowLifecycleWake) -> Result<(), String> {
        self.0.lock().unwrap().push(wake);
        Ok(())
    }
}

pub(super) struct TestCapture {
    outcomes: Mutex<VecDeque<Result<CapturedWindowPlacement, String>>>,
}

impl TestCapture {
    pub(super) fn repeating(window_id: &WindowId) -> Self {
        Self {
            outcomes: Mutex::new(VecDeque::from([Ok(CapturedWindowPlacement::new(
                window_id.clone(),
                placement(20, 30, 800, 600),
                false,
                CapturedDisplayAssociation::Unresolved,
            ))])),
        }
    }

    pub(super) fn outcomes(
        outcomes: impl IntoIterator<Item = Result<CapturedWindowPlacement, String>>,
    ) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        }
    }
}

impl WindowCaptureBackend<MockRuntime> for TestCapture {
    fn capture(
        &self,
        _window_id: &WindowId,
        _window: &WebviewWindow<MockRuntime>,
        _retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, String> {
        let mut outcomes = self.outcomes.lock().unwrap();
        let outcome = outcomes
            .pop_front()
            .expect("test capture outcome is available");
        if outcomes.is_empty() && outcome.is_ok() {
            outcomes.push_back(outcome.clone());
        }
        outcome
    }
}

#[derive(Clone)]
pub(super) enum FlushMode {
    Succeed,
    Fail(String),
    Disconnect,
    Timeout,
}

pub(super) struct TestSink {
    pub(super) staged: Mutex<Vec<CapturedWindowPlacement>>,
    requests: Mutex<Vec<longhorn_tauri_windowing::WindowFlushRequest>>,
    mode: Mutex<FlushMode>,
    held: Mutex<Vec<mpsc::Sender<WindowPlacementFlushCompletion>>>,
    stage_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl TestSink {
    pub(super) fn new(mode: FlushMode) -> Self {
        Self {
            staged: Mutex::default(),
            requests: Mutex::default(),
            mode: Mutex::new(mode),
            held: Mutex::default(),
            stage_hook: Mutex::default(),
        }
    }

    pub(super) fn requests(&self) -> Vec<longhorn_tauri_windowing::WindowFlushRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub(super) fn set_stage_hook(&self, hook: Arc<dyn Fn() + Send + Sync>) {
        *self.stage_hook.lock().unwrap() = Some(hook);
    }
}

impl WindowPlacementSink for TestSink {
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.staged.lock().unwrap().push(placement.clone());
        let hook = self.stage_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }

    fn request_flush(
        &self,
        request: &longhorn_tauri_windowing::WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        self.requests.lock().unwrap().push(request.clone());
        let (sender, receiver) = mpsc::channel();
        match self.mode.lock().unwrap().clone() {
            FlushMode::Succeed => sender
                .send(WindowPlacementFlushCompletion::Succeeded)
                .unwrap(),
            FlushMode::Fail(detail) => sender
                .send(WindowPlacementFlushCompletion::Failed(detail))
                .unwrap(),
            FlushMode::Disconnect => drop(sender),
            FlushMode::Timeout => self.held.lock().unwrap().push(sender),
        }
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
pub(super) struct TestReporter(Mutex<Vec<WindowLifecycleReport>>);

impl WindowLifecycleReporter for TestReporter {
    fn report(&self, report: WindowLifecycleReport) {
        self.0.lock().unwrap().push(report);
    }
}

#[derive(Default)]
pub(super) struct TestReveal(pub(super) AtomicUsize);

impl WindowRevealBackend<MockRuntime> for TestReveal {
    fn reveal(&self, _window: &WebviewWindow<MockRuntime>) -> Result<(), String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub(super) struct Harness {
    pub(super) _app: App<MockRuntime>,
    pub(super) host: Arc<TauriWindowLifecycleHost<MockRuntime>>,
    pub(super) clock: Arc<TestClock>,
    pub(super) scheduler: Arc<TestScheduler>,
    pub(super) sink: Arc<TestSink>,
    pub(super) user_close: Arc<TestUserClose>,
    pub(super) window_id: WindowId,
}

pub(super) fn harness(
    name: &str,
    flush_millis: u64,
    capture: Arc<dyn WindowCaptureBackend<MockRuntime>>,
    sink: Arc<TestSink>,
) -> Harness {
    let app = tauri::test::mock_app();
    let window = WebviewWindowBuilder::new(&app, name, Default::default())
        .build()
        .unwrap();
    let window_id = id(&format!("window:{name}"));
    let clock = Arc::new(TestClock::default());
    let scheduler = Arc::new(TestScheduler::default());
    let user_close = Arc::new(TestUserClose::default());
    let reveal = Arc::new(TestReveal::default());
    let mapper = Arc::new(UniformWindowGeometryMapper::new(
        ScaleFactor::from_thousandths(1000).unwrap(),
    ));
    let host = Arc::new(TauriWindowLifecycleHost::new(
        policy(flush_millis),
        TauriWindowLifecycleServices::new(
            clock.clone(),
            scheduler.clone(),
            mapper,
            capture,
            sink.clone(),
            user_close.clone(),
            Arc::new(TestReporter::default()),
            reveal,
        ),
    ));
    host.install_window(window_id.clone(), window, None)
        .unwrap();
    Harness {
        _app: app,
        host,
        clock,
        scheduler,
        sink,
        user_close,
        window_id,
    }
}
