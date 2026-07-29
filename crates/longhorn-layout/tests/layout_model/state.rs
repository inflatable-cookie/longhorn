use longhorn_core::LayoutRevision;
use longhorn_layout::{
    LayoutContainer, LayoutDocument, LayoutValidationCode, PanelInstance, RegionState,
    SizingSlotState, normalize_document, validate_document, validate_normalized_document,
};

use super::support::*;

#[test]
fn normalization_canonicalizes_structure_but_preserves_tab_order() {
    let chat = instance_id("instance:chat");
    let activity = instance_id("instance:activity");
    let tool = instance_id("instance:tool");
    let input = LayoutDocument::new(
        LayoutRevision::new(7),
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id("schema:workspace"),
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
        )],
        [
            PanelInstance::new(tool.clone(), definition_id("panel:tool")),
            PanelInstance::new(chat.clone(), definition_id("panel:chat")),
            PanelInstance::new(activity.clone(), definition_id("panel:activity")),
        ],
    );

    let normalized = normalize_document(&registry(), &input).unwrap();
    let container = normalized.containers().first().unwrap();

    assert_eq!(
        normalized
            .panel_instances()
            .iter()
            .map(|instance| instance.id().as_str())
            .collect::<Vec<_>>(),
        vec!["instance:activity", "instance:chat", "instance:tool"]
    );
    assert_eq!(
        container
            .regions()
            .iter()
            .map(|region| region.region_id().as_str())
            .collect::<Vec<_>>(),
        vec!["left", "center", "right"]
    );
    assert_eq!(
        container
            .region(&region_id("center"))
            .unwrap()
            .panel_instance_ids(),
        &[tool.clone(), chat]
    );
    assert_eq!(
        container
            .region(&region_id("center"))
            .unwrap()
            .active_panel_instance_id(),
        Some(&tool)
    );
    assert_eq!(
        container.region(&region_id("left")).unwrap().collapsed(),
        Some(false)
    );
    assert_eq!(
        container.region(&region_id("center")).unwrap().collapsed(),
        None
    );
    validate_normalized_document(&registry(), &normalized).unwrap();
}

#[test]
fn normalization_is_idempotent_and_serde_round_trips() {
    let registry = registry();
    let first = normalize_document(&registry, &document()).unwrap();
    let second = normalize_document(&registry, &first).unwrap();
    let encoded = serde_json::to_vec(&second).unwrap();
    let decoded: LayoutDocument = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(first, second);
    assert_eq!(second, decoded);
    assert_eq!(decoded.revision(), LayoutRevision::new(7));
}

#[test]
fn validation_rejects_active_duplicate_unplaced_and_bad_sizing_state() {
    let chat = instance_id("instance:chat");
    let activity = instance_id("instance:activity");

    let invalid_active = single_container_document(
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

    let duplicate = single_container_document(
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

    let unplaced = single_container_document(
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
    let incomplete = single_container_document(
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

    let mut regions = document().containers()[0].regions().to_vec();
    regions[1] = RegionState::new(
        region_id("center"),
        regions[1].panel_instance_ids().iter().cloned(),
        regions[1].active_panel_instance_id().cloned(),
        Some(false),
    );
    let invalid_collapse = LayoutDocument::new(
        document().revision(),
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id("schema:workspace"),
            regions,
            document().containers()[0].sizing_slots().iter().cloned(),
        )],
        document().panel_instances().iter().cloned(),
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
    let document = LayoutDocument::new(
        LayoutRevision::INITIAL,
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id("schema:workspace"),
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
        )],
        [
            PanelInstance::new(first, definition_id("panel:chat")),
            PanelInstance::new(second, definition_id("panel:chat")),
        ],
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

    assert!(serde_json::from_value::<LayoutDocument>(value).is_err());
}

fn single_container_document(
    regions: impl IntoIterator<Item = RegionState>,
    panel_instances: impl IntoIterator<Item = PanelInstance>,
    right_ratio: longhorn_layout::LayoutRatio,
) -> LayoutDocument {
    LayoutDocument::new(
        LayoutRevision::INITIAL,
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id("schema:workspace"),
            regions,
            [
                SizingSlotState::new(slot_id("left-width"), ratio(200_000)),
                SizingSlotState::new(slot_id("right-width"), right_ratio),
            ],
        )],
        panel_instances,
    )
}

fn document_with_right_ratio(right_ratio: longhorn_layout::LayoutRatio) -> LayoutDocument {
    let source = document();
    single_container_document(
        source.containers()[0].regions().iter().cloned(),
        source.panel_instances().iter().cloned(),
        right_ratio,
    )
}

fn assert_code(document: LayoutDocument, expected: LayoutValidationCode) {
    assert_eq!(
        validate_document(&registry(), &document)
            .unwrap_err()
            .code(),
        expected
    );
}
