use std::sync::Arc;

use longhorn_core::{
    PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode, ScaleFactor, ScreenPoint, ScreenRect,
    ScreenSize, WindowId, WindowPlacement,
};
use tauri::{Runtime, WebviewWindow};

use super::{
    CapturedDisplayAssociation, CapturedDisplayEvidence, CapturedWindowPlacement,
    WindowCaptureBackend,
};
use crate::scale_factor_from_tauri;

/// Maps native event/capture geometry into the declared global screen plane.
pub trait WindowGeometryMapper: Send + Sync {
    /// Maps one outer-frame physical origin.
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, String>;

    /// Maps one inner-content physical size.
    fn map_inner_size(
        &self,
        size: PhysicalSize,
        window_scale: ScaleFactor,
    ) -> Result<ScreenSize, String>;
}

/// Mapper for a desktop whose one global scale was established by the caller.
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
    ///
    /// This is not a generic Tauri default. Tauri capture values are physical,
    /// so using this on a scaled display doubles persisted logical geometry.
    #[must_use]
    pub fn identity_scale() -> Self {
        Self::new(ScaleFactor::from_thousandths(1_000).expect("identity scale is valid"))
    }
}

impl WindowGeometryMapper for UniformWindowGeometryMapper {
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        _window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, String> {
        self.scale
            .physical_point_to_screen(origin, RoundingMode::Nearest)
            .map_err(|error| error.to_string())
    }

    fn map_inner_size(
        &self,
        size: PhysicalSize,
        _window_scale: ScaleFactor,
    ) -> Result<ScreenSize, String> {
        self.scale
            .physical_size_to_screen(size, RoundingMode::Nearest)
            .map_err(|error| error.to_string())
    }
}

/// Mapper using the live Tauri window or monitor scale for each conversion.
///
/// This keeps captured sizes correct as a window crosses displays with
/// different scale factors. Global origins are correct only on platforms where
/// dividing Tauri's physical origin by the current scale establishes one
/// coherent logical desktop plane. Otherwise inject a platform mapper.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowScaleGeometryMapper;

impl WindowGeometryMapper for WindowScaleGeometryMapper {
    fn map_outer_origin(
        &self,
        origin: PhysicalPoint,
        window_scale: ScaleFactor,
    ) -> Result<ScreenPoint, String> {
        window_scale
            .physical_point_to_screen(origin, RoundingMode::Nearest)
            .map_err(|error| error.to_string())
    }

    fn map_inner_size(
        &self,
        size: PhysicalSize,
        window_scale: ScaleFactor,
    ) -> Result<ScreenSize, String> {
        window_scale
            .physical_size_to_screen(size, RoundingMode::Nearest)
            .map_err(|error| error.to_string())
    }
}

/// Direct complete Tauri capture using an injected global-plane mapper.
pub struct TauriWindowCaptureBackend {
    mapper: Arc<dyn WindowGeometryMapper>,
}

impl TauriWindowCaptureBackend {
    /// Constructs capture with explicit global coordinate authority.
    #[must_use]
    pub fn new(mapper: Arc<dyn WindowGeometryMapper>) -> Self {
        Self { mapper }
    }
}

impl<R: Runtime> WindowCaptureBackend<R> for TauriWindowCaptureBackend {
    fn capture(
        &self,
        window_id: &WindowId,
        window: &WebviewWindow<R>,
        retained_normal: Option<WindowPlacement>,
    ) -> Result<CapturedWindowPlacement, String> {
        let scale = scale_factor_from_tauri(
            window
                .scale_factor()
                .map_err(|error| format!("window scale probe failed: {error}"))?,
        )
        .map_err(|error| error.to_string())?;
        let maximized = window
            .is_maximized()
            .map_err(|error| format!("maximized probe failed: {error}"))?;
        let normal_placement = if maximized {
            retained_normal
                .ok_or_else(|| "maximized capture has no retained normal placement".to_string())?
        } else {
            let position = window
                .outer_position()
                .map_err(|error| format!("outer position probe failed: {error}"))?;
            let size = window
                .inner_size()
                .map_err(|error| format!("inner size probe failed: {error}"))?;
            WindowPlacement::new(
                self.mapper
                    .map_outer_origin(PhysicalPoint::new(position.x, position.y), scale)?,
                self.mapper
                    .map_inner_size(PhysicalSize::new(size.width, size.height), scale)?,
            )
        };

        let display = match window
            .current_monitor()
            .map_err(|error| format!("current monitor probe failed: {error}"))?
        {
            Some(monitor) => {
                let monitor_scale = scale_factor_from_tauri(monitor.scale_factor())
                    .map_err(|error| format!("current monitor scale is invalid: {error}"))?;
                let position = monitor.position();
                let size = monitor.size();
                let work = monitor.work_area();
                let logical_full_bounds = ScreenRect::new(
                    self.mapper.map_outer_origin(
                        PhysicalPoint::new(position.x, position.y),
                        monitor_scale,
                    )?,
                    self.mapper.map_inner_size(
                        PhysicalSize::new(size.width, size.height),
                        monitor_scale,
                    )?,
                );
                let logical_work_area = ScreenRect::new(
                    self.mapper.map_outer_origin(
                        PhysicalPoint::new(work.position.x, work.position.y),
                        monitor_scale,
                    )?,
                    self.mapper.map_inner_size(
                        PhysicalSize::new(work.size.width, work.size.height),
                        monitor_scale,
                    )?,
                );
                CapturedDisplayAssociation::Observed {
                    evidence: CapturedDisplayEvidence::new(
                        monitor.name().cloned(),
                        PhysicalRect::new(
                            PhysicalPoint::new(position.x, position.y),
                            PhysicalSize::new(size.width, size.height),
                        ),
                        PhysicalRect::new(
                            PhysicalPoint::new(work.position.x, work.position.y),
                            PhysicalSize::new(work.size.width, work.size.height),
                        ),
                        monitor_scale,
                        logical_full_bounds,
                        logical_work_area,
                    ),
                }
            }
            None => CapturedDisplayAssociation::Unresolved,
        };

        Ok(CapturedWindowPlacement::new(
            window_id.clone(),
            normal_placement,
            maximized,
            display,
        ))
    }
}
