use longhorn_core::{
    LayoutContainerId, LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId,
    RegionFamilyId, RegionId, SizingSlotId, WindowId,
};
use longhorn_layout::{
    EmptyRegionPolicy, LayoutContainer, LayoutDocument, LayoutLimits, LayoutRatio,
    LayoutSchemaDefinition, PanelDefinition, PanelInstancePolicy, PlacementSelector,
    RegionDefinition, RegionState, SizingSlotDefinition, SizingSlotState,
};
use serde::Serialize;

const LOOPHOLE_PATH: &str = "fixtures/layout/loophole-conformance-v1.json";
const NUCLEUS_PATH: &str = "fixtures/layout/nucleus-conformance-v1.json";

#[derive(Clone, Copy)]
struct RegionSpec {
    id: &'static str,
    family: &'static str,
    empty_policy: EmptyRegionPolicy,
    collapsible: bool,
}

pub(super) struct ShapeSpec {
    pub(super) name: &'static str,
    pub(super) schema_id: &'static str,
    regions: &'static [RegionSpec],
    pub(super) sizing_slots: &'static [&'static str],
    pub(super) source_region: &'static str,
    pub(super) target_region: &'static str,
    pub(super) singleton_region: &'static str,
    singleton_family: &'static str,
    pub(super) singleton_definition: &'static str,
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
        spec.sizing_slots.iter().enumerate().map(|(index, id)| {
            SizingSlotDefinition::new(
                slot_id(id),
                u32::try_from(index).expect("fixture sizing count fits u32"),
                ratio(100_000),
                ratio(250_000),
                ratio(900_000),
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
            PanelInstancePolicy::Singleton,
            spec.name == "loophole",
            spec.name == "loophole",
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
                .map(|id| SizingSlotState::new(slot_id(id), ratio(250_000))),
        )],
        [],
    )
}

pub(super) fn limits() -> LayoutLimits {
    LayoutLimits::new(4, 16, 8, 8, 4, 32, 16).expect("conformance limits are valid")
}

pub(super) fn loophole_spec() -> (&'static str, ShapeSpec) {
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
    (
        LOOPHOLE_PATH,
        ShapeSpec {
            name: "loophole",
            schema_id: "schema:loophole",
            regions: REGIONS,
            sizing_slots: &["navigation-width", "inspector-width", "utility-height"],
            source_region: "primary",
            target_region: "secondary",
            singleton_region: "primary",
            singleton_family: "workspace",
            singleton_definition: "panel:transport",
            host_binding: HostBinding::Surface {
                surface_id: FixtureSurfaceId("surface:mix".into()),
                container_id: container_id("container:primary"),
            },
        },
    )
}

pub(super) fn nucleus_spec() -> (&'static str, ShapeSpec) {
    const REGIONS: &[RegionSpec] = &[
        region("navigation", "activity", true, true),
        region("main", "workspace", false, false),
        region("detail", "workspace", true, false),
        region("tasks", "utility", true, false),
        region("status", "chrome", false, true),
    ];
    (
        NUCLEUS_PATH,
        ShapeSpec {
            name: "nucleus",
            schema_id: "schema:nucleus",
            regions: REGIONS,
            sizing_slots: &[
                "navigation-width",
                "detail-width",
                "tasks-height",
                "content-scale",
            ],
            source_region: "main",
            target_region: "detail",
            singleton_region: "tasks",
            singleton_family: "utility",
            singleton_definition: "panel:tasks",
            host_binding: HostBinding::Window {
                window_id: WindowId::new("window:project").expect("fixture window id is valid"),
                container_id: container_id("container:primary"),
            },
        },
    )
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
