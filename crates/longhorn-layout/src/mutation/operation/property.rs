use longhorn_core::{LayoutContainerId, PanelInstanceId, RegionId, SizingSlotId};

use crate::{LayoutDefinitionRegistry, LayoutDocument, LayoutRatio};

use super::{
    LayoutMutationOutcome, LayoutMutationRejectionCode, OperationRejection, operation_rejection,
    panel_location, region_mut,
};

pub(super) fn activate_panel(
    document: &mut LayoutDocument,
    panel_instance_id: &PanelInstanceId,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    if document.panel_instance(panel_instance_id).is_none() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::UnknownPanelInstance,
            format!("unknown panel instance {panel_instance_id}"),
        ));
    }
    let location = panel_location(document, panel_instance_id)?;
    let region = region_mut(document, &location.container_id, &location.region_id)?;
    let previous_active_panel_instance_id = region.active_panel_instance_id().cloned();
    region.set_active_panel_instance_id(Some(panel_instance_id.clone()));

    Ok(LayoutMutationOutcome::PanelActivated {
        panel_instance_id: panel_instance_id.clone(),
        container_id: location.container_id,
        region_id: location.region_id,
        previous_active_panel_instance_id,
    })
}

pub(super) fn set_sizing_slot(
    registry: &LayoutDefinitionRegistry,
    document: &mut LayoutDocument,
    container_id: &LayoutContainerId,
    sizing_slot_id: &SizingSlotId,
    ratio: LayoutRatio,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let container = document.container(container_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownContainer,
            format!("unknown layout container {container_id}"),
        )
    })?;
    let schema = registry
        .schema(container.schema_id())
        .expect("current document validation established the schema");
    let definition = schema.sizing_slot(sizing_slot_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownSizingSlot,
            format!("container {container_id} has no sizing slot {sizing_slot_id}"),
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
    let previous_ratio = container
        .sizing_slot(sizing_slot_id)
        .expect("complete sizing state was validated")
        .ratio();
    document
        .container_mut(container_id)
        .expect("container existence was checked")
        .sizing_slot_mut(sizing_slot_id)
        .expect("complete sizing state was validated")
        .set_ratio(ratio);

    Ok(LayoutMutationOutcome::SizingSlotSet {
        container_id: container_id.clone(),
        sizing_slot_id: sizing_slot_id.clone(),
        previous_ratio,
        committed_ratio: ratio,
    })
}

pub(super) fn set_region_collapsed(
    registry: &LayoutDefinitionRegistry,
    document: &mut LayoutDocument,
    container_id: &LayoutContainerId,
    region_id: &RegionId,
    collapsed: bool,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    let container = document.container(container_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownContainer,
            format!("unknown layout container {container_id}"),
        )
    })?;
    let schema = registry
        .schema(container.schema_id())
        .expect("current document validation established the schema");
    let definition = schema.region(region_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownRegion,
            format!("container {container_id} has no region {region_id}"),
        )
    })?;
    if !definition.is_collapsible() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::UnsupportedCollapse,
            format!("region {region_id} does not support collapse state"),
        ));
    }
    let region = container
        .region(region_id)
        .expect("complete region state was validated");
    let previous_collapsed = region.collapsed().unwrap_or(false);
    region_mut(document, container_id, region_id)?.set_collapsed(collapsed);

    Ok(LayoutMutationOutcome::RegionCollapsedSet {
        container_id: container_id.clone(),
        region_id: region_id.clone(),
        previous_collapsed,
        committed_collapsed: collapsed,
    })
}
