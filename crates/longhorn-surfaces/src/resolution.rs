use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{SurfaceId, WindowId};

use crate::{SurfaceDocument, SurfaceLimits, normalize_document};

mod error;
mod model;

pub use error::{SurfaceResolutionError, SurfaceResolutionErrorCode};
pub use model::{
    ResolvedSurface, ResolvedSurfaceWindow, SurfaceResolution, SurfaceResolutionInput,
    SurfaceUnresolvedReason, UnresolvedSurface,
};

use error::resolution_error;

/// Resolves current presence against available declared candidate windows.
pub fn resolve_surfaces(
    limits: SurfaceLimits,
    document: &SurfaceDocument,
    input: &SurfaceResolutionInput,
) -> Result<SurfaceResolution, SurfaceResolutionError> {
    let document = normalize_document(limits, document).map_err(|error| {
        resolution_error(
            SurfaceResolutionErrorCode::InvalidDocument,
            error.to_string(),
        )
    })?;

    if input.admitted_surface_ids().len() > limits.maximum_surfaces() {
        return Err(resolution_error(
            SurfaceResolutionErrorCode::TooManyAdmittedSurfaces,
            format!(
                "{} admitted Surfaces exceed limit {}",
                input.admitted_surface_ids().len(),
                limits.maximum_surfaces()
            ),
        ));
    }
    if input.available_window_ids().len() > limits.maximum_windows() {
        return Err(resolution_error(
            SurfaceResolutionErrorCode::TooManyAvailableWindows,
            format!(
                "{} available windows exceed limit {}",
                input.available_window_ids().len(),
                limits.maximum_windows()
            ),
        ));
    }

    let document_surface_ids = document
        .surfaces()
        .iter()
        .map(|surface| surface.id())
        .collect::<BTreeSet<_>>();
    let document_window_ids = document
        .windows()
        .iter()
        .map(|window| window.id())
        .collect::<BTreeSet<_>>();

    let mut admitted_surface_ids = BTreeSet::<&SurfaceId>::new();
    for surface_id in input.admitted_surface_ids() {
        if !admitted_surface_ids.insert(surface_id) {
            return Err(resolution_error(
                SurfaceResolutionErrorCode::DuplicateAdmittedSurface,
                format!("duplicate admitted Surface {surface_id}"),
            ));
        }
        if !document_surface_ids.contains(surface_id) {
            return Err(resolution_error(
                SurfaceResolutionErrorCode::UnknownAdmittedSurface,
                format!("unknown admitted Surface {surface_id}"),
            ));
        }
    }

    let mut available_window_ids = BTreeSet::<&WindowId>::new();
    for window_id in input.available_window_ids() {
        if !available_window_ids.insert(window_id) {
            return Err(resolution_error(
                SurfaceResolutionErrorCode::DuplicateAvailableWindow,
                format!("duplicate available window {window_id}"),
            ));
        }
        if !document_window_ids.contains(window_id) {
            return Err(resolution_error(
                SurfaceResolutionErrorCode::UnknownAvailableWindow,
                format!("unknown available participating window {window_id}"),
            ));
        }
    }

    let mut assigned_by_window = BTreeMap::<WindowId, Vec<(u32, ResolvedSurface)>>::new();
    let mut unresolved_surfaces = Vec::new();

    for surface in document.surfaces() {
        if !admitted_surface_ids.contains(surface.id()) {
            unresolved_surfaces.push(UnresolvedSurface {
                surface_id: surface.id().clone(),
                layout_container_id: surface.layout_container_id().clone(),
                reason: SurfaceUnresolvedReason::NotAdmitted,
            });
            continue;
        }

        let selected = surface
            .host_preferences()
            .iter()
            .enumerate()
            .find(|(_, preference)| available_window_ids.contains(preference.window_id()));
        if let Some((preference_index, preference)) = selected {
            let host_preference_index = u32::try_from(preference_index)
                .expect("Surface limits keep preference indices within u32");
            assigned_by_window
                .entry(preference.window_id().clone())
                .or_default()
                .push((
                    preference.order(),
                    ResolvedSurface {
                        surface_id: surface.id().clone(),
                        layout_container_id: surface.layout_container_id().clone(),
                        label: surface.label().map(ToOwned::to_owned),
                        host_preference_index,
                    },
                ));
        } else {
            unresolved_surfaces.push(UnresolvedSurface {
                surface_id: surface.id().clone(),
                layout_container_id: surface.layout_container_id().clone(),
                reason: SurfaceUnresolvedReason::NoAvailableWindow,
            });
        }
    }

    let mut windows = Vec::with_capacity(available_window_ids.len());
    for window in document
        .windows()
        .iter()
        .filter(|window| available_window_ids.contains(window.id()))
    {
        let mut ordered = assigned_by_window.remove(window.id()).unwrap_or_default();
        ordered.sort_by_key(|(order, _)| *order);
        let surfaces = ordered
            .into_iter()
            .map(|(_, surface)| surface)
            .collect::<Vec<_>>();
        let active_surface_id = window
            .active_surface_id()
            .filter(|active| {
                surfaces
                    .iter()
                    .any(|surface| surface.surface_id() == *active)
            })
            .cloned()
            .or_else(|| surfaces.first().map(|surface| surface.surface_id().clone()));
        windows.push(ResolvedSurfaceWindow {
            window_id: window.id().clone(),
            surfaces,
            active_surface_id,
        });
    }

    Ok(SurfaceResolution {
        revision: document.revision(),
        windows,
        unresolved_surfaces,
    })
}
