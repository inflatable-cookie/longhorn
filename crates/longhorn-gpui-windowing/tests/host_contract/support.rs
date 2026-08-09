use std::collections::BTreeMap;

use longhorn_core::{ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement};
use longhorn_display::DisplayBuiltinStatus;
use longhorn_gpui_windowing::{
    GpuiDisplayFacts, GpuiDisplayFactsSource, GpuiLogicalRect, GpuiLogicalSize, GpuiWindowBackend,
    GpuiWindowBoundsState, GpuiWindowCreateRequest, GpuiWindowError, GpuiWindowFacts,
    GpuiWindowKey,
};
use longhorn_windowing::{
    ApplyGeneration, DesiredWindow, HostCapabilities, HostWindowHandle, WindowDiffInput,
};

pub(super) fn id(value: &str) -> WindowId {
    WindowId::new(value).unwrap()
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

pub(super) fn plan(
    desired_windows: impl IntoIterator<Item = DesiredWindow>,
    generation: u64,
) -> WindowDiffInput {
    // Capabilities here are a placeholder: the apply engine replaces them with
    // what the backend can actually do, which is the whole point.
    WindowDiffInput::new(
        desired_windows,
        Vec::new(),
        HostCapabilities::all(),
        ApplyGeneration::new(generation),
    )
}

pub(super) fn scale(thousandths: u32) -> ScaleFactor {
    ScaleFactor::from_thousandths(thousandths).unwrap()
}

/// One recorded call against the fake GPUI host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Call {
    Create(WindowId),
    Resize(GpuiWindowKey),
    SetMaximized(GpuiWindowKey, bool),
    Activate(GpuiWindowKey),
    Close(GpuiWindowKey),
}

struct FakeWindow {
    bounds: GpuiLogicalRect,
    content_size: GpuiLogicalSize,
    maximized: bool,
    active: bool,
    scale: f32,
}

/// An in-memory stand-in for a live GPUI application.
///
/// It implements exactly what `PlatformWindow` offers, which is why there is
/// no `set_position` and no `show`. A fake with more surface than the real
/// host would prove nothing.
pub(super) struct FakeGpuiHost {
    windows: BTreeMap<u64, FakeWindow>,
    displays: Vec<GpuiDisplayFacts>,
    next_slot: u64,
    can_create: bool,
    pub(super) calls: Vec<Call>,
    pub(super) fail_next_create: bool,
    lagging_maximize: bool,
}

impl FakeGpuiHost {
    pub(super) fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            // One display, at the zeroed origin gpui actually reports.
            displays: vec![GpuiDisplayFacts::new(
                1,
                Some("6d2f0e5c-0000-4000-8000-000000000001".to_owned()),
                GpuiLogicalRect::new(0.0, 0.0, 1920.0, 1080.0),
                true,
            )],
            next_slot: 1,
            can_create: true,
            calls: Vec::new(),
            fail_next_create: false,
            lagging_maximize: false,
        }
    }

    /// Attaches a second display, reported the way gpui reports one: correct
    /// size, origin discarded. Both therefore claim `(0, 0)`.
    pub(super) fn with_second_display(mut self) -> Self {
        self.displays.push(GpuiDisplayFacts::new(
            2,
            Some("817fc161-2581-4167-83c8-732427e5ac07".to_owned()),
            GpuiLogicalRect::new(0.0, 0.0, 3440.0, 1440.0),
            false,
        ));
        self
    }

    pub(super) fn without_create(mut self) -> Self {
        self.can_create = false;
        self
    }

    /// Accepts a maximize and keeps reporting the old state, as macOS does
    /// while it animates the zoom.
    pub(super) fn with_lagging_maximize(mut self) -> Self {
        self.lagging_maximize = true;
        self
    }

    pub(super) fn with_existing_window(
        mut self,
        bounds: GpuiLogicalRect,
        content: GpuiLogicalSize,
        maximized: bool,
    ) -> (Self, GpuiWindowKey) {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.windows.insert(
            slot,
            FakeWindow {
                bounds,
                content_size: content,
                maximized,
                active: false,
                scale: 2.0,
            },
        );
        (self, GpuiWindowKey::new(slot))
    }

    pub(super) fn is_maximized(&self, key: GpuiWindowKey) -> bool {
        self.windows[&key.slot()].maximized
    }

    pub(super) fn is_open(&self, key: GpuiWindowKey) -> bool {
        self.windows.contains_key(&key.slot())
    }
}

impl GpuiWindowBackend for FakeGpuiHost {
    fn can_create(&self) -> bool {
        self.can_create
    }

    fn create(
        &mut self,
        window_id: &WindowId,
        request: &GpuiWindowCreateRequest,
    ) -> Result<GpuiWindowKey, GpuiWindowError> {
        self.calls.push(Call::Create(window_id.clone()));
        if self.fail_next_create {
            return Err(GpuiWindowError::new("open_window refused"));
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        let bounds = request.bounds();
        self.windows.insert(
            slot,
            FakeWindow {
                bounds,
                content_size: GpuiLogicalSize::new(
                    bounds.to_screen_size().unwrap().width() as f32,
                    bounds.to_screen_size().unwrap().height() as f32,
                ),
                maximized: request.is_maximized(),
                active: request.focuses_on_open(),
                scale: 2.0,
            },
        );
        Ok(GpuiWindowKey::new(slot))
    }

    fn resize(&mut self, key: GpuiWindowKey, size: GpuiLogicalSize) -> Result<(), GpuiWindowError> {
        self.calls.push(Call::Resize(key));
        let window = self
            .windows
            .get_mut(&key.slot())
            .ok_or_else(|| GpuiWindowError::new("no such window"))?;
        window.content_size = size;
        Ok(())
    }

    fn set_maximized(
        &mut self,
        key: GpuiWindowKey,
        maximized: bool,
    ) -> Result<(), GpuiWindowError> {
        self.calls.push(Call::SetMaximized(key, maximized));
        let lagging = self.lagging_maximize;
        let window = self
            .windows
            .get_mut(&key.slot())
            .ok_or_else(|| GpuiWindowError::new("no such window"))?;
        if lagging {
            // The call is accepted; the state is not observable yet.
            return Ok(());
        }
        // Mirrors the real implementation: gpui only has a toggle, so an
        // absolute request reads first and acts only on disagreement.
        if window.maximized != maximized {
            window.maximized = maximized;
        }
        Ok(())
    }

    fn activate(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError> {
        self.calls.push(Call::Activate(key));
        for (slot, window) in &mut self.windows {
            window.active = *slot == key.slot();
        }
        Ok(())
    }

    fn close(&mut self, key: GpuiWindowKey) -> Result<(), GpuiWindowError> {
        self.calls.push(Call::Close(key));
        self.windows
            .remove(&key.slot())
            .map(|_| ())
            .ok_or_else(|| GpuiWindowError::new("no such window"))
    }

    fn observe(&mut self, key: GpuiWindowKey) -> Result<GpuiWindowFacts, GpuiWindowError> {
        let window = self
            .windows
            .get(&key.slot())
            .ok_or_else(|| GpuiWindowError::new("no such window"))?;
        let state = if window.maximized {
            GpuiWindowBoundsState::Maximized(window.bounds)
        } else {
            GpuiWindowBoundsState::Windowed(window.bounds)
        };
        Ok(GpuiWindowFacts::new(
            window.bounds,
            window.content_size,
            state,
            window.scale,
            window.active,
        ))
    }

    fn displays(&mut self) -> Result<Vec<GpuiDisplayFacts>, GpuiWindowError> {
        Ok(self.displays.clone())
    }
}

/// A display facts source that knows nothing GPUI does not.
pub(super) struct BareDisplayFacts;

impl GpuiDisplayFactsSource for BareDisplayFacts {
    fn scale_factor(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScaleFactor> {
        None
    }

    fn work_area(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScreenRect> {
        None
    }
}

/// A display facts source a product supplies from outside GPUI.
pub(super) struct SuppliedDisplayFacts {
    pub(super) scale: ScaleFactor,
    pub(super) work_area: ScreenRect,
}

impl SuppliedDisplayFacts {
    pub(super) fn new() -> Self {
        Self {
            scale: scale(2000),
            work_area: ScreenRect::new(ScreenPoint::new(0, 25), ScreenSize::new(1920, 1055)),
        }
    }
}

impl GpuiDisplayFactsSource for SuppliedDisplayFacts {
    fn scale_factor(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScaleFactor> {
        Some(self.scale)
    }

    fn position(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScreenPoint> {
        Some(ScreenPoint::new(0, 0))
    }

    fn work_area(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScreenRect> {
        Some(self.work_area)
    }

    fn builtin_status(&mut self, _facts: &GpuiDisplayFacts) -> DisplayBuiltinStatus {
        DisplayBuiltinStatus::BuiltIn
    }
}

pub(super) fn handle_of(key: GpuiWindowKey) -> HostWindowHandle {
    key.transport_handle()
}
