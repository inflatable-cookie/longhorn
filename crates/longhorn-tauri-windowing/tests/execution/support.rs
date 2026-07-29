use std::sync::{Arc, Mutex};

use longhorn_core::{
    LiveWindowMetrics, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement,
};
use longhorn_tauri_windowing::{
    DesktopObservation, ManagedDesktopReadback, ManagedWebviewWindow, ManagedWindowRegistry,
    NativeWindowCall, NativeWindowMutationError, TauriObservationError, WindowMutationBackend,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, HostWindowHandle, LiveWindow, WindowDiffInput,
};
use tauri::{App, AppHandle, Runtime, WebviewWindow, WebviewWindowBuilder, test::MockRuntime};

pub(super) fn id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
}

pub(super) fn handle(value: &str) -> HostWindowHandle {
    HostWindowHandle::new(value).unwrap()
}

pub(super) fn placement(x: i32, y: i32, width: u32, height: u32) -> WindowPlacement {
    WindowPlacement::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn desired(
    window_id: &str,
    placement: WindowPlacement,
    maximized: bool,
    visible: bool,
) -> DesiredWindow {
    DesiredWindow::new(id(window_id), placement, maximized, visible)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live(
    window_id: Option<&str>,
    transport_handle: &str,
    x: i32,
    y: i32,
    inner_width: u32,
    inner_height: u32,
    maximized: bool,
    visible: bool,
    focused: bool,
) -> LiveWindow {
    LiveWindow::new(
        window_id.map(id),
        handle(transport_handle),
        LiveWindowMetrics::new(
            ScreenRect::new(
                ScreenPoint::new(x, y),
                ScreenSize::new(inner_width + 20, inner_height + 40),
            ),
            ScreenSize::new(inner_width, inner_height),
        ),
        maximized,
        visible,
        focused,
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

pub(super) fn mock_window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, label, Default::default())
        .build()
        .unwrap()
}

pub(super) fn registry(
    windows: impl IntoIterator<Item = (Option<&'static str>, &'static str)>,
    protected: Option<&str>,
) -> (App<MockRuntime>, ManagedWindowRegistry<MockRuntime>) {
    let app = tauri::test::mock_app();
    let managed = windows
        .into_iter()
        .map(|(window_id, label)| {
            ManagedWebviewWindow::new(window_id.map(id), mock_window(&app, label))
        })
        .collect::<Vec<_>>();
    let registry =
        ManagedWindowRegistry::new(managed, protected.map(handle)).expect("valid registry");
    (app, registry)
}

#[derive(Clone)]
pub(super) struct StaticReadback {
    observation: DesktopObservation,
}

impl StaticReadback {
    pub(super) fn new(windows: impl IntoIterator<Item = LiveWindow>) -> Self {
        Self {
            observation: DesktopObservation::new(Vec::new(), windows.into_iter().collect()),
        }
    }
}

impl ManagedDesktopReadback<MockRuntime> for StaticReadback {
    fn readback(
        &mut self,
        _app: &AppHandle<MockRuntime>,
        _registry: &ManagedWindowRegistry<MockRuntime>,
    ) -> Result<DesktopObservation, TauriObservationError> {
        Ok(self.observation.clone())
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

    pub(super) fn calls(&self) -> Vec<NativeWindowCall> {
        self.calls.lock().unwrap().clone()
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
    fn unmaximize(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Unmaximize)
    }

    fn set_outer_position(
        &mut self,
        _window: &WebviewWindow<R>,
        _origin: ScreenPoint,
    ) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::SetOuterPosition)
    }

    fn set_inner_size(
        &mut self,
        _window: &WebviewWindow<R>,
        _size: ScreenSize,
    ) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::SetInnerSize)
    }

    fn maximize(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Maximize)
    }

    fn show(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Show)
    }

    fn hide(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Hide)
    }

    fn focus(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Focus)
    }

    fn close(&mut self, _window: &WebviewWindow<R>) -> Result<(), NativeWindowMutationError> {
        self.call(NativeWindowCall::Close)
    }
}
