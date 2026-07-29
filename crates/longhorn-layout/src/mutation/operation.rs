use longhorn_core::{LayoutContainerId, PanelInstanceId, RegionId};

use crate::{LayoutDefinitionRegistry, LayoutDocument, LayoutValidationCode, RegionState};

use super::{
    LayoutMutationCommand, LayoutMutationOutcome, LayoutMutationRejectionCode,
    error::{OperationRejection, operation_rejection},
};

mod create_close;
mod property;
mod reorder_move;

/// Existing location of one globally unique panel instance.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelLocation {
    container_id: LayoutContainerId,
    region_id: RegionId,
    index: usize,
}

pub(super) fn apply_command(
    registry: &LayoutDefinitionRegistry,
    document: &mut LayoutDocument,
    command: &LayoutMutationCommand,
) -> Result<LayoutMutationOutcome, OperationRejection> {
    match command {
        LayoutMutationCommand::CreatePanel {
            panel_instance_id,
            panel_definition_id,
            container_id,
            region_id,
            insertion_index,
        } => create_close::create_panel(
            registry,
            document,
            panel_instance_id,
            panel_definition_id,
            container_id,
            region_id,
            *insertion_index,
        ),
        LayoutMutationCommand::ClosePanel { panel_instance_id } => {
            create_close::close_panel(registry, document, panel_instance_id)
        }
        LayoutMutationCommand::ActivatePanel { panel_instance_id } => {
            property::activate_panel(document, panel_instance_id)
        }
        LayoutMutationCommand::ReorderRegion {
            container_id,
            region_id,
            panel_instance_ids,
        } => reorder_move::reorder_region(document, container_id, region_id, panel_instance_ids),
        LayoutMutationCommand::MovePanel {
            panel_instance_id,
            target_container_id,
            target_region_id,
            insertion_index,
        } => reorder_move::move_panel(
            registry,
            document,
            panel_instance_id,
            target_container_id,
            target_region_id,
            *insertion_index,
        ),
        LayoutMutationCommand::SetSizingSlot {
            container_id,
            sizing_slot_id,
            ratio,
        } => property::set_sizing_slot(registry, document, container_id, sizing_slot_id, *ratio),
        LayoutMutationCommand::SetRegionCollapsed {
            container_id,
            region_id,
            collapsed,
        } => {
            property::set_region_collapsed(registry, document, container_id, region_id, *collapsed)
        }
    }
}

pub(super) fn map_candidate_validation(code: LayoutValidationCode) -> LayoutMutationRejectionCode {
    match code {
        LayoutValidationCode::InstancePolicyExceeded => {
            LayoutMutationRejectionCode::InstancePolicyExceeded
        }
        LayoutValidationCode::PanelPlacementNotAllowed => {
            LayoutMutationRejectionCode::PanelPlacementNotAllowed
        }
        LayoutValidationCode::SizingRatioOutOfBounds => {
            LayoutMutationRejectionCode::InvalidSizingRatio
        }
        LayoutValidationCode::UnsupportedCollapseState => {
            LayoutMutationRejectionCode::UnsupportedCollapse
        }
        _ => LayoutMutationRejectionCode::InvalidCandidate,
    }
}

fn panel_location(
    document: &LayoutDocument,
    panel_instance_id: &PanelInstanceId,
) -> Result<PanelLocation, OperationRejection> {
    for container in document.containers() {
        for region in container.regions() {
            if let Some(index) = region
                .panel_instance_ids()
                .iter()
                .position(|id| id == panel_instance_id)
            {
                return Ok(PanelLocation {
                    container_id: container.id().clone(),
                    region_id: region.region_id().clone(),
                    index,
                });
            }
        }
    }
    Err(operation_rejection(
        LayoutMutationRejectionCode::UnknownPanelInstance,
        format!("unknown panel instance {panel_instance_id}"),
    ))
}

fn region_mut<'a>(
    document: &'a mut LayoutDocument,
    container_id: &LayoutContainerId,
    region_id: &RegionId,
) -> Result<&'a mut RegionState, OperationRejection> {
    let container = document.container_mut(container_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownContainer,
            format!("unknown layout container {container_id}"),
        )
    })?;
    container.region_mut(region_id).ok_or_else(|| {
        operation_rejection(
            LayoutMutationRejectionCode::UnknownRegion,
            format!("container {container_id} has no region {region_id}"),
        )
    })
}

fn checked_index(insertion_index: u32, maximum: usize) -> Result<usize, OperationRejection> {
    let index = insertion_index as usize;
    if index > maximum {
        return Err(operation_rejection(
            LayoutMutationRejectionCode::InvalidInsertionIndex,
            format!("insertion index {insertion_index} exceeds target length {maximum}"),
        ));
    }
    Ok(index)
}

fn remove_with_active_fallback(
    region: &mut RegionState,
    panel_instance_id: &PanelInstanceId,
    index: usize,
) {
    let removed_active = region.active_panel_instance_id() == Some(panel_instance_id);
    region.panel_instance_ids_mut().remove(index);
    if removed_active {
        let fallback = region
            .panel_instance_ids()
            .get(index)
            .or_else(|| region.panel_instance_ids().last())
            .cloned();
        region.set_active_panel_instance_id(fallback);
    }
}

fn outcome_index(index: usize) -> u32 {
    u32::try_from(index).expect("validated panel count fits u32")
}
