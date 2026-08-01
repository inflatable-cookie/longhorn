use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use longhorn_core::{PhysicalRect, ScaleFactor};
use longhorn_native_content_backing_surface_prototype::{
    BackingSurfaceError, BackingSurfaceRuntime, DetachOutcome, RuntimeAttachRequest,
    RuntimeSnapshot,
};
use longhorn_native_content_prototype::{DetachPolicy, InputRoutingMode};
use tauri::{AppHandle, Manager, WebviewWindow, Wry};

use crate::{
    deterministic_renderer::{DeterministicRenderer, PixelEvidence},
    native_macos::{self, NativeToken},
};

const MAIN_THREAD_WAIT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct TauriBackingRuntime {
    app: AppHandle<Wry>,
    window_label: String,
    detach_policy: DetachPolicy,
    state: Arc<Mutex<RuntimeState>>,
}

#[derive(Default)]
struct RuntimeState {
    next_handle: u64,
    sessions: BTreeMap<u64, Session>,
}

struct Session {
    native: NativeToken,
    snapshot: RuntimeSnapshot,
    renderer: DeterministicRenderer,
    pixels: PixelEvidence,
}

impl TauriBackingRuntime {
    pub(crate) fn new(
        app: AppHandle<Wry>,
        window_label: impl Into<String>,
        detach_policy: DetachPolicy,
    ) -> Self {
        Self {
            app,
            window_label: window_label.into(),
            detach_policy,
            state: Arc::new(Mutex::new(RuntimeState::default())),
        }
    }

    pub(crate) fn pixels(&self, handle: u64) -> Result<PixelEvidence, BackingSurfaceError> {
        self.state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?
            .sessions
            .get(&handle)
            .map(|session| session.pixels.clone())
            .ok_or(BackingSurfaceError::NotAttached)
    }

    pub(crate) fn snapshot(&self, handle: u64) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        self.state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?
            .sessions
            .get(&handle)
            .map(|session| session.snapshot.clone())
            .ok_or(BackingSurfaceError::NotAttached)
    }

    pub(crate) fn only_handle(&self) -> Result<u64, BackingSurfaceError> {
        let state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        if state.sessions.len() != 1 {
            return Err(BackingSurfaceError::Runtime {
                operation: "proof-handle",
                detail: format!("expected one session, found {}", state.sessions.len()),
            });
        }
        state
            .sessions
            .keys()
            .next()
            .copied()
            .ok_or(BackingSurfaceError::NotAttached)
    }

    fn window(&self) -> Result<WebviewWindow<Wry>, BackingSurfaceError> {
        self.app
            .get_webview_window(&self.window_label)
            .ok_or_else(|| BackingSurfaceError::Runtime {
                operation: "window",
                detail: format!("webview window {} is missing", self.window_label),
            })
    }

    fn render(session: &mut Session) {
        let (sequence, pixels) = session.renderer.render(
            session.snapshot.storage_bounds,
            session.snapshot.clip,
            session.snapshot.presentation_enabled,
        );
        session.snapshot.frame_sequence = sequence;
        session.pixels = pixels;
    }

    fn native_state(
        &self,
        handle: u64,
    ) -> Result<(NativeToken, RuntimeSnapshot), BackingSurfaceError> {
        self.state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?
            .sessions
            .get(&handle)
            .map(|session| (session.native, session.snapshot.clone()))
            .ok_or(BackingSurfaceError::NotAttached)
    }
}

impl BackingSurfaceRuntime for TauriBackingRuntime {
    type Handle = u64;

    fn attach(
        &self,
        _request: RuntimeAttachRequest,
    ) -> Result<(Self::Handle, RuntimeSnapshot), BackingSurfaceError> {
        let window = self.window()?;
        let native_scale = window.scale_factor().map_err(native_error("scale"))?;
        let scale = scale_factor(native_scale)?;
        let (native, evidence) = on_main(window, move |window| {
            native_macos::attach(&window, native_scale)
        })?;
        let mut renderer = DeterministicRenderer::default();
        let initial_clip = PhysicalRect::new(
            longhorn_core::PhysicalPoint::new(0, 0),
            longhorn_core::PhysicalSize::new(0, 0),
        );
        let (sequence, pixels) = renderer.render(evidence.storage_bounds, initial_clip, false);
        let snapshot = RuntimeSnapshot {
            storage_bounds: evidence.storage_bounds,
            clip: initial_clip,
            presentation_enabled: false,
            input_routing: InputRoutingMode::Disabled,
            native_scale: scale,
            native_view_attached: evidence.attached,
            frame_sequence: sequence,
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        state.next_handle += 1;
        let handle = state.next_handle;
        state.sessions.insert(
            handle,
            Session {
                native,
                snapshot: snapshot.clone(),
                renderer,
                pixels,
            },
        );
        Ok((handle, snapshot))
    }

    fn set_viewport(
        &self,
        handle: &Self::Handle,
        clip: PhysicalRect,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let (native, current) = self.native_state(*handle)?;
        let window = self.window()?;
        let scale = window.scale_factor().map_err(native_error("scale"))?;
        on_main(window, move |_| {
            native_macos::set_clip(native, clip, scale, current.presentation_enabled)
        })?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let session = state
            .sessions
            .get_mut(handle)
            .ok_or(BackingSurfaceError::NotAttached)?;
        session.snapshot.clip = clip;
        session.snapshot.native_scale = scale_factor(scale)?;
        Self::render(session);
        Ok(session.snapshot.clone())
    }

    fn set_presentation_enabled(
        &self,
        handle: &Self::Handle,
        enabled: bool,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let (native, current) = self.native_state(*handle)?;
        let window = self.window()?;
        let scale = window.scale_factor().map_err(native_error("scale"))?;
        on_main(window, move |_| {
            native_macos::set_clip(native, current.clip, scale, enabled)
        })?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let session = state
            .sessions
            .get_mut(handle)
            .ok_or(BackingSurfaceError::NotAttached)?;
        session.snapshot.presentation_enabled = enabled;
        session.snapshot.native_scale = scale_factor(scale)?;
        Self::render(session);
        Ok(session.snapshot.clone())
    }

    fn set_input_routing(
        &self,
        handle: &Self::Handle,
        mode: InputRoutingMode,
    ) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let session = state
            .sessions
            .get_mut(handle)
            .ok_or(BackingSurfaceError::NotAttached)?;
        session.snapshot.input_routing = mode;
        Self::render(session);
        Ok(session.snapshot.clone())
    }

    fn refresh(&self, handle: &Self::Handle) -> Result<RuntimeSnapshot, BackingSurfaceError> {
        let (native, current) = self.native_state(*handle)?;
        let window = self.window()?;
        let scale = window.scale_factor().map_err(native_error("scale"))?;
        let evidence = on_main(window, move |window| {
            native_macos::refresh(
                &window,
                native,
                current.clip,
                scale,
                current.presentation_enabled,
            )
        })?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?;
        let session = state
            .sessions
            .get_mut(handle)
            .ok_or(BackingSurfaceError::NotAttached)?;
        session.snapshot.storage_bounds = evidence.storage_bounds;
        session.snapshot.native_view_attached = evidence.attached;
        session.snapshot.native_scale = scale_factor(scale)?;
        Self::render(session);
        Ok(session.snapshot.clone())
    }

    fn detach(&self, handle: &Self::Handle) -> Result<DetachOutcome, BackingSurfaceError> {
        let session = self
            .state
            .lock()
            .map_err(|_| BackingSurfaceError::Poisoned)?
            .sessions
            .remove(handle)
            .ok_or(BackingSurfaceError::NotAttached)?;
        let release = self.detach_policy == DetachPolicy::Reversible;
        let window = self.window()?;
        on_main(window, move |_| {
            native_macos::detach(session.native, release)
        })?;
        Ok(if release {
            DetachOutcome::Detached
        } else {
            DetachOutcome::RetainedForProcessLifetime
        })
    }
}

fn on_main<T: Send + 'static>(
    window: WebviewWindow<Wry>,
    action: impl FnOnce(WebviewWindow<Wry>) -> Result<T, String> + Send + 'static,
) -> Result<T, BackingSurfaceError> {
    let runner = window.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    runner
        .run_on_main_thread(move || {
            let _ = sender.send(action(window));
        })
        .map_err(native_error("main-thread-dispatch"))?;
    receiver
        .recv_timeout(MAIN_THREAD_WAIT)
        .map_err(|error| BackingSurfaceError::Runtime {
            operation: "main-thread-wait",
            detail: error.to_string(),
        })?
        .map_err(|detail| BackingSurfaceError::Runtime {
            operation: "app-kit",
            detail,
        })
}

fn scale_factor(value: f64) -> Result<ScaleFactor, BackingSurfaceError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(BackingSurfaceError::Runtime {
            operation: "scale",
            detail: format!("invalid native scale {value}"),
        });
    }
    let thousandths = (value * 1_000.0).round();
    if thousandths > f64::from(u32::MAX) {
        return Err(BackingSurfaceError::Runtime {
            operation: "scale",
            detail: format!("native scale {value} exceeds model range"),
        });
    }
    ScaleFactor::from_thousandths(thousandths as u32).map_err(|error| {
        BackingSurfaceError::Runtime {
            operation: "scale",
            detail: error.to_string(),
        }
    })
}

fn native_error(operation: &'static str) -> impl FnOnce(tauri::Error) -> BackingSurfaceError {
    move |error| BackingSurfaceError::Runtime {
        operation,
        detail: error.to_string(),
    }
}
