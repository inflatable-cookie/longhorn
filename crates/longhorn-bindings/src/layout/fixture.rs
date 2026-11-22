use std::error::Error;

use longhorn_core::{
    LayoutRequestId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId,
    SizingSlotId, SurfaceId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LAYOUT_PROTOCOL_VERSION, LayoutDefinitionRegistry, LayoutMutationCommand,
    LayoutMutationEngine, LayoutMutationReceipt, LayoutMutationRejection, LayoutMutationRequest,
    LayoutRatio, LayoutSchemaDefinition, PanelDefinition, PanelInstance, PanelInstancePolicy,
    ParticipatingWindow, PlacementSelector, RegionDefinition, RegionState, RegionVisibility,
    SizingSlotDefinition, SizingSlotState, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceRecord, project_region_visibility,
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct GoldenFixture {
    protocol_version: u32,
    definitions: GoldenDefinitions,
    snapshots: Vec<SurfaceDocument>,
    commands: Vec<LayoutMutationRequest>,
    receipts: Vec<LayoutMutationReceipt>,
    errors: Vec<LayoutMutationRejection>,
    visibility: Vec<RegionVisibility>,
    incompatibility: IncompatibilityFixture,
}

#[derive(Serialize)]
struct GoldenDefinitions {
    limits: SurfaceLimits,
    schemas: Vec<LayoutSchemaDefinition>,
    panels: Vec<PanelDefinition>,
}

#[derive(Serialize)]
struct IncompatibilityFixture {
    future_protocol_version: u32,
    unknown_command: Value,
    unknown_outcome: Value,
    unknown_rejection_code: Value,
}

/// The registry the Surface fixture also needs, so both golden files describe
/// the same registered schema.
pub(crate) fn registry() -> Result<LayoutDefinitionRegistry, Box<dyn Error>> {
    Ok(LayoutDefinitionRegistry::new(
        limits(),
        [schema()],
        panel_definitions(),
    )?)
}

pub fn render(rejection_codes: &[String]) -> Result<String, Box<dyn Error>> {
    let limits = limits();
    let schema = schema();
    let panels = panel_definitions();
    let registry = LayoutDefinitionRegistry::new(limits, [schema.clone()], panels.iter().cloned())?;
    let source = document();
    let engine = LayoutMutationEngine::new(&registry);
    let commands = requests(source.revision());
    let receipts: Vec<_> = commands
        .iter()
        .map(|request| engine.apply(&source, request))
        .collect::<Result<_, _>>()?;
    let stale = LayoutMutationRequest::new(
        request_id("request:stale"),
        SurfaceRevision::INITIAL,
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
    );
    let stale_rejection = engine
        .apply(&source, &stale)
        .expect_err("stale fixture request must be rejected");
    let mut visibility =
        project_region_visibility(&registry, &source, &surface_id("surface:primary"), None)?;
    visibility.extend(project_region_visibility(
        &registry,
        &source,
        &surface_id("surface:primary"),
        Some(&definition_id("panel:tool")),
    )?);
    let errors = rejection_fixtures(&source, &stale_rejection, rejection_codes)?;
    let mut snapshots = vec![source];
    snapshots.extend(
        receipts
            .iter()
            .map(|receipt| receipt.authoritative_document().clone()),
    );
    let fixture = GoldenFixture {
        protocol_version: LAYOUT_PROTOCOL_VERSION,
        definitions: GoldenDefinitions {
            limits,
            schemas: vec![schema],
            panels,
        },
        snapshots,
        commands,
        receipts,
        errors,
        visibility,
        incompatibility: IncompatibilityFixture {
            future_protocol_version: LAYOUT_PROTOCOL_VERSION + 1,
            unknown_command: json!({"kind": "future_layout_command"}),
            unknown_outcome: json!({"kind": "future_layout_outcome"}),
            unknown_rejection_code: json!("future_layout_rejection"),
        },
    };

    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}

fn rejection_fixtures(
    source: &SurfaceDocument,
    stale_rejection: &LayoutMutationRejection,
    rejection_codes: &[String],
) -> Result<Vec<LayoutMutationRejection>, serde_json::Error> {
    rejection_codes
        .iter()
        .enumerate()
        .map(|(index, code)| {
            if *code == "stale_revision" {
                return Ok(stale_rejection.clone());
            }
            serde_json::from_value(json!({
                "request_id": format!("request:error-{index:02}"),
                "current_revision": source.revision(),
                "code": code,
                "detail": format!("golden fixture for {code}"),
                "authoritative_document": source,
            }))
        })
        .collect()
}

fn requests(revision: SurfaceRevision) -> Vec<LayoutMutationRequest> {
    vec![
        request(
            "request:create",
            revision,
            LayoutMutationCommand::CreatePanel {
                panel_instance_id: instance_id("instance:tool-2"),
                panel_definition_id: definition_id("panel:tool"),
                surface_id: surface_id("surface:primary"),
                region_id: region_id("right"),
                insertion_index: 0,
            },
        ),
        request(
            "request:close",
            revision,
            LayoutMutationCommand::ClosePanel {
                panel_instance_id: instance_id("instance:chat"),
            },
        ),
        request(
            "request:activate",
            revision,
            LayoutMutationCommand::ActivatePanel {
                panel_instance_id: instance_id("instance:tool"),
            },
        ),
        request(
            "request:reorder",
            revision,
            LayoutMutationCommand::ReorderRegion {
                surface_id: surface_id("surface:primary"),
                region_id: region_id("center"),
                panel_instance_ids: vec![
                    instance_id("instance:tool"),
                    instance_id("instance:chat"),
                ],
            },
        ),
        request(
            "request:move",
            revision,
            LayoutMutationCommand::MovePanel {
                panel_instance_id: instance_id("instance:chat"),
                target_surface_id: surface_id("surface:primary"),
                target_region_id: region_id("right"),
                insertion_index: 0,
            },
        ),
        request(
            "request:sizing",
            revision,
            LayoutMutationCommand::SetSizingSlot {
                surface_id: surface_id("surface:primary"),
                sizing_slot_id: slot_id("right-width"),
                ratio: ratio(400_000),
            },
        ),
        request(
            "request:collapse",
            revision,
            LayoutMutationCommand::SetRegionCollapsed {
                surface_id: surface_id("surface:primary"),
                region_id: region_id("left"),
                collapsed: true,
            },
        ),
    ]
}

fn request(
    id: &str,
    revision: SurfaceRevision,
    command: LayoutMutationCommand,
) -> LayoutMutationRequest {
    LayoutMutationRequest::new(request_id(id), revision, command)
}

fn limits() -> SurfaceLimits {
    SurfaceLimits::builder()
        .maximum_schemas(8)
        .maximum_regions_per_schema(16)
        .maximum_sizing_slots_per_schema(16)
        .maximum_panel_definitions(32)
        .maximum_surfaces(8)
        .maximum_windows(4)
        .maximum_host_preferences_per_surface(4)
        .maximum_label_bytes(64)
        .maximum_panel_instances(128)
        .maximum_panel_instances_per_region(64)
        .build()
        .expect("fixture limits are valid")
}

pub(crate) fn schema() -> LayoutSchemaDefinition {
    LayoutSchemaDefinition::new(
        schema_id("schema:workspace"),
        [
            RegionDefinition::new(
                region_id("left"),
                family_id("activity"),
                10,
                EmptyRegionPolicy::KeepVisible,
                true,
            ),
            RegionDefinition::new(
                region_id("center"),
                family_id("workspace"),
                20,
                EmptyRegionPolicy::HideWhenEmpty,
                false,
            ),
            RegionDefinition::new(
                region_id("right"),
                family_id("workspace"),
                30,
                EmptyRegionPolicy::HideWhenEmpty,
                true,
            ),
        ],
        [
            SizingSlotDefinition::new(
                slot_id("left-width"),
                10,
                ratio(100_000),
                ratio(200_000),
                ratio(500_000),
            ),
            SizingSlotDefinition::new(
                slot_id("right-width"),
                20,
                ratio(100_000),
                ratio(250_000),
                ratio(500_000),
            ),
        ],
    )
}

fn panel_definitions() -> Vec<PanelDefinition> {
    vec![
        PanelDefinition::new(
            definition_id("panel:chat"),
            [PlacementSelector::Region(region_id("center"))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Singleton,
            true,
            true,
        ),
        PanelDefinition::new(
            definition_id("panel:tool"),
            [PlacementSelector::Region(region_id("right"))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Multiple,
            true,
            true,
        ),
        PanelDefinition::new(
            definition_id("panel:activity"),
            [PlacementSelector::Region(region_id("left"))],
            [PlacementSelector::Family(family_id("activity"))],
            PanelInstancePolicy::OnePerSurface,
            false,
            false,
        ),
        PanelDefinition::new(
            definition_id("panel:bounded"),
            [PlacementSelector::Region(region_id("center"))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Bounded {
                maximum_per_document: 2,
                maximum_per_surface: 1,
            },
            true,
            true,
        ),
    ]
}

fn document() -> SurfaceDocument {
    let activity = instance_id("instance:activity");
    let chat = instance_id("instance:chat");
    let tool = instance_id("instance:tool");
    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            [
                RegionState::new(
                    region_id("left"),
                    [activity.clone()],
                    Some(activity.clone()),
                    Some(false),
                ),
                RegionState::new(
                    region_id("center"),
                    [chat.clone(), tool.clone()],
                    Some(chat.clone()),
                    None,
                ),
                RegionState::new(region_id("right"), [], None, Some(false)),
            ],
            [
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
                SizingSlotState::new(slot_id("right-width"), ratio(250_000)),
            ],
            [SurfaceHostPreference::new(window_id("window:primary"), 0)],
        )],
        [
            PanelInstance::new(activity, definition_id("panel:activity")),
            PanelInstance::new(chat, definition_id("panel:chat")),
            PanelInstance::new(tool, definition_id("panel:tool")),
        ],
        [ParticipatingWindow::new(
            window_id("window:primary"),
            Some(surface_id("surface:primary")),
        )],
    )
}

fn window_id(value: &str) -> WindowId {
    WindowId::new(value).expect("fixture window id is valid")
}

fn schema_id(value: &str) -> LayoutSchemaId {
    LayoutSchemaId::new(value).expect("fixture schema id is valid")
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).expect("fixture container id is valid")
}

fn region_id(value: &str) -> RegionId {
    RegionId::new(value).expect("fixture region id is valid")
}

fn family_id(value: &str) -> RegionFamilyId {
    RegionFamilyId::new(value).expect("fixture family id is valid")
}

fn slot_id(value: &str) -> SizingSlotId {
    SizingSlotId::new(value).expect("fixture sizing slot id is valid")
}

fn definition_id(value: &str) -> PanelDefinitionId {
    PanelDefinitionId::new(value).expect("fixture panel definition id is valid")
}

fn instance_id(value: &str) -> PanelInstanceId {
    PanelInstanceId::new(value).expect("fixture panel instance id is valid")
}

fn request_id(value: &str) -> LayoutRequestId {
    LayoutRequestId::new(value).expect("fixture request id is valid")
}

fn ratio(value: u32) -> LayoutRatio {
    LayoutRatio::from_millionths(value).expect("fixture ratio is valid")
}
