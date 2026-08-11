use longhorn_core::{PanelInstanceId, RegionId, SizingSlotId, SurfaceId};

use crate::{LayoutDefinitionRegistry, LayoutRatio, SurfaceDocument};

use super::{
    LayoutMutationOutcome, LayoutMutationRejectionCode, OperationRejection, operation_rejection,
    panel_location, region_mut,
};

pub(super) fn activate_panel(
    document: &mut SurfaceDocument,
    panel_instance_id: &PanelInstanceId,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    if document.panel_instance(panel_instance_id).is_none() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::UnknownPanelInstance,
            format!("unknown panel instance {panel_instance_id}"),
        ));
    }
    let location = panel_location(document, panel_instance_id)?;
    let region = region_mut(document, &location.surface_id, &location.region_id)?;
    let previous_active_panel_instance_id = region.active_panel_instance_id().cloned();
    region.set_active_panel_instance_id(Some(panel_instance_id.clone()));

    Ok(LayoutMutationOutcome::PanelActivated {
        panel_instance_id: panel_instance_id.clone(),
        surface_id: location.surface_id,
        region_id: location.region_id,
        previous_active_panel_instance_id,
    })
}

pub(super) fn set_sizing_slot(
    registry: &LayoutDefinitionRegistry,
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    sizing_slot_id: &SizingSlotId,
    ratio: LayoutRatio,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let surface = document.surface(surface_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        )
    })?;
    let schema = registry
        .schema(surface.schema_id())
        .expect("current document validation established the schema");
    let definition = schema.sizing_slot(sizing_slot_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownSizingSlot,
            format!("Surface {surface_id} has no sizing slot {sizing_slot_id}"),
        )
    })?;
    if !definition.contains(ratio) {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::InvalidSizingRatio,
            format!(
                "sizing slot {sizing_slot_id} rejects ratio {}",
                ratio.millionths()
            ),
        ));
    }
    let previous_ratio = surface
        .sizing_slot(sizing_slot_id)
        .expect("complete sizing state was validated")
        .ratio();
    document
        .surface_mut(surface_id)
        .expect("Surface existence was checked")
        .sizing_slot_mut(sizing_slot_id)
        .expect("complete sizing state was validated")
        .set_ratio(ratio);

    Ok(LayoutMutationOutcome::SizingSlotSet {
        surface_id: surface_id.clone(),
        sizing_slot_id: sizing_slot_id.clone(),
        previous_ratio,
        committed_ratio: ratio,
    })
}

pub(super) fn set_region_collapsed(
    registry: &LayoutDefinitionRegistry,
    document: &mut SurfaceDocument,
    surface_id: &SurfaceId,
    region_id: &RegionId,
    collapsed: bool,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let surface = document.surface(surface_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownSurface,
            format!("unknown Surface {surface_id}"),
        )
    })?;
    let schema = registry
        .schema(surface.schema_id())
        .expect("current document validation established the schema");
    let definition = schema.region(region_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownRegion,
            format!("Surface {surface_id} has no region {region_id}"),
        )
    })?;
    if !definition.is_collapsible() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::UnsupportedCollapse,
            format!("region {region_id} does not support collapse state"),
        ));
    }
    let region = surface
        .region(region_id)
        .expect("complete region state was validated");
    let previous_collapsed = region.collapsed().unwrap_or(false);
    region_mut(document, surface_id, region_id)?.set_collapsed(collapsed);

    Ok(LayoutMutationOutcome::RegionCollapsedSet {
        surface_id: surface_id.clone(),
        region_id: region_id.clone(),
        previous_collapsed,
        committed_collapsed: collapsed,
    })
}
