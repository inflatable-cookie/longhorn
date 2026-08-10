use longhorn_core::{
    LayoutContainerId, LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId,
    RegionFamilyId, RegionId, SizingSlotId, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutContainer, LayoutDocument, LayoutLimits, LayoutRatio,
    LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy, PlacementSelector,
    RegionDefinition, RegionState, SizingSlotDefinition, SizingSlotState,
};
use serde::Serialize;

const SURFACE_BOUND_PATH: &str = "fixtures/layout/surface-bound-conformance-v1.json";
const WINDOW_BOUND_PATH: &str = "fixtures/layout/window-bound-conformance-v1.json";

#[derive(Clone, Copy)]
struct RegionSpec {
    id: &'static str,
    family: &'static str,
    empty_policy: EmptyRegionPolicy,
    collapsible: bool,
}

#[derive(Clone, Copy)]
pub(super) struct SizingSpec {
    pub(super) id: &'static str,
    minimum: u32,
    default: u32,
    maximum: u32,
}

pub(super) struct ShapeSpec {
    pub(super) name: &'static str,
    pub(super) schema_id: &'static str,
    regions: &'static [RegionSpec],
    pub(super) sizing_slots: &'static [SizingSpec],
    pub(super) source_region: &'static str,
    pub(super) target_region: &'static str,
    pub(super) singleton_region: &'static str,
    singleton_family: &'static str,
    pub(super) singleton_definition: &'static str,
    singleton_policy: PanelInstancePolicy,
    pub(super) host_binding: HostBinding,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(super) enum HostBinding {
    Surface {
        surface_id: FixtureSurfaceId,
        container_id: LayoutContainerId,
    },
    Window {
        window_id: WindowId,
        container_id: LayoutContainerId,
    },
}

#[derive(Clone, Serialize)]
#[serde(transparent)]
pub(super) struct FixtureSurfaceId(String);

pub(super) fn schema(spec: &ShapeSpec) -> LayoutSchemaDefinition {
    LayoutSchemaDefinition::new(
        schema_id(spec.schema_id),
        spec.regions.iter().enumerate().map(|(index, region)| {
            RegionDefinition::new(
                region_id(region.id),
                family_id(region.family),
                u32::try_from(index).expect("fixture region count fits u32"),
                region.empty_policy,
                region.collapsible,
            )
        }),
        spec.sizing_slots.iter().enumerate().map(|(index, slot)| {
            SizingSlotDefinition::new(
                slot_id(slot.id),
                u32::try_from(index).expect("fixture sizing count fits u32"),
                ratio(slot.minimum),
                ratio(slot.default),
                ratio(slot.maximum),
            )
        }),
    )
}

pub(super) fn panels(spec: &ShapeSpec) -> Vec<PanelDefinition> {
    vec![
        PanelDefinition::new(
            definition_id("panel:workspace-tool"),
            [PlacementSelector::Region(region_id(spec.source_region))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Multiple,
            true,
            true,
        ),
        PanelDefinition::new(
            definition_id(spec.singleton_definition),
            [PlacementSelector::Region(region_id(spec.singleton_region))],
            [PlacementSelector::Family(family_id(spec.singleton_family))],
            spec.singleton_policy,
            true,
            true,
        ),
    ]
}

pub(super) fn initial_document(spec: &ShapeSpec) -> LayoutDocument {
    LayoutDocument::new(
        longhorn_core::LayoutRevision::INITIAL,
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id(spec.schema_id),
            spec.regions.iter().map(|region| {
                RegionState::new(
                    region_id(region.id),
                    [],
                    None,
                    region.collapsible.then_some(false),
                )
            }),
            spec.sizing_slots
                .iter()
                .map(|slot| SizingSlotState::new(slot_id(slot.id), ratio(slot.default))),
        )],
        [],
    )
}

pub(super) fn limits() -> LayoutLimits {
    LayoutLimits::new(4, 16, 8, 8, 4, 32, 16).expect("conformance limits are valid")
}

pub(super) fn surface_bound_spec() -> (&'static str, ShapeSpec) {
    const REGIONS: &[RegionSpec] = &[
        region("navigation", "edge", true, false),
        region("activity", "edge", true, false),
        region("primary", "workspace", false, false),
        region("secondary", "workspace", true, false),
        region("inspector", "edge", true, false),
        region("timeline", "utility", true, false),
        region("console", "utility", true, false),
        region("status", "chrome", false, true),
    ];
    const SIZING: &[SizingSpec] = &[
        sizing("navigation-width", 100_000, 250_000, 900_000),
        sizing("inspector-width", 100_000, 250_000, 900_000),
        sizing("utility-height", 100_000, 250_000, 900_000),
    ];
    (
        SURFACE_BOUND_PATH,
        ShapeSpec {
            name: "surface-bound",
            schema_id: "schema:surface-bound",
            regions: REGIONS,
            sizing_slots: SIZING,
            source_region: "primary",
            target_region: "secondary",
            singleton_region: "primary",
            singleton_family: "workspace",
            singleton_definition: "panel:transport",
            singleton_policy: PanelInstancePolicy::Singleton,
            host_binding: HostBinding::Surface {
                surface_id: FixtureSurfaceId("surface:mix".into()),
                container_id: container_id("container:primary"),
            },
        },
    )
}

pub(super) fn window_bound_spec() -> (&'static str, ShapeSpec) {
    const REGIONS: &[RegionSpec] = &[
        region("left", "activity", false, false),
        region("center_top", "workspace", false, false),
        region("center_bottom", "workspace", true, false),
        region("right_top", "workspace", true, false),
        region("right_bottom", "workspace", true, false),
    ];
    const SIZING: &[SizingSpec] = &[
        sizing("left-center", 200_000, 200_000, 900_000),
        sizing("center-right", 200_000, 740_000, 900_000),
        sizing("center-stack", 200_000, 740_000, 900_000),
        sizing("right-stack", 200_000, 740_000, 900_000),
    ];
    (
        WINDOW_BOUND_PATH,
        ShapeSpec {
            name: "window-bound",
            schema_id: "schema:window-bound",
            regions: REGIONS,
            sizing_slots: SIZING,
            source_region: "center_top",
            target_region: "right_top",
            singleton_region: "center_top",
            singleton_family: "workspace",
            singleton_definition: "panel:tasks",
            singleton_policy: PanelInstancePolicy::OnePerContainer,
            host_binding: HostBinding::Window {
                window_id: WindowId::new("window:primary").expect("fixture window id is valid"),
                container_id: container_id("container:primary"),
            },
        },
    )
}

const fn sizing(id: &'static str, minimum: u32, default: u32, maximum: u32) -> SizingSpec {
    SizingSpec {
        id,
        minimum,
        default,
        maximum,
    }
}

const fn region(
    id: &'static str,
    family: &'static str,
    collapsible: bool,
    keep_visible: bool,
) -> RegionSpec {
    RegionSpec {
        id,
        family,
        empty_policy: if keep_visible {
            EmptyRegionPolicy::KeepVisible
        } else {
            EmptyRegionPolicy::HideWhenEmpty
        },
        collapsible,
    }
}

pub(super) fn schema_id(value: &str) -> LayoutSchemaId {
    LayoutSchemaId::new(value).expect("fixture schema id is valid")
}

pub(super) fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).expect("fixture container id is valid")
}

pub(super) fn region_id(value: &str) -> RegionId {
    RegionId::new(value).expect("fixture region id is valid")
}

fn family_id(value: &str) -> RegionFamilyId {
    RegionFamilyId::new(value).expect("fixture family id is valid")
}

pub(super) fn slot_id(value: &str) -> SizingSlotId {
    SizingSlotId::new(value).expect("fixture sizing slot id is valid")
}

pub(super) fn definition_id(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).expect("fixture panel definition id is valid")
}

pub(super) fn instance_id(value: &str) -> PanelInstanceId {
    PanelInstanceId::new(value).expect("fixture panel instance id is valid")
}

pub(super) fn request_id(value: &str) -> LayoutRequestId {
    LayoutRequestId::new(value).expect("fixture request id is valid")
}

pub(super) fn ratio(value: u32) -> LayoutRatio {
    LayoutRatio::from_millionths(value).expect("fixture ratio is valid")
}
