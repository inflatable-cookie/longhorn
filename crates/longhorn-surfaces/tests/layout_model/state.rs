use longhorn_core::SurfaceRevision;
use longhorn_surfaces::{
    LayoutRatio, LayoutValidationCode, PanelInstance, RegionState, SizingSlotState,
    SurfaceDocument, SurfaceRecord, normalize_layout, validate_layout, validate_normalized_layout,
};

use super::support::*;

#[test]
fn normalization_canonicalizes_structure_but_preserves_tab_order() {
    let chat = instance_id("instance:chat");
    let activity = instance_id("instance:activity");
    let tool = instance_id("instance:tool");
    let input = SurfaceDocument::new(
        SurfaceRevision::new(7),
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            [
                RegionState::new(region_id("right"), [], None, None),
                RegionState::new(
                    region_id("center"),
                    [tool.clone(), chat.clone()],
                    None,
                    None,
                ),
                RegionState::new(region_id("left"), [activity.clone()], None, None),
            ],
            [
                SizingSlotState::new(slot_id("right-width"), ratio(250_000)),
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
            ],
            [],
        )],
        [
            PanelInstance::new(tool.clone(), definition_id("panel:tool")),
            PanelInstance::new(chat.clone(), definition_id("panel:chat")),
            PanelInstance::new(activity.clone(), definition_id("panel:activity")),
        ],
        [],
    );

    let normalized = normalize_layout(&registry(), &input).unwrap();
    let surface = normalized.surfaces().first().unwrap();

    assert_eq!(
        normalized
            .panel_instances()
            .iter()
            .map(|instance| instance.id().as_str())
            .collect::<Vec<_>>(),
        vec!["instance:activity", "instance:chat", "instance:tool"]
    );
    assert_eq!(
        surface
            .regions()
            .iter()
            .map(|region| region.region_id().as_str())
            .collect::<Vec<_>>(),
        vec!["left", "center", "right"]
    );
    assert_eq!(
        surface
            .region(&region_id("center"))
            .unwrap()
            .panel_instance_ids(),
        &[tool.clone(), chat]
    );
    assert_eq!(
        surface
            .region(&region_id("center"))
            .unwrap()
            .active_panel_instance_id(),
        Some(&tool)
    );
    assert_eq!(
        surface.region(&region_id("left")).unwrap().collapsed(),
        Some(false)
    );
    assert_eq!(
        surface.region(&region_id("center")).unwrap().collapsed(),
        None
    );
    validate_normalized_layout(&registry(), &normalized).unwrap();
}

#[test]
fn normalization_is_idempotent_and_serde_round_trips() {
    let registry = registry();
    let first = normalize_layout(&registry, &document()).unwrap();
    let second = normalize_layout(&registry, &first).unwrap();
    let encoded = serde_json::to_vec(&second).unwrap();
    let decoded: SurfaceDocument = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(first, second);
    assert_eq!(second, decoded);
    assert_eq!(decoded.revision(), SurfaceRevision::new(7));
}

#[test]
fn validation_rejects_active_duplicate_unplaced_and_bad_sizing_state() {
    let chat = instance_id("instance:chat");
    let activity = instance_id("instance:activity");

    let invalid_active = single_surface_document(
        [
            RegionState::new(
                region_id("left"),
                [activity.clone()],
                Some(activity.clone()),
                Some(false),
            ),
            RegionState::new(
                region_id("center"),
                [chat.clone()],
                Some(instance_id("instance:missing")),
                None,
            ),
            RegionState::new(region_id("right"), [], None, Some(false)),
        ],
        [
            PanelInstance::new(activity.clone(), definition_id("panel:activity")),
            PanelInstance::new(chat.clone(), definition_id("panel:chat")),
        ],
        ratio(250_000),
    );
    assert_code(invalid_active, LayoutValidationCode::ActivePanelNotInRegion);

    let duplicate = single_surface_document(
        [
            RegionState::new(
                region_id("left"),
                [activity.clone()],
                Some(activity.clone()),
                Some(false),
            ),
            RegionState::new(
                region_id("center"),
                [chat.clone()],
                Some(chat.clone()),
                None,
            ),
            RegionState::new(
                region_id("right"),
                [chat.clone()],
                Some(chat.clone()),
                Some(false),
            ),
        ],
        [
            PanelInstance::new(activity.clone(), definition_id("panel:activity")),
            PanelInstance::new(chat.clone(), definition_id("panel:chat")),
        ],
        ratio(250_000),
    );
    assert_code(duplicate, LayoutValidationCode::DuplicatePanelPlacement);

    let unplaced = single_surface_document(
        [
            RegionState::new(
                region_id("left"),
                [activity.clone()],
                Some(activity.clone()),
                Some(false),
            ),
            RegionState::new(region_id("center"), [], None, None),
            RegionState::new(region_id("right"), [], None, Some(false)),
        ],
        [
            PanelInstance::new(activity, definition_id("panel:activity")),
            PanelInstance::new(chat, definition_id("panel:chat")),
        ],
        ratio(250_000),
    );
    assert_code(unplaced, LayoutValidationCode::UnplacedPanelInstance);

    assert_code(
        document_with_right_ratio(ratio(900_000)),
        LayoutValidationCode::SizingRatioOutOfBounds,
    );
}

#[test]
fn validation_rejects_incomplete_and_unsupported_region_state() {
    let activity = instance_id("instance:activity");
    let incomplete = single_surface_document(
        [
            RegionState::new(
                region_id("left"),
                [activity.clone()],
                Some(activity.clone()),
                Some(false),
            ),
            RegionState::new(region_id("center"), [], None, None),
        ],
        [PanelInstance::new(
            activity,
            definition_id("panel:activity"),
        )],
        ratio(250_000),
    );
    assert_code(incomplete, LayoutValidationCode::IncompleteRegionState);

    let mut regions = document().surfaces()[0].regions().to_vec();
    regions[1] = RegionState::new(
        region_id("center"),
        regions[1].panel_instance_ids().iter().cloned(),
        regions[1].active_panel_instance_id().cloned(),
        Some(false),
    );
    let invalid_collapse = SurfaceDocument::new(
        document().revision(),
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            regions,
            document().surfaces()[0].sizing_slots().iter().cloned(),
            [],
        )],
        document().panel_instances().iter().cloned(),
        [],
    );
    assert_code(
        invalid_collapse,
        LayoutValidationCode::UnsupportedCollapseState,
    );
}

#[test]
fn singleton_policy_is_enforced_across_the_document() {
    let first = instance_id("instance:chat-a");
    let second = instance_id("instance:chat-b");
    let document = SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            [
                RegionState::new(region_id("left"), [], None, Some(false)),
                RegionState::new(
                    region_id("center"),
                    [first.clone(), second.clone()],
                    Some(first.clone()),
                    None,
                ),
                RegionState::new(region_id("right"), [], None, Some(false)),
            ],
            [
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
                SizingSlotState::new(slot_id("right-width"), ratio(250_000)),
            ],
            [],
        )],
        [
            PanelInstance::new(first, definition_id("panel:chat")),
            PanelInstance::new(second, definition_id("panel:chat")),
        ],
        [],
    );

    assert_code(document, LayoutValidationCode::InstancePolicyExceeded);
}

#[test]
fn unknown_serialized_fields_fail_closed() {
    let mut value = serde_json::to_value(document()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("future_policy".into(), serde_json::Value::Bool(true));

    assert!(serde_json::from_value::<SurfaceDocument>(value).is_err());
}

#[test]
fn serialized_document_rejects_out_of_range_sizing_ratio() {
    let mut value = serde_json::to_value(document()).unwrap();
    value.as_object_mut().unwrap()["surfaces"][0]["sizing_slots"][0]
        .as_object_mut()
        .unwrap()
        .insert("ratio".into(), 1_000_001.into());

    assert!(serde_json::from_value::<SurfaceDocument>(value).is_err());
}

fn single_surface_document(
    regions: impl IntoIterator<Item = RegionState>,
    panel_instances: impl IntoIterator<Item = PanelInstance>,
    right_ratio: LayoutRatio,
) -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::INITIAL,
        [SurfaceRecord::new(
            surface_id("surface:primary"),
            schema_id("schema:workspace"),
            None,
            regions,
            [
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
                SizingSlotState::new(slot_id("right-width"), right_ratio),
            ],
            [],
        )],
        panel_instances,
        [],
    )
}

fn document_with_right_ratio(right_ratio: LayoutRatio) -> SurfaceDocument {
    let source = document();
    single_surface_document(
        source.surfaces()[0].regions().iter().cloned(),
        source.panel_instances().iter().cloned(),
        right_ratio,
    )
}

fn assert_code(document: SurfaceDocument, expected: LayoutValidationCode) {
    assert_eq!(
        validate_layout(&registry(), &document).unwrap_err().code(),
        expected
    );
}
