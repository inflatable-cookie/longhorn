use std::collections::BTreeSet;

use crate::LayoutLimits;

use super::{
    DefinitionErrorCode, DefinitionRegistryError, LayoutSchemaDefinition, PanelDefinition,
    PanelInstancePolicy, PlacementSelector, error::definition_error,
};

pub(super) fn validate_schema(
    schema: &LayoutSchemaDefinition,
    limits: LayoutLimits,
) -> Result<(), DefinitionRegistryError> {
    if schema.regions.is_empty() {
        return Err(definition_error(
            DefinitionErrorCode::EmptySchema,
            format!("layout schema {} has no regions", schema.id),
        ));
    }
    if schema.regions.len() > limits.maximum_regions_per_schema() {
        return Err(definition_error(
            DefinitionErrorCode::TooManyRegions,
            format!(
                "layout schema {} has {} regions; limit is {}",
                schema.id,
                schema.regions.len(),
                limits.maximum_regions_per_schema()
            ),
        ));
    }
    if schema.sizing_slots.len() > limits.maximum_sizing_slots_per_schema() {
        return Err(definition_error(
            DefinitionErrorCode::TooManySizingSlots,
            format!(
                "layout schema {} has {} sizing slots; limit is {}",
                schema.id,
                schema.sizing_slots.len(),
                limits.maximum_sizing_slots_per_schema()
            ),
        ));
    }

    let mut region_ids = BTreeSet::new();
    let mut region_orders = BTreeSet::new();
    for region in &schema.regions {
        if !region_ids.insert(region.id.clone()) {
            return Err(definition_error(
                DefinitionErrorCode::DuplicateRegion,
                format!("layout schema {} repeats region {}", schema.id, region.id),
            ));
        }
        if !region_orders.insert(region.order) {
            return Err(definition_error(
                DefinitionErrorCode::DuplicateRegionOrder,
                format!(
                    "layout schema {} repeats region order {}",
                    schema.id, region.order
                ),
            ));
        }
    }

    let mut slot_ids = BTreeSet::new();
    let mut slot_orders = BTreeSet::new();
    for slot in &schema.sizing_slots {
        if !slot_ids.insert(slot.id.clone()) {
            return Err(definition_error(
                DefinitionErrorCode::DuplicateSizingSlot,
                format!(
                    "layout schema {} repeats sizing slot {}",
                    schema.id, slot.id
                ),
            ));
        }
        if !slot_orders.insert(slot.order) {
            return Err(definition_error(
                DefinitionErrorCode::DuplicateSizingSlotOrder,
                format!(
                    "layout schema {} repeats sizing-slot order {}",
                    schema.id, slot.order
                ),
            ));
        }
        if slot.minimum > slot.default || slot.default > slot.maximum {
            return Err(definition_error(
                DefinitionErrorCode::InvalidSizingBounds,
                format!(
                    "sizing slot {} requires minimum <= default <= maximum",
                    slot.id
                ),
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_panel<'a>(
    panel: &PanelDefinition,
    schemas: impl Iterator<Item = &'a LayoutSchemaDefinition> + Clone,
) -> Result<(), DefinitionRegistryError> {
    if panel.default_placements.is_empty() {
        return Err(definition_error(
            DefinitionErrorCode::EmptyDefaultPlacement,
            format!("panel definition {} has no default placement", panel.id),
        ));
    }
    if panel.allowed_placements.is_empty() {
        return Err(definition_error(
            DefinitionErrorCode::EmptyAllowedPlacement,
            format!("panel definition {} allows no placement", panel.id),
        ));
    }
    if panel
        .default_placements
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        != panel.default_placements.len()
    {
        return Err(definition_error(
            DefinitionErrorCode::DuplicateDefaultPlacement,
            format!("panel definition {} repeats a default selector", panel.id),
        ));
    }
    if panel
        .allowed_placements
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        != panel.allowed_placements.len()
    {
        return Err(definition_error(
            DefinitionErrorCode::DuplicateAllowedPlacement,
            format!("panel definition {} repeats an allowed selector", panel.id),
        ));
    }

    if let PanelInstancePolicy::Bounded {
        maximum_per_document,
        maximum_per_container,
    } = panel.instance_policy
    {
        if maximum_per_document == 0
            || maximum_per_container == 0
            || maximum_per_container > maximum_per_document
        {
            return Err(definition_error(
                DefinitionErrorCode::InvalidInstanceLimit,
                format!(
                    "panel definition {} has invalid bounded instance policy",
                    panel.id
                ),
            ));
        }
    }

    for selector in &panel.default_placements {
        if !selector_known(selector, schemas.clone()) {
            return Err(definition_error(
                DefinitionErrorCode::UnknownDefaultSelector,
                format!(
                    "panel definition {} has an unknown default selector",
                    panel.id
                ),
            ));
        }
    }
    for selector in &panel.allowed_placements {
        if !selector_known(selector, schemas.clone()) {
            return Err(definition_error(
                DefinitionErrorCode::UnknownAllowedSelector,
                format!(
                    "panel definition {} has an unknown allowed selector",
                    panel.id
                ),
            ));
        }
    }

    for schema in schemas {
        for region in schema.regions() {
            let is_default = panel
                .default_placements
                .iter()
                .any(|selector| selector.matches(region));
            let is_allowed = panel
                .allowed_placements
                .iter()
                .any(|selector| selector.matches(region));
            if is_default && !is_allowed {
                return Err(definition_error(
                    DefinitionErrorCode::DefaultPlacementNotAllowed,
                    format!(
                        "panel definition {} defaults to disallowed region {}",
                        panel.id,
                        region.id()
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn selector_known<'a>(
    selector: &PlacementSelector,
    schemas: impl Iterator<Item = &'a LayoutSchemaDefinition>,
) -> bool {
    schemas
        .flat_map(LayoutSchemaDefinition::regions)
        .any(|region| selector.matches(region))
}
