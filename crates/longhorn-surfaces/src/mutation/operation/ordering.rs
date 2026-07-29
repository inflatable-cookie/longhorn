use std::collections::BTreeSet;

use longhorn_core::{SurfaceId, WindowId};

use crate::SurfaceDocument;

use super::{
    OperationRejection, SurfaceMutationOutcome, SurfaceMutationRejectionCode, operation_rejection,
};

pub(super) fn activate_surface(
    document: &mut SurfaceDocument,
    window_id: &WindowId,
    surface_id: &SurfaceId,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    if document.window(window_id).is_none() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UnknownWindow,
            format!("unknown participating window {window_id}"),
        ));
    }
    if document.surface(surface_id).is_none() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        ));
    }
    if !is_member(document, window_id, surface_id) {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UndeclaredTargetWindow,
            format!("Surface {surface_id} is not declared in window {window_id}"),
        ));
    }

    let window = document
        .window_mut(window_id)
        .expect("participating window was checked");
    let previous_active_surface_id = window.active_surface_id().cloned();
    window.set_active_surface_id(Some(surface_id.clone()));
    Ok(SurfaceMutationOutcome::SurfaceActivated {
        window_id: window_id.clone(),
        surface_id: surface_id.clone(),
        previous_active_surface_id,
    })
}

pub(super) fn reorder_window(
    document: &mut SurfaceDocument,
    window_id: &WindowId,
    surface_ids: &[SurfaceId],
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    if document.window(window_id).is_none() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::UnknownWindow,
            format!("unknown participating window {window_id}"),
        ));
    }
    let members = ordered_members(document, window_id);
    if members.len() != surface_ids.len() {
        return Err(operation_rejection(
            SurfaceMutationRejectionCode::IncompleteReorder,
            format!(
                "window {window_id} requires {} members; request supplied {}",
                members.len(),
                surface_ids.len()
            ),
        ));
    }
    let mut requested = BTreeSet::new();
    for surface_id in surface_ids {
        if !requested.insert(surface_id) {
            return Err(operation_rejection(
                SurfaceMutationRejectionCode::DuplicateReorderMember,
                format!("Surface {surface_id} is repeated"),
            ));
        }
        if !members.contains(surface_id) {
            return Err(operation_rejection(
                SurfaceMutationRejectionCode::ForeignReorderMember,
                format!("Surface {surface_id} is not declared in window {window_id}"),
            ));
        }
    }
    set_window_order(document, window_id, surface_ids);
    Ok(SurfaceMutationOutcome::WindowReordered {
        window_id: window_id.clone(),
        surface_ids: surface_ids.to_vec(),
    })
}

pub(super) fn ordered_members(document: &SurfaceDocument, window_id: &WindowId) -> Vec<SurfaceId> {
    let mut members = document
        .surfaces()
        .iter()
        .filter_map(|surface| {
            surface
                .host_preferences()
                .iter()
                .find(|preference| preference.window_id() == window_id)
                .map(|preference| (preference.order(), surface.id().clone()))
        })
        .collect::<Vec<_>>();
    members.sort_by_key(|(order, _)| *order);
    members.into_iter().map(|(_, id)| id).collect()
}

pub(super) fn primary_members(document: &SurfaceDocument, window_id: &WindowId) -> Vec<SurfaceId> {
    let all_members = ordered_members(document, window_id);
    all_members
        .into_iter()
        .filter(|surface_id| {
            document
                .surface(surface_id)
                .and_then(|surface| surface.host_preferences().first())
                .is_some_and(|preference| preference.window_id() == window_id)
        })
        .collect()
}

pub(super) fn set_window_order(
    document: &mut SurfaceDocument,
    window_id: &WindowId,
    surface_ids: &[SurfaceId],
) {
    for (index, surface_id) in surface_ids.iter().enumerate() {
        let order = u32::try_from(index).expect("bounded Surface count fits u32");
        let surface = document
            .surface_mut(surface_id)
            .expect("ordered Surface member exists");
        let preference = surface
            .host_preferences_mut()
            .iter_mut()
            .find(|preference| preference.window_id() == window_id)
            .expect("ordered Surface declares window");
        preference.set_order(order);
    }
}

fn is_member(document: &SurfaceDocument, window_id: &WindowId, surface_id: &SurfaceId) -> bool {
    document.surface(surface_id).is_some_and(|surface| {
        surface
            .host_preferences()
            .iter()
            .any(|preference| preference.window_id() == window_id)
    })
}
