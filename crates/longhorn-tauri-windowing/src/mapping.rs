use std::collections::BTreeSet;

use longhorn_core::{RoundingMode, ScreenRect, ScreenSize};
use longhorn_display::ObservationId;
use longhorn_windowing::HostWindowHandle;
use serde::{Deserialize, Serialize};

use crate::{DesktopMappingError, PhysicalDesktopSnapshot};

mod project;

pub use project::project_desktop;

/// Logical geometry returned for one raw display observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MappedDisplayGeometry {
    observation_id: ObservationId,
    full_bounds: ScreenRect,
    work_area: ScreenRect,
}

impl MappedDisplayGeometry {
    /// Constructs mapped display geometry.
    #[must_use]
    pub const fn new(
        observation_id: ObservationId,
        full_bounds: ScreenRect,
        work_area: ScreenRect,
    ) -> Self {
        Self {
            observation_id,
            full_bounds,
            work_area,
        }
    }

    /// Returns the raw observation being mapped.
    #[must_use]
    pub const fn observation_id(&self) -> &ObservationId {
        &self.observation_id
    }

    /// Returns global logical full bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> ScreenRect {
        self.full_bounds
    }

    /// Returns global logical work area.
    #[must_use]
    pub const fn work_area(&self) -> ScreenRect {
        self.work_area
    }
}

/// Logical geometry returned for one managed raw window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MappedWindowGeometry {
    transport_handle: HostWindowHandle,
    outer_bounds: ScreenRect,
    inner_size: ScreenSize,
}

impl MappedWindowGeometry {
    /// Constructs mapped window geometry.
    #[must_use]
    pub const fn new(
        transport_handle: HostWindowHandle,
        outer_bounds: ScreenRect,
        inner_size: ScreenSize,
    ) -> Self {
        Self {
            transport_handle,
            outer_bounds,
            inner_size,
        }
    }

    /// Returns the managed transport handle being mapped.
    #[must_use]
    pub const fn transport_handle(&self) -> &HostWindowHandle {
        &self.transport_handle
    }

    /// Returns global logical outer-frame bounds.
    #[must_use]
    pub const fn outer_bounds(&self) -> ScreenRect {
        self.outer_bounds
    }

    /// Returns logical inner content size.
    #[must_use]
    pub const fn inner_size(&self) -> ScreenSize {
        self.inner_size
    }
}

/// Complete logical geometry returned by a whole-desktop mapper.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MappedDesktopGeometry {
    displays: Vec<MappedDisplayGeometry>,
    windows: Vec<MappedWindowGeometry>,
}

impl MappedDesktopGeometry {
    /// Constructs complete mapped geometry.
    #[must_use]
    pub fn new(
        displays: impl IntoIterator<Item = MappedDisplayGeometry>,
        windows: impl IntoIterator<Item = MappedWindowGeometry>,
    ) -> Self {
        Self {
            displays: displays.into_iter().collect(),
            windows: windows.into_iter().collect(),
        }
    }

    /// Returns all mapped displays.
    #[must_use]
    pub fn displays(&self) -> &[MappedDisplayGeometry] {
        &self.displays
    }

    /// Returns all mapped managed windows.
    #[must_use]
    pub fn windows(&self) -> &[MappedWindowGeometry] {
        &self.windows
    }
}

/// Maps one complete physical desktop into one coherent global logical plane.
pub trait DesktopCoordinateMapper {
    /// Maps every display and managed window or rejects the complete snapshot.
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError>;
}

impl<F> DesktopCoordinateMapper for F
where
    F: Fn(&PhysicalDesktopSnapshot) -> Result<MappedDesktopGeometry, DesktopMappingError>,
{
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        self(snapshot)
    }
}

/// Built-in mapper for a desktop where every raw fact has one exact scale.
#[derive(Clone, Copy, Debug, Default)]
pub struct UniformScaleMapper;

impl DesktopCoordinateMapper for UniformScaleMapper {
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        let scales = snapshot
            .displays()
            .iter()
            .map(|display| display.facts().scale())
            .chain(snapshot.windows().iter().map(|window| window.scale()))
            .collect::<BTreeSet<_>>();

        if scales.len() > 1 {
            return Err(DesktopMappingError::MixedScaleUnavailable {
                scales: scales.into_iter().collect(),
            });
        }
        let Some(scale) = scales.into_iter().next() else {
            return Ok(MappedDesktopGeometry::default());
        };

        let displays = snapshot
            .displays()
            .iter()
            .map(|display| {
                Ok(MappedDisplayGeometry::new(
                    display.metadata().observation_id().clone(),
                    scale.physical_rect_to_screen(
                        display.facts().full_bounds(),
                        RoundingMode::Nearest,
                    )?,
                    scale.physical_rect_to_screen(
                        display.facts().work_area(),
                        RoundingMode::Nearest,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;
        let windows = snapshot
            .windows()
            .iter()
            .map(|window| {
                Ok(MappedWindowGeometry::new(
                    window.transport_handle().clone(),
                    scale.physical_rect_to_screen(window.outer_bounds(), RoundingMode::Nearest)?,
                    scale.physical_size_to_screen(window.inner_size(), RoundingMode::Nearest)?,
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;

        Ok(MappedDesktopGeometry::new(displays, windows))
    }
}

/// Maps a desktop whose physical facts are derived from a logical layout.
///
/// Converts every display and window through its own scale, which establishes
/// one coherent plane across mixed scales — the arrangement
/// [`UniformScaleMapper`] refuses.
///
/// # Where this is valid, and where it is not
///
/// It depends entirely on what the host means by "physical", and the three
/// platforms do not agree:
///
/// - **macOS** lays the desktop out in points. Tauri reports a monitor's
///   position as `CGDisplayBounds.origin * that display's own scale`, so
///   dividing by the same scale returns the original layout exactly.
/// - **Linux** has the same shape through GTK: the monitor geometry is logical
///   and Tauri multiplies it by the scale to report physical.
/// - **Windows does not.** Tauri reports `rcMonitor` straight from the OS, and
///   that is a real physical-pixel virtual desktop rather than anything derived
///   from a logical layout. Per-monitor division breaks there. A 3840x2160
///   display at 200% followed by a 1920x1080 display at 100% puts the second
///   monitor's physical origin at x=3840; divided by its own scale that stays
///   3840, where a coherent logical layout wants 1920. The result is a
///   1920-wide gap between two monitors that physically touch.
///
/// So this is not a general desktop mapping and must not be composed on
/// Windows, which is what contract 009's rule against per-monitor division is
/// really about. Windows needs a mapper that reads its own layout; until one
/// exists, a mixed-scale Windows desktop keeps [`UniformScaleMapper`]'s typed
/// refusal, which is honest rather than silently wrong.
#[derive(Clone, Copy, Debug, Default)]
pub struct LogicalLayoutMapper;

impl DesktopCoordinateMapper for LogicalLayoutMapper {
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        let displays = snapshot
            .displays()
            .iter()
            .map(|display| {
                let scale = display.facts().scale();
                Ok(MappedDisplayGeometry::new(
                    display.metadata().observation_id().clone(),
                    scale.physical_rect_to_screen(
                        display.facts().full_bounds(),
                        RoundingMode::Nearest,
                    )?,
                    scale.physical_rect_to_screen(
                        display.facts().work_area(),
                        RoundingMode::Nearest,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;
        let windows = snapshot
            .windows()
            .iter()
            .map(|window| {
                let scale = window.scale();
                Ok(MappedWindowGeometry::new(
                    window.transport_handle().clone(),
                    scale.physical_rect_to_screen(window.outer_bounds(), RoundingMode::Nearest)?,
                    scale.physical_size_to_screen(window.inner_size(), RoundingMode::Nearest)?,
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;

        Ok(MappedDesktopGeometry::new(displays, windows))
    }
}
