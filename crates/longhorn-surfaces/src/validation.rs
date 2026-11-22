use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{PanelDefinitionId, PanelInstanceId, SurfaceId, WindowId};

use crate::{LayoutDefinitionRegistry, PanelInstancePolicy, SurfaceDocument};

/// Stable category for invalid current Surface document state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceValidationCode {
    /// Surface count exceeded configured limits.
    TooManySurfaces,
    /// Participating-window count exceeded configured limits.
    TooManyWindows,
    /// Panel-instance count exceeded configured limits.
    TooManyPanelInstances,
    /// A Surface had no declared candidate host.
    MissingHostPreference,
    /// One Surface exceeded its candidate-host limit.
    TooManyHostPreferences,
    /// A label exceeded its configured UTF-8 byte limit.
    LabelTooLong,
    /// A Surface id appeared more than once.
    DuplicateSurface,
    /// A participating-window id appeared more than once.
    DuplicateWindow,
    /// A host preference referenced a non-participating window.
    UnknownHostWindow,
    /// A Surface repeated one candidate window.
    DuplicateHostPreference,
    /// Two Surfaces declared the same order in one candidate window.
    DuplicateHostOrder,
    /// Candidate-window order was not a complete zero-based sequence.
    IncompleteHostOrder,
    /// A participating window selected a Surface outside its declared members.
    ActiveSurfaceNotHosted,
    /// A panel-instance id appeared more than once.
    DuplicatePanelInstance,
    /// A Surface referenced an unknown schema.
    UnknownSchema,
    /// A panel instance referenced an unknown definition.
    UnknownPanelDefinition,
    /// A region id appeared more than once in one Surface.
    DuplicateRegionState,
    /// A region was not part of its Surface schema.
    UnknownRegion,
    /// Complete schema region state was missing.
    IncompleteRegionState,
    /// A region exceeded its panel-instance limit.
    TooManyPanelInstancesInRegion,
    /// Collapse state appeared on an unsupported region.
    UnsupportedCollapseState,
    /// The active panel did not belong to its region.
    ActivePanelNotInRegion,
    /// One panel instance appeared in more than one placement.
    DuplicatePanelPlacement,
    /// A region referenced an unknown panel instance.
    UnknownPanelInstance,
    /// Panel policy rejected its current region.
    PanelPlacementNotAllowed,
    /// A sizing-slot id appeared more than once in one Surface.
    DuplicateSizingSlotState,
    /// A sizing slot was not part of its Surface schema.
    UnknownSizingSlot,
    /// Complete schema sizing-slot state was missing.
    IncompleteSizingSlotState,
    /// A sizing ratio fell outside registered bounds.
    SizingRatioOutOfBounds,
    /// A panel instance had no placement.
    UnplacedPanelInstance,
    /// Registered panel instance-count policy was exceeded.
    InstancePolicyExceeded,
    /// Valid state was not in canonical normalized form.
    NonCanonicalDocument,
}

/// Invalid current Surface document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceValidationError {
    code: SurfaceValidationCode,
    detail: String,
}

impl SurfaceValidationError {
    /// Returns the stable validation category.
    #[must_use]
    pub const fn code(&self) -> SurfaceValidationCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SurfaceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceValidationError {}

/// Validates all current-schema Surface and layout document invariants.
///
/// # Errors
///
/// Returns [`SurfaceValidationError`] naming the first violated topology,
/// limit, schema, placement, or instance-policy invariant.
pub fn validate_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<(), SurfaceValidationError> {
    let limits = registry.limits();
    if document.surfaces().len() > limits.maximum_surfaces() {
        return Err(validation_error(
            SurfaceValidationCode::TooManySurfaces,
            format!(
                "{} Surfaces exceed limit {}",
                document.surfaces().len(),
                limits.maximum_surfaces()
            ),
        ));
    }
    if document.windows().len() > limits.maximum_windows() {
        return Err(validation_error(
            SurfaceValidationCode::TooManyWindows,
            format!(
                "{} participating windows exceed limit {}",
                document.windows().len(),
                limits.maximum_windows()
            ),
        ));
    }
    if document.panel_instances().len() > limits.maximum_panel_instances() {
        return Err(validation_error(
            SurfaceValidationCode::TooManyPanelInstances,
            format!(
                "{} panel instances exceed limit {}",
                document.panel_instances().len(),
                limits.maximum_panel_instances()
            ),
        ));
    }

    let mut windows = BTreeSet::<&WindowId>::new();
    for window in document.windows() {
        if !windows.insert(window.id()) {
            return Err(validation_error(
                SurfaceValidationCode::DuplicateWindow,
                format!("duplicate participating window {}", window.id()),
            ));
        }
    }

    let mut instances = BTreeMap::new();
    let mut totals_by_definition = BTreeMap::<PanelDefinitionId, usize>::new();
    for instance in document.panel_instances() {
        if registry
            .panel_definition(instance.definition_id())
            .is_none()
        {
            return Err(validation_error(
                SurfaceValidationCode::UnknownPanelDefinition,
                format!(
                    "panel instance {} uses unknown definition {}",
                    instance.id(),
                    instance.definition_id()
                ),
            ));
        }
        if instances.insert(instance.id(), instance).is_some() {
            return Err(validation_error(
                SurfaceValidationCode::DuplicatePanelInstance,
                format!("duplicate panel instance {}", instance.id()),
            ));
        }
        *totals_by_definition
            .entry(instance.definition_id().clone())
            .or_default() += 1;
    }

    let mut surface_ids = BTreeSet::<&SurfaceId>::new();
    let mut orders_by_window = BTreeMap::<&WindowId, BTreeSet<u32>>::new();
    let mut members_by_window = BTreeMap::<&WindowId, BTreeSet<&SurfaceId>>::new();
    let mut placed_instances = BTreeSet::<PanelInstanceId>::new();
    let mut counts_by_surface = BTreeMap::<&SurfaceId, BTreeMap<PanelDefinitionId, usize>>::new();

    for surface in document.surfaces() {
        if !surface_ids.insert(surface.id()) {
            return Err(validation_error(
                SurfaceValidationCode::DuplicateSurface,
                format!("duplicate Surface {}", surface.id()),
            ));
        }
        if let Some(label) = surface.label()
            && label.len() > limits.maximum_label_bytes()
        {
            return Err(validation_error(
                SurfaceValidationCode::LabelTooLong,
                format!(
                    "Surface {} label is {} bytes; limit is {}",
                    surface.id(),
                    label.len(),
                    limits.maximum_label_bytes()
                ),
            ));
        }
        if surface.host_preferences().is_empty() {
            return Err(validation_error(
                SurfaceValidationCode::MissingHostPreference,
                format!("Surface {} has no candidate host", surface.id()),
            ));
        }
        if surface.host_preferences().len() > limits.maximum_host_preferences_per_surface() {
            return Err(validation_error(
                SurfaceValidationCode::TooManyHostPreferences,
                format!(
                    "Surface {} has {} host preferences; limit is {}",
                    surface.id(),
                    surface.host_preferences().len(),
                    limits.maximum_host_preferences_per_surface()
                ),
            ));
        }

        let mut preferred_windows = BTreeSet::<&WindowId>::new();
        for preference in surface.host_preferences() {
            if !windows.contains(preference.window_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::UnknownHostWindow,
                    format!(
                        "Surface {} references unknown participating window {}",
                        surface.id(),
                        preference.window_id()
                    ),
                ));
            }
            if !preferred_windows.insert(preference.window_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateHostPreference,
                    format!(
                        "Surface {} repeats host window {}",
                        surface.id(),
                        preference.window_id()
                    ),
                ));
            }

            let orders = orders_by_window.entry(preference.window_id()).or_default();
            if !orders.insert(preference.order()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateHostOrder,
                    format!(
                        "window {} repeats Surface order {}",
                        preference.window_id(),
                        preference.order()
                    ),
                ));
            }
            members_by_window
                .entry(preference.window_id())
                .or_default()
                .insert(surface.id());
        }

        let schema = registry.schema(surface.schema_id()).ok_or_else(|| {
            validation_error(
                SurfaceValidationCode::UnknownSchema,
                format!(
                    "Surface {} uses unknown schema {}",
                    surface.id(),
                    surface.schema_id()
                ),
            )
        })?;

        let mut region_ids = BTreeSet::new();
        let mut surface_definition_counts = BTreeMap::<PanelDefinitionId, usize>::new();
        for region in surface.regions() {
            if !region_ids.insert(region.region_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateRegionState,
                    format!(
                        "Surface {} repeats region {}",
                        surface.id(),
                        region.region_id()
                    ),
                ));
            }
            let region_definition = schema.region(region.region_id()).ok_or_else(|| {
                validation_error(
                    SurfaceValidationCode::UnknownRegion,
                    format!(
                        "Surface {} has unknown region {}",
                        surface.id(),
                        region.region_id()
                    ),
                )
            })?;
            if region.panel_instance_ids().len() > limits.maximum_panel_instances_per_region() {
                return Err(validation_error(
                    SurfaceValidationCode::TooManyPanelInstancesInRegion,
                    format!(
                        "Surface {} region {} has {} panel instances; limit is {}",
                        surface.id(),
                        region.region_id(),
                        region.panel_instance_ids().len(),
                        limits.maximum_panel_instances_per_region()
                    ),
                ));
            }
            if !region_definition.is_collapsible() && region.collapsed().is_some() {
                return Err(validation_error(
                    SurfaceValidationCode::UnsupportedCollapseState,
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
                    SurfaceValidationCode::ActivePanelNotInRegion,
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
                        SurfaceValidationCode::DuplicatePanelPlacement,
                        format!("panel instance {panel_instance_id} is placed more than once"),
                    ));
                }
                let instance = instances.get(panel_instance_id).ok_or_else(|| {
                    validation_error(
                        SurfaceValidationCode::UnknownPanelInstance,
                        format!(
                            "region {} references unknown panel instance {}",
                            region.region_id(),
                            panel_instance_id
                        ),
                    )
                })?;
                let allowed = registry
                    .is_panel_allowed_in(
                        surface.schema_id(),
                        instance.definition_id(),
                        region.region_id(),
                    )
                    .expect("schema, definition, and region were validated");
                if !allowed {
                    return Err(validation_error(
                        SurfaceValidationCode::PanelPlacementNotAllowed,
                        format!(
                            "panel instance {} is not allowed in region {}",
                            panel_instance_id,
                            region.region_id()
                        ),
                    ));
                }
                *surface_definition_counts
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
                SurfaceValidationCode::IncompleteRegionState,
                format!(
                    "Surface {} does not contain every schema region exactly once",
                    surface.id()
                ),
            ));
        }

        let mut sizing_slot_ids = BTreeSet::new();
        for slot in surface.sizing_slots() {
            if !sizing_slot_ids.insert(slot.sizing_slot_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateSizingSlotState,
                    format!(
                        "Surface {} repeats sizing slot {}",
                        surface.id(),
                        slot.sizing_slot_id()
                    ),
                ));
            }
            let definition = schema.sizing_slot(slot.sizing_slot_id()).ok_or_else(|| {
                validation_error(
                    SurfaceValidationCode::UnknownSizingSlot,
                    format!(
                        "Surface {} has unknown sizing slot {}",
                        surface.id(),
                        slot.sizing_slot_id()
                    ),
                )
            })?;
            if !definition.contains(slot.ratio()) {
                return Err(validation_error(
                    SurfaceValidationCode::SizingRatioOutOfBounds,
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
                SurfaceValidationCode::IncompleteSizingSlotState,
                format!(
                    "Surface {} does not contain every sizing slot exactly once",
                    surface.id()
                ),
            ));
        }

        counts_by_surface.insert(surface.id(), surface_definition_counts);
    }

    for (window_id, orders) in &orders_by_window {
        let complete = (0..orders.len()).all(|index| {
            u32::try_from(index)
                .ok()
                .is_some_and(|order| orders.contains(&order))
        });
        if !complete {
            return Err(validation_error(
                SurfaceValidationCode::IncompleteHostOrder,
                format!("window {window_id} Surface order is not a complete zero-based sequence"),
            ));
        }
    }

    for window in document.windows() {
        if let Some(active_surface_id) = window.active_surface_id()
            && !members_by_window
                .get(window.id())
                .is_some_and(|members| members.contains(active_surface_id))
        {
            return Err(validation_error(
                SurfaceValidationCode::ActiveSurfaceNotHosted,
                format!(
                    "active Surface {} is not a declared member of window {}",
                    active_surface_id,
                    window.id()
                ),
            ));
        }
    }

    if placed_instances.len() != instances.len() {
        return Err(validation_error(
            SurfaceValidationCode::UnplacedPanelInstance,
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
            PanelInstancePolicy::OnePerSurface => {
                for counts in counts_by_surface.values() {
                    let actual = counts.get(&definition_id).copied().unwrap_or_default();
                    if actual > 1 {
                        return Err(instance_limit_error(&definition_id, actual, 1, "surface"));
                    }
                }
            }
            PanelInstancePolicy::Bounded {
                maximum_per_document,
                maximum_per_surface,
            } => {
                if total > maximum_per_document as usize {
                    return Err(instance_limit_error(
                        &definition_id,
                        total,
                        maximum_per_document as usize,
                        "document",
                    ));
                }
                for counts in counts_by_surface.values() {
                    let actual = counts.get(&definition_id).copied().unwrap_or_default();
                    if actual > maximum_per_surface as usize {
                        return Err(instance_limit_error(
                            &definition_id,
                            actual,
                            maximum_per_surface as usize,
                            "surface",
                        ));
                    }
                }
            }
            PanelInstancePolicy::Singleton | PanelInstancePolicy::Multiple => {}
        }
    }

    Ok(())
}

/// Returns one canonical structural ordering for a valid Surface document.
///
/// # Errors
///
/// Returns [`SurfaceValidationError`] if `document` fails [`validate_document`].
pub fn normalize_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<SurfaceDocument, SurfaceValidationError> {
    validate_document(registry, document)?;
    let mut normalized = document.clone();

    normalized
        .surfaces_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));
    normalized
        .windows_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));
    normalized
        .panel_instances_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));

    for surface in normalized.surfaces_mut() {
        let schema = registry
            .schema(surface.schema_id())
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

        surface
            .regions_mut()
            .sort_by_key(|region| region_order[region.region_id()]);
        for region in surface.regions_mut() {
            let definition = schema
                .region(region.region_id())
                .expect("region existence was validated");
            region.normalize_active();
            region.normalize_collapse(definition.is_collapsible());
        }
        surface
            .sizing_slots_mut()
            .sort_by_key(|slot| sizing_order[slot.sizing_slot_id()]);
    }

    Ok(normalized)
}

/// Requires a valid Surface document to already use canonical structural order.
///
/// Intended for fixture verification and strict-load canonical checks where
/// callers want to reject non-canonical ordering rather than re-sorting it.
///
/// # Errors
///
/// Returns [`SurfaceValidationError`] if `document` fails [`validate_document`]
/// or is not already in canonical normalized form.
pub fn validate_normalized_document(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
) -> Result<(), SurfaceValidationError> {
    let normalized = normalize_document(registry, document)?;
    if normalized == *document {
        Ok(())
    } else {
        Err(validation_error(
            SurfaceValidationCode::NonCanonicalDocument,
            "Surface document is valid but not canonically normalized",
        ))
    }
}

fn instance_limit_error(
    definition_id: &PanelDefinitionId,
    actual: usize,
    maximum: usize,
    scope: &str,
) -> SurfaceValidationError {
    validation_error(
        SurfaceValidationCode::InstancePolicyExceeded,
        format!(
            "panel definition {definition_id} has {actual} instances per {scope}; maximum is {maximum}"
        ),
    )
}

fn validation_error(
    code: SurfaceValidationCode,
    detail: impl Into<String>,
) -> SurfaceValidationError {
    SurfaceValidationError {
        code,
        detail: detail.into(),
    }
}
