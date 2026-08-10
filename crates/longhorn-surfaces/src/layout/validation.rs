use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{PanelDefinitionId, PanelInstanceId};

use crate::{LayoutDefinitionRegistry, PanelInstancePolicy, SurfaceDocument};

mod error;

use error::validation_error;
pub use error::{LayoutValidationCode, LayoutValidationError};

/// Validates all current-schema layout document invariants.
pub fn validate_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<(), LayoutValidationError> {
    let limits = registry.limits();
    if document.surfaces().len() > limits.maximum_containers() {
        return Err(validation_error(
            LayoutValidationCode::TooManyContainers,
            format!(
                "{} containers exceed limit {}",
                document.surfaces().len(),
                limits.maximum_containers()
            ),
        ));
    }
    if document.panel_instances().len() > limits.maximum_panel_instances() {
        return Err(validation_error(
            LayoutValidationCode::TooManyPanelInstances,
            format!(
                "{} panel instances exceed limit {}",
                document.panel_instances().len(),
                limits.maximum_panel_instances()
            ),
        ));
    }

    let mut instances = BTreeMap::new();
    let mut totals_by_definition = BTreeMap::<PanelDefinitionId, usize>::new();
    for instance in document.panel_instances() {
        if registry
            .panel_definition(instance.definition_id())
            .is_none()
        {
            return Err(validation_error(
                LayoutValidationCode::UnknownPanelDefinition,
                format!(
                    "panel instance {} uses unknown definition {}",
                    instance.id(),
                    instance.definition_id()
                ),
            ));
        }
        if instances.insert(instance.id(), instance).is_some() {
            return Err(validation_error(
                LayoutValidationCode::DuplicatePanelInstance,
                format!("duplicate panel instance {}", instance.id()),
            ));
        }
        *totals_by_definition
            .entry(instance.definition_id().clone())
            .or_default() += 1;
    }

    let mut surface_ids = BTreeSet::new();
    let mut placed_instances = BTreeSet::<PanelInstanceId>::new();
    let mut counts_by_container = BTreeMap::<_, BTreeMap<PanelDefinitionId, usize>>::new();

    for container in document.surfaces() {
        if !surface_ids.insert(container.id()) {
            return Err(validation_error(
                LayoutValidationCode::DuplicateContainer,
                format!("duplicate layout container {}", container.id()),
            ));
        }
        let schema = registry.schema(container.schema_id()).ok_or_else(|| {
            validation_error(
                LayoutValidationCode::UnknownSchema,
                format!(
                    "container {} uses unknown schema {}",
                    container.id(),
                    container.schema_id()
                ),
            )
        })?;

        let mut region_ids = BTreeSet::new();
        let mut container_definition_counts = BTreeMap::<PanelDefinitionId, usize>::new();
        for region in container.regions() {
            if !region_ids.insert(region.region_id()) {
                return Err(validation_error(
                    LayoutValidationCode::DuplicateRegionState,
                    format!(
                        "container {} repeats region {}",
                        container.id(),
                        region.region_id()
                    ),
                ));
            }
            let region_definition = schema.region(region.region_id()).ok_or_else(|| {
                validation_error(
                    LayoutValidationCode::UnknownRegion,
                    format!(
                        "container {} has unknown region {}",
                        container.id(),
                        region.region_id()
                    ),
                )
            })?;
            if region.panel_instance_ids().len() > limits.maximum_panel_instances_per_region() {
                return Err(validation_error(
                    LayoutValidationCode::TooManyPanelInstancesInRegion,
                    format!(
                        "container {} region {} has {} panel instances; limit is {}",
                        container.id(),
                        region.region_id(),
                        region.panel_instance_ids().len(),
                        limits.maximum_panel_instances_per_region()
                    ),
                ));
            }
            if !region_definition.is_collapsible() && region.collapsed().is_some() {
                return Err(validation_error(
                    LayoutValidationCode::UnsupportedCollapseState,
                    format!(
                        "region {} does not support collapse state",
                        region.region_id()
                    ),
                ));
            }
            if let Some(active) = region.active_panel_instance_id()
                && !region.panel_instance_ids().contains(active)
            {
                return Err(validation_error(
                    LayoutValidationCode::ActivePanelNotInRegion,
                    format!(
                        "active panel {} is not in region {}",
                        active,
                        region.region_id()
                    ),
                ));
            }

            for panel_instance_id in region.panel_instance_ids() {
                if !placed_instances.insert(panel_instance_id.clone()) {
                    return Err(validation_error(
                        LayoutValidationCode::DuplicatePanelPlacement,
                        format!("panel instance {panel_instance_id} is placed more than once"),
                    ));
                }
                let instance = instances.get(panel_instance_id).ok_or_else(|| {
                    validation_error(
                        LayoutValidationCode::UnknownPanelInstance,
                        format!(
                            "region {} references unknown panel instance {}",
                            region.region_id(),
                            panel_instance_id
                        ),
                    )
                })?;
                let allowed = registry
                    .is_panel_allowed_in(
                        container.schema_id(),
                        instance.definition_id(),
                        region.region_id(),
                    )
                    .map_err(|error| {
                        validation_error(
                            LayoutValidationCode::UnknownDefinitionReference,
                            error.to_string(),
                        )
                    })?;
                if !allowed {
                    return Err(validation_error(
                        LayoutValidationCode::PanelPlacementNotAllowed,
                        format!(
                            "panel instance {} is not allowed in region {}",
                            panel_instance_id,
                            region.region_id()
                        ),
                    ));
                }
                *container_definition_counts
                    .entry(instance.definition_id().clone())
                    .or_default() += 1;
            }
        }
        if region_ids.len() != schema.regions().len()
            || schema
                .regions()
                .iter()
                .any(|region| !region_ids.contains(region.id()))
        {
            return Err(validation_error(
                LayoutValidationCode::IncompleteRegionState,
                format!(
                    "container {} does not contain every schema region exactly once",
                    container.id()
                ),
            ));
        }

        let mut sizing_slot_ids = BTreeSet::new();
        for slot in container.sizing_slots() {
            if !sizing_slot_ids.insert(slot.sizing_slot_id()) {
                return Err(validation_error(
                    LayoutValidationCode::DuplicateSizingSlotState,
                    format!(
                        "container {} repeats sizing slot {}",
                        container.id(),
                        slot.sizing_slot_id()
                    ),
                ));
            }
            let definition = schema.sizing_slot(slot.sizing_slot_id()).ok_or_else(|| {
                validation_error(
                    LayoutValidationCode::UnknownSizingSlot,
                    format!(
                        "container {} has unknown sizing slot {}",
                        container.id(),
                        slot.sizing_slot_id()
                    ),
                )
            })?;
            if !definition.contains(slot.ratio()) {
                return Err(validation_error(
                    LayoutValidationCode::SizingRatioOutOfBounds,
                    format!(
                        "sizing slot {} ratio {} is outside {}..={}",
                        slot.sizing_slot_id(),
                        slot.ratio().millionths(),
                        definition.minimum().millionths(),
                        definition.maximum().millionths()
                    ),
                ));
            }
        }
        if sizing_slot_ids.len() != schema.sizing_slots().len()
            || schema
                .sizing_slots()
                .iter()
                .any(|slot| !sizing_slot_ids.contains(slot.id()))
        {
            return Err(validation_error(
                LayoutValidationCode::IncompleteSizingSlotState,
                format!(
                    "container {} does not contain every sizing slot exactly once",
                    container.id()
                ),
            ));
        }

        counts_by_container.insert(container.id(), container_definition_counts);
    }

    if placed_instances.len() != instances.len() {
        return Err(validation_error(
            LayoutValidationCode::UnplacedPanelInstance,
            "every panel instance must be placed exactly once",
        ));
    }

    for (definition_id, total) in totals_by_definition {
        let definition = registry
            .panel_definition(&definition_id)
            .expect("definition existence was validated");
        match definition.instance_policy() {
            PanelInstancePolicy::Singleton if total > 1 => {
                return Err(instance_limit_error(&definition_id, total, 1, "document"));
            }
            PanelInstancePolicy::OnePerContainer => {
                for counts in counts_by_container.values() {
                    let actual = counts.get(&definition_id).copied().unwrap_or_default();
                    if actual > 1 {
                        return Err(instance_limit_error(&definition_id, actual, 1, "container"));
                    }
                }
            }
            PanelInstancePolicy::Bounded {
                maximum_per_document,
                maximum_per_container,
            } => {
                if total > maximum_per_document as usize {
                    return Err(instance_limit_error(
                        &definition_id,
                        total,
                        maximum_per_document as usize,
                        "document",
                    ));
                }
                for counts in counts_by_container.values() {
                    let actual = counts.get(&definition_id).copied().unwrap_or_default();
                    if actual > maximum_per_container as usize {
                        return Err(instance_limit_error(
                            &definition_id,
                            actual,
                            maximum_per_container as usize,
                            "container",
                        ));
                    }
                }
            }
            PanelInstancePolicy::Singleton | PanelInstancePolicy::Multiple => {}
        }
    }

    Ok(())
}

/// Returns one canonical structural ordering for a valid current-schema document.
pub fn normalize_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<SurfaceDocument, LayoutValidationError> {
    validate_document(registry, document)?;
    let mut normalized = document.clone();

    normalized
        .panel_instances_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));
    normalized
        .surfaces_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));

    for container in normalized.surfaces_mut() {
        let schema = registry
            .schema(container.schema_id())
            .expect("schema existence was validated");
        let region_order: BTreeMap<_, _> = schema
            .regions()
            .iter()
            .enumerate()
            .map(|(index, region)| (region.id(), index))
            .collect();
        let sizing_order: BTreeMap<_, _> = schema
            .sizing_slots()
            .iter()
            .enumerate()
            .map(|(index, slot)| (slot.id(), index))
            .collect();

        container
            .regions_mut()
            .sort_by_key(|region| region_order[region.region_id()]);
        for region in container.regions_mut() {
            let definition = schema
                .region(region.region_id())
                .expect("region existence was validated");
            region.normalize_active();
            region.normalize_collapse(definition.is_collapsible());
        }
        container
            .sizing_slots_mut()
            .sort_by_key(|slot| sizing_order[slot.sizing_slot_id()]);
    }

    validate_document(registry, &normalized)?;
    Ok(normalized)
}

/// Requires a valid document to already use canonical normalized form.
pub fn validate_normalized_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<(), LayoutValidationError> {
    let normalized = normalize_document(registry, document)?;
    if normalized == *document {
        Ok(())
    } else {
        Err(validation_error(
            LayoutValidationCode::NonCanonicalDocument,
            "layout document is valid but not canonically normalized",
        ))
    }
}

fn instance_limit_error(
    definition_id: &PanelDefinitionId,
    actual: usize,
    maximum: usize,
    scope: &str,
) -> LayoutValidationError {
    validation_error(
        LayoutValidationCode::InstancePolicyExceeded,
        format!(
            "panel definition {definition_id} has {actual} instances per {scope}; maximum is {maximum}"
        ),
    )
}
