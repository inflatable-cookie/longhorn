use std::collections::BTreeSet;

use longhorn_core::SurfaceId;

use crate::{
    SurfaceDocument, SurfaceHostPreference, SurfacePresentation, SurfaceRecord,
    SurfaceValidationCode,
};

use super::{
    EmptyWindowPolicy, LayoutContainerCleanupIntent, LayoutContainerInventory,
    SurfaceMutationCommand, SurfaceMutationOutcome, SurfaceMutationRejectionCode,
    error::{OperationRejection, operation_rejection},
};

pub(super) fn apply_command(
    document: &mut SurfaceDocument,
    command: &SurfaceMutationCommand,
    layout_containers: &LayoutContainerInventory,
    empty_policy: EmptyWindowPolicy,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    match command {
        SurfaceMutationCommand::CreateSurface {
            surface_id,
            layout_container_id,
            label,
            host_preferences,
        } => {
            require_fresh_ids(document, layout_containers, surface_id, layout_container_id)?;
            reject_repeated_preferences(host_preferences)?;
            document.surfaces_mut().push(SurfaceRecord::new(
                surface_id.clone(),
                layout_container_id.clone(),
                label.clone(),
                host_preferences.clone(),
            ));
            Ok(SurfaceMutationOutcome::SurfaceCreated {
                surface_id: surface_id.clone(),
            })
        }
        SurfaceMutationCommand::DuplicateSurface {
            source_surface_id,
            surface_id,
            layout_container_id,
        } => duplicate_surface(
            document,
            layout_containers,
            source_surface_id,
            surface_id,
            layout_container_id,
        ),
        SurfaceMutationCommand::RenameSurface { surface_id, label } => {
            let surface = document.surface_mut(surface_id).ok_or_else(|| {
                operation_rejection(
                    SurfaceMutationRejectionCode::UnknownSurface,
                    format!("unknown Surface {surface_id}"),
                )
            })?;
            surface.set_label(label.clone());
            Ok(SurfaceMutationOutcome::SurfaceRenamed {
                surface_id: surface_id.clone(),
            })
        }
        SurfaceMutationCommand::SetSurfacePresentation {
            surface_id,
            presentation,
        } => set_presentation(document, surface_id, presentation),
        SurfaceMutationCommand::ActivateSurface {
            window_id,
            surface_id,
        } => activate_surface(document, window_id, surface_id),
        SurfaceMutationCommand::ReorderWindow {
            window_id,
            surface_ids,
        } => reorder_window(document, window_id, surface_ids),
        SurfaceMutationCommand::MoveSurface {
            surface_id,
            target_window_id,
            insertion_index,
        } => move_surface(
            document,
            surface_id,
            target_window_id,
            *insertion_index,
            empty_policy,
        ),
        SurfaceMutationCommand::CloseSurface { surface_id } => {
            close_surface(document, surface_id, empty_policy)
        }
    }
}

/// Replaces one Surface's presentation.
///
/// Setting the same presentation twice is accepted rather than rejected. There
/// is no `MoveTargetUnchanged` equivalent here because a no-op presentation
/// carries no ambiguity for a caller to have got wrong -- unlike a move, which
/// names a target window the caller believed was different.
fn set_presentation(
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    presentation: &SurfacePresentation,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    let surface = document.surface_mut(surface_id).ok_or_else(|| {
        operation_rejection(
            SurfaceMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        )
    })?;
    let previous_presentation = surface.presentation().clone();
    surface.set_presentation(presentation.clone());
    Ok(SurfaceMutationOutcome::SurfacePresentationSet {
        surface_id: surface_id.clone(),
        presentation: presentation.clone(),
        previous_presentation,
    })
}

fn require_fresh_ids(
    document: &SurfaceDocument,
    layout_containers: &LayoutContainerInventory,
    surface_id: &SurfaceId,
    layout_container_id: &longhorn_core::LayoutContainerId,
) -> Result<(), OperationRejection> {
    if document.surface(surface_id).is_some() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::DuplicateSurface,
            format!("Surface {surface_id} already exists"),
        ));
    }
    if !layout_containers.contains(layout_container_id) {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UnknownLayoutContainer,
            format!("layout container {layout_container_id} does not exist"),
        ));
    }
    if document
        .surfaces()
        .iter()
        .any(|surface| surface.layout_container_id() == layout_container_id)
    {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::LayoutContainerAlreadyBound,
            format!("layout container {layout_container_id} is already bound"),
        ));
    }
    Ok(())
}

fn reject_repeated_preferences(
    preferences: &[SurfaceHostPreference],
) -> Result<(), OperationRejection> {
    let mut windows = BTreeSet::new();
    for preference in preferences {
        if !windows.insert(preference.window_id()) {
            return Err(operation_rejection(
                SurfaceMutationRejectionCode::DuplicateHostPreference,
                format!("host window {} is repeated", preference.window_id()),
            ));
        }
    }
    Ok(())
}

pub(super) fn map_candidate_validation(
    _code: SurfaceValidationCode,
) -> SurfaceMutationRejectionCode {
    SurfaceMutationRejectionCode::InvalidCandidate
}

mod close_move;
mod metadata;
mod ordering;

use close_move::{close_surface, move_surface};
use metadata::duplicate_surface;
use ordering::{activate_surface, reorder_window};
