use longhorn_core::{LayoutContainerId, PanelDefinitionId, PanelInstanceId, RegionId};

use crate::{LayoutDefinitionRegistry, LayoutDocument, PanelInstance};

use super::{
    LayoutMutationOutcome, LayoutMutationRejectionCode, OperationRejection, checked_index,
    operation_rejection, outcome_index, panel_location, region_mut, remove_with_active_fallback,
};

pub(super) fn create_panel(
    registry: &LayoutDefinitionRegistry,
    document: &mut LayoutDocument,
    panel_instance_id: &PanelInstanceId,
    panel_definition_id: &PanelDefinitionId,
    container_id: &LayoutContainerId,
    region_id: &RegionId,
    insertion_index: u32,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    if document.panel_instance(panel_instance_id).is_some() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::DuplicatePanelInstance,
            format!("panel instance {panel_instance_id} already exists"),
        ));
    }
    let definition = registry
        .panel_definition(panel_definition_id)
        .ok_or_else(|| {
            operation_rejection(
                LayoutMutationRejectionCode::UnknownPanelDefinition,
                format!("unknown panel definition {panel_definition_id}"),
            )
        })?;
    let container = document.container(container_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownContainer,
            format!("unknown layout container {container_id}"),
        )
    })?;
    let region = container.region(region_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownRegion,
            format!("container {container_id} has no region {region_id}"),
        )
    })?;
    let allowed = registry
        .is_panel_allowed_in(container.schema_id(), definition.id(), region_id)
        .expect("validated registry lookup uses known schema, definition, and region");
    if !allowed {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::PanelPlacementNotAllowed,
            format!("panel definition {panel_definition_id} is not allowed in region {region_id}"),
        ));
    }
    let index = checked_index(insertion_index, region.panel_instance_ids().len())?;

    let region = region_mut(document, container_id, region_id)?;
    region
        .panel_instance_ids_mut()
        .insert(index, panel_instance_id.clone());
    region.set_active_panel_instance_id(Some(panel_instance_id.clone()));
    document.panel_instances_mut().push(PanelInstance::new(
        panel_instance_id.clone(),
        panel_definition_id.clone(),
    ));

    Ok(LayoutMutationOutcome::PanelCreated {
        panel_instance_id: panel_instance_id.clone(),
        container_id: container_id.clone(),
        region_id: region_id.clone(),
        insertion_index,
    })
}

pub(super) fn close_panel(
    registry: &LayoutDefinitionRegistry,
    document: &mut LayoutDocument,
    panel_instance_id: &PanelInstanceId,
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
    if !definition.is_closeable() {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::PanelNotCloseable,
            format!("panel instance {panel_instance_id} is not closeable"),
        ));
    }
    let location = panel_location(document, panel_instance_id)?;
    let region = region_mut(document, &location.container_id, &location.region_id)?;
    remove_with_active_fallback(region, panel_instance_id, location.index);
    document
        .panel_instances_mut()
        .retain(|instance| instance.id() != panel_instance_id);

    Ok(LayoutMutationOutcome::PanelClosed {
        panel_instance_id: panel_instance_id.clone(),
        container_id: location.container_id,
        region_id: location.region_id,
        former_index: outcome_index(location.index),
    })
}
