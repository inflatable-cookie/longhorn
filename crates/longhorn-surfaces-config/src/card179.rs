//! One-time transform from the two documents Card 179 merged into one.
//!
//! Before Card 179 a consumer stored a layout document and a Surface document
//! separately, joined by `layout_container_id`. Now there is one document, so
//! existing state needs joining once. Longhorn ships the transform rather than
//! leaving every consumer to write the same join, because getting it wrong
//! silently loses panels.
//!
//! This is deliberately not a `SurfaceMigration` implementation. The migration
//! hook is handed one raw document at its own schema version; this needs both,
//! and only the consumer knows where the second one is stored. Call it from
//! inside your own `migrate_one`.

use longhorn_config::DomainIssue;
use longhorn_core::{
    LayoutSchemaId, PanelInstanceId, RegionId, SizingSlotId, SurfaceId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    LayoutRatio, PanelInstance, ParticipatingWindow, RegionState, SizingSlotState, SurfaceDocument,
    SurfaceHostPreference, SurfacePresentation, SurfaceRecord,
};
use serde::Deserialize;

/// One pre-Card-179 layout document.
#[derive(Debug, Deserialize)]
struct StoredLayout {
    revision: u64,
    #[serde(default)]
    containers: Vec<StoredContainer>,
    #[serde(default)]
    panel_instances: Vec<StoredPanelInstance>,
}

#[derive(Debug, Deserialize)]
struct StoredContainer {
    id: String,
    schema_id: String,
    #[serde(default)]
    regions: Vec<StoredRegion>,
    #[serde(default)]
    sizing_slots: Vec<StoredSizingSlot>,
}

#[derive(Debug, Deserialize)]
struct StoredRegion {
    region_id: String,
    #[serde(default)]
    panel_instance_ids: Vec<String>,
    #[serde(default)]
    active_panel_instance_id: Option<String>,
    #[serde(default)]
    collapsed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StoredSizingSlot {
    sizing_slot_id: String,
    ratio: u32,
}

#[derive(Debug, Deserialize)]
struct StoredPanelInstance {
    id: String,
    definition_id: String,
}

/// One pre-Card-179 Surface document.
#[derive(Debug, Default, Deserialize)]
struct StoredSurfaces {
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    surfaces: Vec<StoredSurface>,
    #[serde(default)]
    windows: Vec<StoredWindow>,
}

#[derive(Debug, Deserialize)]
struct StoredSurface {
    id: String,
    layout_container_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    host_preferences: Vec<StoredHostPreference>,
}

#[derive(Debug, Deserialize)]
struct StoredHostPreference {
    window_id: String,
    order: u32,
}

#[derive(Debug, Deserialize)]
struct StoredWindow {
    id: String,
    #[serde(default)]
    active_surface_id: Option<String>,
}

fn issue(detail: impl Into<String>) -> DomainIssue {
    DomainIssue::new("card179-merge", detail.into())
}

fn parse<T: for<'de> Deserialize<'de>>(
    what: &str,
    value: &serde_json::Value,
) -> Result<T, DomainIssue> {
    serde_json::from_value(value.clone())
        .map_err(|error| issue(format!("stored {what} did not parse: {error}")))
}

/// Joins a stored layout document and Surface document into one.
///
/// Every container becomes a Surface. A container bound to a Surface keeps that
/// Surface's identity, label and hosting policy; a container bound to nothing
/// becomes an unlabelled Surface hosted in `unbound_host`, which is why the
/// caller has to name a window — a layout renders somewhere, and this transform
/// cannot know where.
///
/// The merged revision is the higher of the two, so neither side's
/// expected-revision history can appear to move backwards.
///
/// Surfaces naming a container that is not in the layout document are an error
/// rather than a silent drop: that pairing means the two files were not saved
/// together, and guessing would lose a panel arrangement.
pub fn merge_pre_card179_state(
    layout: &serde_json::Value,
    surfaces: Option<&serde_json::Value>,
    unbound_host: &WindowId,
) -> Result<SurfaceDocument, DomainIssue> {
    let layout: StoredLayout = parse("layout document", layout)?;
    let stored = match surfaces {
        Some(value) => parse::<StoredSurfaces>("Surface document", value)?,
        None => StoredSurfaces::default(),
    };

    let mut by_container = std::collections::BTreeMap::new();
    for surface in &stored.surfaces {
        if by_container
            .insert(surface.layout_container_id.as_str(), surface)
            .is_some()
        {
            return Err(issue(format!(
                "layout container {} was bound to more than one Surface",
                surface.layout_container_id
            )));
        }
    }

    let container_ids: std::collections::BTreeSet<&str> =
        layout.containers.iter().map(|c| c.id.as_str()).collect();
    for surface in &stored.surfaces {
        if !container_ids.contains(surface.layout_container_id.as_str()) {
            return Err(issue(format!(
                "Surface {} names layout container {}, which the stored layout document does not contain",
                surface.id, surface.layout_container_id
            )));
        }
    }

    let mut records = Vec::with_capacity(layout.containers.len());
    for container in &layout.containers {
        let bound = by_container.get(container.id.as_str());
        let id = match bound {
            Some(surface) => surface_id(&surface.id)?,
            // The container id becomes the Surface id, so a consumer that had
            // no Surface document keeps identifiers it already stores elsewhere.
            None => surface_id(&container.id)?,
        };
        let preferences = match bound {
            Some(surface) if !surface.host_preferences.is_empty() => surface
                .host_preferences
                .iter()
                .map(|preference| {
                    Ok(SurfaceHostPreference::new(
                        window_id(&preference.window_id)?,
                        preference.order,
                    ))
                })
                .collect::<Result<Vec<_>, DomainIssue>>()?,
            _ => vec![SurfaceHostPreference::new(unbound_host.clone(), 0)],
        };
        records.push(SurfaceRecord::with_presentation(
            id,
            schema_id(&container.schema_id)?,
            bound.and_then(|surface| surface.label.clone()),
            SurfacePresentation::Regional,
            regions(container)?,
            sizing_slots(container)?,
            preferences,
        ));
    }

    let windows = stored
        .windows
        .iter()
        .map(|window| {
            Ok(ParticipatingWindow::new(
                window_id(&window.id)?,
                window
                    .active_surface_id
                    .as_deref()
                    .map(surface_id)
                    .transpose()?,
            ))
        })
        .collect::<Result<Vec<_>, DomainIssue>>()?;

    let panel_instances = layout
        .panel_instances
        .iter()
        .map(|instance| {
            Ok(PanelInstance::new(
                instance_id(&instance.id)?,
                panel_definition_id(&instance.definition_id)?,
            ))
        })
        .collect::<Result<Vec<_>, DomainIssue>>()?;

    Ok(SurfaceDocument::new(
        SurfaceRevision::new(layout.revision.max(stored.revision)),
        records,
        panel_instances,
        windows,
    ))
}

fn regions(container: &StoredContainer) -> Result<Vec<RegionState>, DomainIssue> {
    container
        .regions
        .iter()
        .map(|region| {
            Ok(RegionState::new(
                region_id(&region.region_id)?,
                region
                    .panel_instance_ids
                    .iter()
                    .map(|id| instance_id(id))
                    .collect::<Result<Vec<_>, DomainIssue>>()?,
                region
                    .active_panel_instance_id
                    .as_deref()
                    .map(instance_id)
                    .transpose()?,
                region.collapsed,
            ))
        })
        .collect()
}

fn sizing_slots(container: &StoredContainer) -> Result<Vec<SizingSlotState>, DomainIssue> {
    container
        .sizing_slots
        .iter()
        .map(|slot| {
            Ok(SizingSlotState::new(
                slot_id(&slot.sizing_slot_id)?,
                LayoutRatio::from_millionths(slot.ratio)
                    .map_err(|error| issue(format!("sizing ratio: {error}")))?,
            ))
        })
        .collect()
}

macro_rules! identity {
    ($name:ident, $type:ty, $what:literal) => {
        fn $name(value: &str) -> Result<$type, DomainIssue> {
            <$type>::new(value).map_err(|error| issue(format!("{} {value}: {error}", $what)))
        }
    };
}

identity!(surface_id, SurfaceId, "Surface id");
identity!(window_id, WindowId, "window id");
identity!(schema_id, LayoutSchemaId, "layout schema id");
identity!(region_id, RegionId, "region id");
identity!(slot_id, SizingSlotId, "sizing slot id");
identity!(instance_id, PanelInstanceId, "panel instance id");
identity!(
    panel_definition_id,
    longhorn_core::PanelDefinitionId,
    "panel definition id"
);
