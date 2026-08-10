use longhorn_core::{SurfaceId, WindowId};

use crate::SurfaceDocument;

use super::{
    EmptyWindowPolicy, OperationRejection, SurfaceMutationOutcome, SurfaceMutationRejectionCode,
    operation_rejection,
    ordering::{ordered_members, primary_members, set_window_order},
};

pub(super) fn move_surface(
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    target_window_id: &WindowId,
    insertion_index: u32,
    empty_policy: EmptyWindowPolicy,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    let surface = document.surface(surface_id).ok_or_else(|| {
        operation_rejection(
            SurfaceMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        )
    })?;
    let source_window_id = surface
        .host_preferences()
        .first()
        .expect("valid Surface has a host preference")
        .window_id()
        .clone();
    if &source_window_id == target_window_id {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::MoveTargetUnchanged,
            format!("Surface {surface_id} is already primary in window {target_window_id}"),
        ));
    }
    if document.window(target_window_id).is_none() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UnknownWindow,
            format!("unknown participating window {target_window_id}"),
        ));
    }
    if !surface
        .host_preferences()
        .iter()
        .any(|preference| preference.window_id() == target_window_id)
    {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UndeclaredTargetWindow,
            format!("Surface {surface_id} does not declare window {target_window_id}"),
        ));
    }

    let source_order = ordered_members(document, &source_window_id);
    let former_index = source_order
        .iter()
        .position(|member| member == surface_id)
        .expect("source preference supplies membership");
    let mut target_order = ordered_members(document, target_window_id);
    target_order.retain(|member| member != surface_id);
    let insertion_index = usize::try_from(insertion_index).map_err(|_| invalid_index())?;
    if insertion_index > target_order.len() {
        return Err(invalid_index());
    }
    target_order.insert(insertion_index, surface_id.clone());

    let preferences = document
        .surface_mut(surface_id)
        .expect("Surface was checked")
        .host_preferences_mut();
    let target_preference_index = preferences
        .iter()
        .position(|preference| preference.window_id() == target_window_id)
        .expect("target declaration was checked");
    let target_preference = preferences.remove(target_preference_index);
    preferences.insert(0, target_preference);
    set_window_order(document, target_window_id, &target_order);

    let remaining_source_members = primary_members(document, &source_window_id);
    if remaining_source_members.is_empty() && empty_policy == EmptyWindowPolicy::Reject {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::EmptyWindowNotAllowed,
            format!("move would leave window {source_window_id} without a primary Surface"),
        ));
    }
    replace_removed_active(
        document,
        &source_window_id,
        surface_id,
        former_index,
        &remaining_source_members,
    );

    Ok(SurfaceMutationOutcome::SurfaceMoved {
        surface_id: surface_id.clone(),
        source_window_id,
        target_window_id: target_window_id.clone(),
        insertion_index: u32::try_from(insertion_index)
            .expect("input insertion index originated as u32"),
    })
}

pub(super) fn close_surface(
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    empty_policy: EmptyWindowPolicy,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    let surface = document.surface(surface_id).ok_or_else(|| {
        operation_rejection(
            SurfaceMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        )
    })?;
    let former_memberships = surface
        .host_preferences()
        .iter()
        .map(|preference| {
            let order =
                usize::try_from(preference.order()).expect("bounded Surface order fits usize");
            (preference.window_id().clone(), order)
        })
        .collect::<Vec<_>>();

    if empty_policy == EmptyWindowPolicy::Reject {
        for (window_id, _) in &former_memberships {
            if ordered_members(document, window_id).len() == 1 {
                return Err(operation_rejection(
                    SurfaceMutationRejectionCode::EmptyWindowNotAllowed,
                    format!("close would leave window {window_id} without a Surface"),
                ));
            }
        }
    }

    document
        .surfaces_mut()
        .retain(|surface| surface.id() != surface_id);
    for (window_id, former_index) in former_memberships {
        let remaining = ordered_members(document, &window_id);
        set_window_order(document, &window_id, &remaining);
        replace_removed_active(document, &window_id, surface_id, former_index, &remaining);
    }

    Ok(SurfaceMutationOutcome::SurfaceClosed {
        surface_id: surface_id.clone(),
    })
}

fn replace_removed_active(
    document: &mut SurfaceDocument,
    window_id: &WindowId,
    removed_surface_id: &SurfaceId,
    former_index: usize,
    remaining: &[SurfaceId],
) {
    let window = document
        .window_mut(window_id)
        .expect("valid host preference names participating window");
    if window.active_surface_id() == Some(removed_surface_id) {
        let replacement = remaining
            .get(former_index)
            .or_else(|| remaining.last())
            .cloned();
        window.set_active_surface_id(replacement);
    }
}

fn invalid_index() -> OperationRejection {
    operation_rejection(
        SurfaceMutationRejectionCode::InvalidInsertionIndex,
        "target insertion index exceeds target membership length",
    )
}
