use std::{error::Error, fmt, sync::Arc};

use longhorn_core::{
    PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode, ScaleFactor, ScreenPoint, ScreenSize,
    WindowId, WindowPlacement,
};
use longhorn_windowing::{
    CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement,
};
use tauri::{Runtime, WebviewWindow, WindowEvent};

use super::WindowCaptureBackend;
use crate::{
    DefaultDisplayMetadata, DesktopCoordinateMapper, DesktopMappingError, HostProbeOperation,
    LogicalLayoutMapper, ManagedWebviewWindow, MappedDesktopGeometry, PhysicalDesktopSnapshot,
    PhysicalLiveWindowObservation, PlatformDesktopMapper, ProbeTarget, TauriProbeError,
    mapping::{ensure_mapper_platform, map_desktop_by_scale},
    probe::{host_result, monitor_facts, probe_displays},
    probe_managed_windows,
};

/// Point and size conversion primitives for scale-based desktop mappers.
pub trait WindowGeometryMapper: DesktopCoordinateMapper {
    /// Maps one physical outer-frame origin.
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, DesktopMappingError>;

    /// Maps one physical inner-content size.
    fn map_inner_size(
        &self,
        size: PhysicalSize,
        window_scale: ScaleFactor,
    ) -> Result<ScreenSize, DesktopMappingError>;
}

/// Scale-based mapper for a test or a host with an explicitly established plane.
#[derive(Clone, Copy, Debug)]
pub struct UniformWindowGeometryMapper {
    scale: ScaleFactor,
}

impl UniformWindowGeometryMapper {
    /// Declares the scale shared by the complete desktop plane.
    #[must_use]
    pub const fn new(scale: ScaleFactor) -> Self {
        Self { scale }
    }

    /// Declares an exact 1× physical/logical desktop.
    #[must_use]
    pub fn identity_scale() -> Self {
        Self::new(ScaleFactor::from_thousandths(1_000).expect("identity scale is valid"))
    }
}

impl DesktopCoordinateMapper for UniformWindowGeometryMapper {
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        map_desktop_by_scale(snapshot, |_| self.scale)
    }
}

impl WindowGeometryMapper for UniformWindowGeometryMapper {
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        _window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, DesktopMappingError> {
        Ok(self
            .scale
            .physical_point_to_screen(origin, RoundingMode::Nearest)?)
    }

    fn map_inner_size(
        &self,
        size: PhysicalSize,
        _window_scale: ScaleFactor,
    ) -> Result<ScreenSize, DesktopMappingError> {
        Ok(self
            .scale
            .physical_size_to_screen(size, RoundingMode::Nearest)?)
    }
}

/// Scale-based projection for hosts whose native facts derive from logical layout.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowScaleGeometryMapper;

impl DesktopCoordinateMapper for WindowScaleGeometryMapper {
    fn supports_current_platform(&self) -> bool {
        LogicalLayoutMapper.supports_current_platform()
    }

    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        LogicalLayoutMapper.map_desktop(snapshot)
    }
}

impl WindowGeometryMapper for WindowScaleGeometryMapper {
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, DesktopMappingError> {
        Ok(window_scale.physical_point_to_screen(origin, RoundingMode::Nearest)?)
    }

    fn map_inner_size(
        &self,
        size: PhysicalSize,
        window_scale: ScaleFactor,
    ) -> Result<ScreenSize, DesktopMappingError> {
        Ok(window_scale.physical_size_to_screen(size, RoundingMode::Nearest)?)
    }
}

/// Failure to produce a complete durable placement proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TauriWindowCaptureError {
    /// Native window or monitor facts were unavailable.
    Probe(TauriProbeError),
    /// The desktop coordinate plane could not be mapped.
    Mapping(DesktopMappingError),
    /// A maximized window had no known normal bounds.
    MissingNormalPlacement,
    /// An injected capture backend rejected the request.
    Provider {
        /// Backend diagnostic.
        detail: String,
    },
}

impl fmt::Display for TauriWindowCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Probe(error) => error.fmt(formatter),
            Self::Mapping(error) => error.fmt(formatter),
            Self::MissingNormalPlacement => {
                formatter.write_str("maximized capture has no retained normal placement")
            }
            Self::Provider { detail } => formatter.write_str(detail),
        }
    }
}

impl Error for TauriWindowCaptureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Probe(source) => Some(source),
            Self::Mapping(source) => Some(source),
            _ => None,
        }
    }
}

impl From<TauriProbeError> for TauriWindowCaptureError {
    fn from(error: TauriProbeError) -> Self {
        Self::Probe(error)
    }
}

impl From<DesktopMappingError> for TauriWindowCaptureError {
    fn from(error: DesktopMappingError) -> Self {
        Self::Mapping(error)
    }
}

/// Complete native capture in the same coordinate plane as desktop observation.
pub struct TauriWindowCaptureBackend {
    mapper: Arc<dyn DesktopCoordinateMapper + Send + Sync>,
}

impl TauriWindowCaptureBackend {
    /// Uses the platform desktop mapper for capture.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mapper: Arc::new(PlatformDesktopMapper::default()),
        }
    }

    /// Uses a nominated desktop mapper for a test or an undescribed host.
    ///
    /// # Errors
    /// The mapper must describe the current host.
    pub fn with_mapper(
        mapper: Arc<dyn DesktopCoordinateMapper + Send + Sync>,
    ) -> Result<Self, DesktopMappingError> {
        ensure_mapper_platform(mapper.as_ref())?;
        Ok(Self { mapper })
    }
}

impl Default for TauriWindowCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Runtime> WindowCaptureBackend<R> for TauriWindowCaptureBackend {
    fn capture(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
        retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, TauriWindowCaptureError> {
        let (snapshot, mapped) =
            map_lifecycle_window(window_id, window, None, self.mapper.as_ref())?;
        let raw = snapshot
            .windows()
            .first()
            .ok_or(DesktopMappingError::MissingLifecycleGeometry)?;
        let geometry = mapped
            .windows()
            .iter()
            .find(|geometry| geometry.transport_handle() == raw.transport_handle())
            .ok_or(DesktopMappingError::MissingLifecycleGeometry)?;
        let normal_placement = if raw.is_maximized() {
            retained_normal.ok_or(TauriWindowCaptureError::MissingNormalPlacement)?
        } else {
            WindowPlacement::new(geometry.outer_bounds().origin(), geometry.inner_size())
        };

        let monitor = host_result(
            window.current_monitor(),
            HostProbeOperation::CurrentMonitor,
            Some(raw.transport_handle()),
        )?;
        let display = if let Some(monitor) = monitor {
            let facts = monitor_facts(
                &monitor,
                ProbeTarget::Window(raw.transport_handle().clone()),
            )?;
            let raw_display = snapshot
                .displays()
                .iter()
                .find(|display| {
                    let observed = display.facts();
                    observed.machine_label() == facts.machine_label()
                        && observed.full_bounds() == facts.full_bounds()
                        && observed.scale() == facts.scale()
                })
                .ok_or(DesktopMappingError::MissingLifecycleGeometry)?;
            let display = mapped
                .displays()
                .iter()
                .find(|display| display.observation_id() == raw_display.metadata().observation_id())
                .ok_or(DesktopMappingError::MissingLifecycleGeometry)?;
            CapturedDisplayAssociation::Observed {
                evidence: CapturedDisplayEvidence::new(
                    raw_display.facts().machine_label().cloned(),
                    raw_display.facts().full_bounds(),
                    raw_display.facts().work_area(),
                    raw_display.facts().scale(),
                    display.full_bounds(),
                    display.work_area(),
                ),
            }
        } else {
            CapturedDisplayAssociation::Unresolved
        };

        Ok(CapturedWindowPlacement::new(
            window_id.clone(),
            normal_placement,
            raw.is_maximized(),
            display,
        ))
    }
}

pub(super) fn map_lifecycle_window<R: Runtime>(
    window_id: &WindowId,
    window: &WebviewWindow<R>,
    event: Option<&WindowEvent>,
    mapper: &(impl DesktopCoordinateMapper + ?Sized),
) -> Result<(PhysicalDesktopSnapshot, MappedDesktopGeometry), TauriWindowCaptureError> {
    ensure_mapper_platform(mapper)?;
    let displays = probe_displays(
        window.available_monitors(),
        || window.primary_monitor(),
        &mut DefaultDisplayMetadata,
    )?;
    let mut windows = probe_managed_windows(&[ManagedWebviewWindow::new(
        Some(window_id.clone()),
        window.clone(),
    )])?;
    let raw = windows
        .first_mut()
        .ok_or(DesktopMappingError::MissingLifecycleGeometry)?;
    let geometry = match event {
        Some(WindowEvent::Moved(position)) => Some((
            PhysicalRect::new(
                PhysicalPoint::new(position.x, position.y),
                raw.outer_bounds().size(),
            ),
            raw.inner_size(),
        )),
        Some(WindowEvent::Resized(size)) => Some((
            raw.outer_bounds(),
            PhysicalSize::new(size.width, size.height),
        )),
        _ => None,
    };
    if let Some((bounds, size)) = geometry {
        *raw = PhysicalLiveWindowObservation::new(
            Some(window_id.clone()),
            raw.transport_handle().clone(),
            bounds,
            size,
            raw.scale(),
            raw.is_maximized(),
            raw.is_visible(),
            raw.is_focused(),
        );
    }
    let snapshot = PhysicalDesktopSnapshot::new(displays, windows);
    let mapped = mapper.map_desktop(&snapshot)?;
    Ok((snapshot, mapped))
}
