//! The macOS whole-desktop coordinate mapper.
//!
//! Contract 009 requires an injected platform mapper before a mixed-scale
//! desktop has a global logical origin, and refuses the arithmetic shortcut:
//! dividing each monitor origin by its own scale is not a valid generic desktop
//! mapping. This module supplies the macOS answer to that requirement.
//!
//! # Why macOS needs no arithmetic at all
//!
//! macOS already composites one coherent logical desktop. `CGDisplayBounds`
//! reports each display in **points**, in a single global plane whose origin is
//! the top-left of the main display and whose y grows downward — which is
//! exactly [`ScreenDip`](longhorn_core::ScreenDip). The plane is not something
//! to derive; it is something to read.
//!
//! Measured on a genuinely mixed-scale desktop (2026-08-17), the arrangement
//! this mapper exists for:
//!
//! ```text
//! DELL U3415W   CGDisplayBounds (0, 0) 3440x1440 pt    backingScale 1.0
//! Built-in XDR  CGDisplayBounds (-1577, 1440) 1800x1169 pt  backingScale 2.0
//! ```
//!
//! One plane, negative origin included, with no per-monitor division anywhere.
//!
//! # Why the physical facts cannot be the source
//!
//! On macOS, Tauri's "physical" values are *derived from* that points plane
//! rather than being an independent physical desktop: tao computes a monitor's
//! position as `CGDisplayBounds.origin * that display's own scale`, and a
//! window's as its flipped `NSWindow.frame` origin times the window's scale.
//! Division therefore happens to invert on macOS today — and that is precisely
//! why contract 009 refuses to bless it. It is a property of one windowing
//! crate's arithmetic, not of the platform, and a change there would silently
//! move every restored window.
//!
//! So the physical facts are used for exactly one thing: **correlating** a
//! Tauri observation to the native display it describes. Display geometry
//! always comes back from the platform's own plane. If the correlation stops
//! matching, this mapper fails typed rather than mapping anything.
//!
//! # Windows convert; displays are read
//!
//! A window's frame is not read natively. Doing so means dereferencing the
//! `NSWindow` pointer Tauri hands out, and this workspace forbids `unsafe`
//! outright — a boundary worth more than the last unit of precision here.
//!
//! Instead a window converts through its own scale, which is exact on macOS for
//! the same reason the correlation works: the host derives a window's physical
//! frame from its logical frame times that scale. The display correlation is
//! what licenses it. If the host ever changed that derivation, correlation
//! would fail on the displays first and the snapshot would be refused before
//! any window was mapped — so the two are not independent assumptions, they are
//! one assumption with a guard in front of it.

use crate::{DesktopMappingError, PhysicalDesktopSnapshot};
use longhorn_core::{PhysicalSize, ScaleFactor, ScreenRect};

use super::{MappedDesktopGeometry, MappedDisplayGeometry, MappedWindowGeometry};

#[cfg(target_os = "macos")]
mod appkit;

#[cfg(target_os = "macos")]
pub use appkit::AppKitDesktopPlane;

/// One display as the platform's own logical plane reports it.
///
/// `full_bounds` and `work_area` are global logical points, top-left origin.
/// `physical_size` and `scale` exist to correlate this display with the Tauri
/// observation that describes it, and are never used to compute geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDisplayGeometry {
    full_bounds: ScreenRect,
    work_area: ScreenRect,
    physical_size: PhysicalSize,
    scale: ScaleFactor,
    is_main: bool,
}

impl NativeDisplayGeometry {
    /// Records one native display.
    #[must_use]
    pub const fn new(
        full_bounds: ScreenRect,
        work_area: ScreenRect,
        physical_size: PhysicalSize,
        scale: ScaleFactor,
        is_main: bool,
    ) -> Self {
        Self {
            full_bounds,
            work_area,
            physical_size,
            scale,
            is_main,
        }
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

    /// Returns the physical size this display presents to Tauri.
    #[must_use]
    pub const fn physical_size(&self) -> PhysicalSize {
        self.physical_size
    }

    /// Returns the backing scale.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }

    /// Returns whether this is the main display.
    #[must_use]
    pub const fn is_main(&self) -> bool {
        self.is_main
    }
}

/// Reads the platform's own logical desktop plane.
///
/// Injected rather than called directly so the mapper's correlation and refusal
/// rules are provable without a display attached. The production implementation
/// reads Core Graphics and AppKit; tests supply measured arrangements.
pub trait NativeDesktopPlane {
    /// Returns every active display in the platform's logical plane.
    fn displays(&self) -> Result<Vec<NativeDisplayGeometry>, String>;
}

/// Maps a mixed-scale macOS desktop through the platform's own logical plane.
///
/// Composed in place of [`UniformScaleMapper`](super::UniformScaleMapper) when
/// the desktop may contain displays of more than one scale. It does not weaken
/// that mapper's contract: uniform desktops map identically here, and anything
/// this mapper cannot establish fails typed rather than being approximated.
#[derive(Clone, Copy, Debug, Default)]
pub struct MacOsDesktopMapper<N> {
    plane: N,
}

impl<N> MacOsDesktopMapper<N> {
    /// Records a mapper over one native plane reader.
    #[must_use]
    pub const fn new(plane: N) -> Self {
        Self { plane }
    }
}

impl<N: NativeDesktopPlane> super::DesktopCoordinateMapper for MacOsDesktopMapper<N> {
    fn map_desktop(
        &self,
        snapshot: &PhysicalDesktopSnapshot,
    ) -> Result<MappedDesktopGeometry, DesktopMappingError> {
        let native = self
            .plane
            .displays()
            .map_err(|detail| DesktopMappingError::Provider { detail })?;

        let displays = snapshot
            .displays()
            .iter()
            .map(|observed| {
                let matched = correlate(&native, observed)?;
                Ok(MappedDisplayGeometry::new(
                    observed.metadata().observation_id().clone(),
                    matched.full_bounds(),
                    matched.work_area(),
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;

        let windows = snapshot
            .windows()
            .iter()
            .map(|observed| {
                // Licensed by the correlation above, not assumed: every
                // display's physical facts were just shown to be its own points
                // times its own scale, so the same derivation holds for a
                // window and inverts through the window's own scale. Had that
                // stopped being true, the display correlation would already
                // have refused this snapshot.
                let scale = observed.scale();
                Ok(MappedWindowGeometry::new(
                    observed.transport_handle().clone(),
                    scale.physical_rect_to_screen(
                        observed.outer_bounds(),
                        longhorn_core::RoundingMode::Nearest,
                    )?,
                    scale.physical_size_to_screen(
                        observed.inner_size(),
                        longhorn_core::RoundingMode::Nearest,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, DesktopMappingError>>()?;

        Ok(MappedDesktopGeometry::new(displays, windows))
    }
}

/// Finds the one native display an observation describes.
///
/// Correlates on physical size, scale, and main-display status — facts both
/// sides state independently. Position is deliberately not part of the key: it
/// is the value most exposed to a future change in how the windowing crate
/// derives physical coordinates, and a key that drifts silently is worse than
/// one that is coarse.
///
/// Anything other than exactly one match fails typed. Two identical external
/// displays are genuinely ambiguous from these facts, and contract 009 already
/// sets the precedent: main-display attribution fails rather than marking an
/// arbitrary monitor.
fn correlate<'plane>(
    native: &'plane [NativeDisplayGeometry],
    observed: &crate::PhysicalDisplayObservation,
) -> Result<&'plane NativeDisplayGeometry, DesktopMappingError> {
    let facts = observed.facts();
    let physical_size = facts.full_bounds().size();
    let matches = native
        .iter()
        .filter(|candidate| {
            candidate.physical_size() == physical_size
                && candidate.scale() == facts.scale()
                && candidate.is_main() == facts.is_main()
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [single] => Ok(single),
        [] => Err(DesktopMappingError::Provider {
            detail: format!(
                "no native display matches observation {} ({}x{} at scale {}); the platform \
                 plane and the host's physical facts disagree",
                observed.metadata().observation_id(),
                physical_size.width(),
                physical_size.height(),
                facts.scale().thousandths(),
            ),
        }),
        ambiguous => Err(DesktopMappingError::Provider {
            detail: format!(
                "{} native displays match observation {} ({}x{} at scale {}); identical \
                 displays cannot be told apart from these facts",
                ambiguous.len(),
                observed.metadata().observation_id(),
                physical_size.width(),
                physical_size.height(),
                facts.scale().thousandths(),
            ),
        }),
    }
}
