use std::collections::BTreeSet;

use longhorn_core::{PanelInstanceId, RegionId, SurfaceId};

use crate::{LayoutDefinitionRegistry, SurfaceDocument};

use super::{
    LayoutMutationOutcome, LayoutMutationRejectionCode, OperationRejection, checked_index,
    operation_rejection, outcome_index, panel_location, region_mut, remove_with_active_fallback,
};

pub(super) fn reorder_region(
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    region_id: &RegionId,
    panel_instance_ids: &[PanelInstanceId],
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let region = document
        .surface(surface_id)
        .ok_or_else(|| {
            operation_rejection(
                LayoutMutationRejectionCode::UnknownSurface,
                format!("unknown layout container {surface_id}"),
            )
        })?
        .region(region_id)
        .ok_or_else(|| {
            operation_rejection(
                LayoutMutationRejectionCode::UnknownRegion,
                format!("container {surface_id} has no region {region_id}"),
            )
        })?;

    let proposed: BTreeSet<_> = panel_instance_ids.iter().collect();
    if proposed.len() != panel_instance_ids.len() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::DuplicateReorderMember,
            format!("region {region_id} reorder repeats a panel instance"),
        ));
    }
    let current: BTreeSet<_> = region.panel_instance_ids().iter().collect();
    if proposed.iter().any(|id| !current.contains(id)) {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::ForeignReorderMember,
            format!("region {region_id} reorder contains a foreign panel instance"),
        ));
    }
    if panel_instance_ids.len() != region.panel_instance_ids().len() || proposed != current {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::IncompleteReorder,
            format!("region {region_id} reorder is not a complete permutation"),
        ));
    }

    *region_mut(document, surface_id, region_id)?.panel_instance_ids_mut() =
        panel_instance_ids.to_vec();

    Ok(LayoutMutationOutcome::RegionReordered {
        surface_id: surface_id.clone(),
        region_id: region_id.clone(),
        panel_instance_ids: panel_instance_ids.to_vec(),
    })
}

pub(super) fn move_panel(
    registry: &LayoutDefinitionRegistry,
    document: &mut SurfaceDocument,
    panel_instance_id: &PanelInstanceId,
    target_surface_id: &SurfaceId,
    target_region_id: &RegionId,
    insertion_index: u32,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let instance = document.panel_instance(panel_instance_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownPanelInstance,
            format!("unknown panel instance {panel_instance_id}"),
        )
    })?;
    let definition = registry
        .panel_definition(instance.definition_id())
        .expect("current document validation established the definition");
    if !definition.is_movable() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::PanelNotMovable,
            format!("panel instance {panel_instance_id} is not movable"),
        ));
    }
    let source = panel_location(document, panel_instance_id)?;
    if source.surface_id == *target_surface_id && source.region_id == *target_region_id {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::MoveTargetUnchanged,
            "same-region position changes require a complete reorder command",
        ));
    }
    let target_container = document.surface(target_surface_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownSurface,
            format!("unknown layout container {target_surface_id}"),
        )
    })?;
    let target_region = target_container.region(target_region_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownRegion,
            format!("container {target_surface_id} has no region {target_region_id}"),
        )
    })?;
    let allowed = registry
        .is_panel_allowed_in(
            target_container.schema_id(),
            definition.id(),
            target_region_id,
        )
        .expect("validated registry lookup uses known schema, definition, and region");
    if !allowed {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::PanelPlacementNotAllowed,
            format!(
                "panel definition {} is not allowed in region {target_region_id}",
                definition.id()
            ),
        ));
    }
    let target_index = checked_index(insertion_index, target_region.panel_instance_ids().len())?;

    let source_region = region_mut(document, &source.surface_id, &source.region_id)?;
    remove_with_active_fallback(source_region, panel_instance_id, source.index);
    let target_region = region_mut(document, target_surface_id, target_region_id)?;
    target_region
        .panel_instance_ids_mut()
        .insert(target_index, panel_instance_id.clone());
    target_region.set_active_panel_instance_id(Some(panel_instance_id.clone()));

    Ok(LayoutMutationOutcome::PanelMoved {
        panel_instance_id: panel_instance_id.clone(),
        source_surface_id: source.surface_id,
        source_region_id: source.region_id,
        former_index: outcome_index(source.index),
        target_surface_id: target_surface_id.clone(),
        target_region_id: target_region_id.clone(),
        insertion_index,
    })
}
