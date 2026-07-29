use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{SurfaceId, SurfaceRevision, WindowId};
use longhorn_surfaces::{
    ResolvedSurfaceWindow, SurfaceDocument, SurfaceLimits, SurfaceResolutionError,
    SurfaceResolutionInput, UnresolvedSurface, resolve_surfaces,
};
use longhorn_windowing::{DesiredWindow, ResolvedWindowPlacement, WindowPlacementResolution};

/// One participating window's resolved Surface binding and desired host state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowBinding {
    surfaces: ResolvedSurfaceWindow,
    placement: ResolvedWindowPlacement,
    desired_window: DesiredWindow,
}

impl SurfaceWindowBinding {
    /// Returns resolved ordered Surface membership and active state.
    #[must_use]
    pub const fn surfaces(&self) -> &ResolvedSurfaceWindow {
        &self.surfaces
    }

    /// Returns complete placement evidence selected by `longhorn-windowing`.
    #[must_use]
    pub const fn placement(&self) -> &ResolvedWindowPlacement {
        &self.placement
    }

    /// Returns the desired host input projected from the placement.
    #[must_use]
    pub const fn desired_window(&self) -> &DesiredWindow {
        &self.desired_window
    }
}

/// Complete current Surface-to-window host projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowPlan {
    surface_revision: SurfaceRevision,
    windows: Vec<SurfaceWindowBinding>,
    unresolved_surfaces: Vec<UnresolvedSurface>,
}

impl SurfaceWindowPlan {
    /// Returns the durable Surface revision projected by this plan.
    #[must_use]
    pub const fn surface_revision(&self) -> SurfaceRevision {
        self.surface_revision
    }

    /// Returns participating windows in canonical identity order.
    #[must_use]
    pub fn windows(&self) -> &[SurfaceWindowBinding] {
        self.windows.as_slice()
    }

    /// Returns Surfaces without a current eligible host.
    #[must_use]
    pub fn unresolved_surfaces(&self) -> &[UnresolvedSurface] {
        self.unresolved_surfaces.as_slice()
    }

    /// Iterates desired host inputs without exposing consumer factory policy.
    pub fn desired_windows(&self) -> impl ExactSizeIterator<Item = &DesiredWindow> {
        self.windows
            .iter()
            .map(SurfaceWindowBinding::desired_window)
    }
}

/// Stable Surface/window composition failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceWindowCompositionErrorCode {
    /// One participating window had more than one placement outcome.
    DuplicatePlacementOutcome,
    /// Surface state or caller-resolved presence input was invalid.
    SurfaceResolution,
    /// An internal input relationship was incomplete.
    MissingResolvedPlacement,
}

/// Rejected Surface/window composition input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceWindowCompositionError {
    code: SurfaceWindowCompositionErrorCode,
    detail: String,
    surface_resolution: Option<SurfaceResolutionError>,
}

impl SurfaceWindowCompositionError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn code(&self) -> SurfaceWindowCompositionErrorCode {
        self.code
    }

    /// Returns the diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns the underlying Surface resolution rejection when applicable.
    #[must_use]
    pub const fn surface_resolution(&self) -> Option<&SurfaceResolutionError> {
        self.surface_resolution.as_ref()
    }
}

impl fmt::Display for SurfaceWindowCompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceWindowCompositionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.surface_resolution
            .as_ref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// Resolves admitted Surfaces against currently placeable participating windows.
///
/// Placement remains owned by `longhorn-windowing`. Direct non-Surface window
/// outcomes are ignored. The visibility callback is consumer policy.
pub fn compose_surface_window_plan<F>(
    limits: SurfaceLimits,
    document: &SurfaceDocument,
    admitted_surface_ids: impl IntoIterator<Item = SurfaceId>,
    placement_outcomes: &[WindowPlacementResolution],
    mut is_visible: F,
) -> Result<SurfaceWindowPlan, SurfaceWindowCompositionError>
where
    F: FnMut(&WindowId) -> bool,
{
    let participating = document
        .windows()
        .iter()
        .map(|window| window.id())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut placements = BTreeMap::<WindowId, ResolvedWindowPlacement>::new();

    for outcome in placement_outcomes {
        let window_id = placement_window_id(outcome);
        if !participating.contains(window_id) {
            continue;
        }
        if !seen.insert(window_id) {
            return Err(composition_error(
                SurfaceWindowCompositionErrorCode::DuplicatePlacementOutcome,
                format!("participating window {window_id} has duplicate placement outcomes"),
            ));
        }
        if let WindowPlacementResolution::Resolved(placement) = outcome {
            placements.insert(window_id.clone(), placement.clone());
        }
    }

    let resolution = resolve_surfaces(
        limits,
        document,
        &SurfaceResolutionInput::new(admitted_surface_ids, placements.keys().cloned()),
    )
    .map_err(surface_resolution_error)?;
    let mut windows = Vec::with_capacity(resolution.windows().len());
    for surfaces in resolution.windows() {
        let placement = placements.get(surfaces.window_id()).ok_or_else(|| {
            composition_error(
                SurfaceWindowCompositionErrorCode::MissingResolvedPlacement,
                format!(
                    "resolved Surface window {} has no placement evidence",
                    surfaces.window_id()
                ),
            )
        })?;
        windows.push(SurfaceWindowBinding {
            surfaces: surfaces.clone(),
            placement: placement.clone(),
            desired_window: DesiredWindow::from_resolved(
                placement,
                is_visible(surfaces.window_id()),
            ),
        });
    }

    Ok(SurfaceWindowPlan {
        surface_revision: resolution.revision(),
        windows,
        unresolved_surfaces: resolution.unresolved_surfaces().to_vec(),
    })
}

fn placement_window_id(outcome: &WindowPlacementResolution) -> &WindowId {
    match outcome {
        WindowPlacementResolution::Disabled { window_id, .. } => window_id,
        WindowPlacementResolution::Resolved(placement) => placement.window_id(),
        WindowPlacementResolution::Unavailable(placement) => placement.window_id(),
    }
}

fn surface_resolution_error(error: SurfaceResolutionError) -> SurfaceWindowCompositionError {
    SurfaceWindowCompositionError {
        code: SurfaceWindowCompositionErrorCode::SurfaceResolution,
        detail: error.to_string(),
        surface_resolution: Some(error),
    }
}

fn composition_error(
    code: SurfaceWindowCompositionErrorCode,
    detail: impl Into<String>,
) -> SurfaceWindowCompositionError {
    SurfaceWindowCompositionError {
        code,
        detail: detail.into(),
        surface_resolution: None,
    }
}
