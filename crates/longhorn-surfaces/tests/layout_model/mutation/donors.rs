use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutMutationCommand, LayoutMutationEngine,
    LayoutMutationReceipt, LayoutMutationRequest, LayoutSchemaDefinition, PanelDefinition,
    PanelInstancePolicy, PlacementSelector, RegionDefinition, RegionState, SizingSlotDefinition,
    SizingSlotState, SurfaceDocument, SurfaceRecord,
};

use crate::support::*;

#[test]
fn loophole_eight_region_sequence_uses_the_shared_engine() {
    run_donor_sequence(
        "schema:loophole",
        [
            "navigation",
            "activity",
            "primary",
            "secondary",
            "inspector",
            "timeline",
            "console",
            "status",
        ],
        ["navigation-width", "inspector-width", "utility-height"],
    );
}

#[test]
fn nucleus_five_region_sequence_uses_the_shared_engine_without_surfaces() {
    run_donor_sequence(
        "schema:nucleus",
        ["navigation", "main", "detail", "tasks", "status"],
        [
            "navigation-width",
            "detail-width",
            "tasks-height",
            "content-scale",
        ],
    );
}

fn run_donor_sequence<const REGIONS: usize, const SLOTS: usize>(
    schema: &str,
    regions: [&str; REGIONS],
    slots: [&str; SLOTS],
) {
    let registry = donor_registry(schema, &regions, &slots);
    let engine = LayoutMutationEngine::new(&registry);
    let source = donor_document(schema, &regions, &slots);
    let first = apply(
        &engine,
        &source,
        "request:create-a",
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:a"),
            panel_definition_id: definition_id("panel:workspace"),
            surface_id: surface_id("surface:primary"),
            region_id: region_id(regions[0]),
            insertion_index: 0,
        },
    );
    let second = apply(
        &engine,
        first.authoritative_document(),
        "request:create-b",
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:b"),
            panel_definition_id: definition_id("panel:workspace"),
            surface_id: surface_id("surface:primary"),
            region_id: region_id(regions[0]),
            insertion_index: 1,
        },
    );
    let reordered = apply(
        &engine,
        second.authoritative_document(),
        "request:reorder",
        LayoutMutationCommand::ReorderRegion {
            surface_id: surface_id("surface:primary"),
            region_id: region_id(regions[0]),
            panel_instance_ids: vec![instance_id("instance:b"), instance_id("instance:a")],
        },
    );
    let moved = apply(
        &engine,
        reordered.authoritative_document(),
        "request:move",
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:b"),
            target_surface_id: surface_id("surface:primary"),
            target_region_id: region_id(regions[1]),
            insertion_index: 0,
        },
    );
    let collapsed = apply(
        &engine,
        moved.authoritative_document(),
        "request:collapse",
        LayoutMutationCommand::SetRegionCollapsed {
            surface_id: surface_id("surface:primary"),
            region_id: region_id(regions[1]),
            collapsed: true,
        },
    );
    let sized = apply(
        &engine,
        collapsed.authoritative_document(),
        "request:size",
        LayoutMutationCommand::SetSizingSlot {
            surface_id: surface_id("surface:primary"),
            sizing_slot_id: slot_id(slots[0]),
            ratio: ratio(300_000),
        },
    );

    assert_eq!(sized.committed_revision(), SurfaceRevision::new(6));
    assert_eq!(
        sized.authoritative_document().surfaces()[0]
            .region(&region_id(regions[1]))
            .unwrap()
            .panel_instance_ids(),
        &[instance_id("instance:b")]
    );
}

fn donor_registry(schema: &str, regions: &[&str], slots: &[&str]) -> LayoutDefinitionRegistry {
    let schema = LayoutSchemaDefinition::new(
        schema_id(schema),
        regions.iter().enumerate().map(|(index, id)| {
            RegionDefinition::new(
                region_id(id),
                family_id("workspace"),
                u32::try_from(index).unwrap(),
                EmptyRegionPolicy::HideWhenEmpty,
                index != 0,
            )
        }),
        slots.iter().enumerate().map(|(index, id)| {
            SizingSlotDefinition::new(
                slot_id(id),
                u32::try_from(index).unwrap(),
                ratio(100_000),
                ratio(250_000),
                ratio(900_000),
            )
        }),
    );
    LayoutDefinitionRegistry::new(
        limits(),
        [schema],
        [PanelDefinition::new(
            definition_id("panel:workspace"),
            [PlacementSelector::Region(region_id(regions[0]))],
            [PlacementSelector::Family(family_id("workspace"))],
            PanelInstancePolicy::Multiple,
            true,
            true,
        )],
    )
    .unwrap()
}

fn donor_document(schema: &str, regions: &[&str], slots: &[&str]) -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id(schema),
            None,
            regions.iter().enumerate().map(|(index, id)| {
                RegionState::new(region_id(id), [], None, (index != 0).then_some(false))
            }),
            slots
                .iter()
                .map(|id| SizingSlotState::new(slot_id(id), ratio(250_000))),
            [],
        )],
        [],
        [],
    )
}

fn apply(
    engine: &LayoutMutationEngine<'_>,
    document: &SurfaceDocument,
    id: &str,
    command: LayoutMutationCommand,
) -> LayoutMutationReceipt {
    engine
        .apply(
            document,
            &LayoutMutationRequest::new(request_id(id), document.revision(), command),
        )
        .unwrap()
}
