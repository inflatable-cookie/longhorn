use longhorn_core::LayoutRevision;
use longhorn_layout::{
    LayoutContainer, LayoutDocument, LayoutMutationCommand, LayoutMutationEngine,
    LayoutMutationRejectionCode, LayoutMutationRequest, RegionState,
};

use crate::support::*;

#[test]
fn admission_and_policy_failures_preserve_exact_source() {
    assert_rejected(
        document(),
        LayoutRevision::new(6),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
        LayoutMutationRejectionCode::StaleRevision,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:chat"),
            panel_definition_id: definition_id("panel:tool"),
            container_id: container_id("container:primary"),
            region_id: region_id("right"),
            insertion_index: 0,
        },
        LayoutMutationRejectionCode::DuplicatePanelInstance,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:tool-left"),
            panel_definition_id: definition_id("panel:tool"),
            container_id: container_id("container:primary"),
            region_id: region_id("left"),
            insertion_index: 1,
        },
        LayoutMutationRejectionCode::PanelPlacementNotAllowed,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::ClosePanel {
            panel_instance_id: instance_id("instance:activity"),
        },
        LayoutMutationRejectionCode::PanelNotCloseable,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:activity"),
            target_container_id: container_id("container:primary"),
            target_region_id: region_id("center"),
            insertion_index: 0,
        },
        LayoutMutationRejectionCode::PanelNotMovable,
    );
}

#[test]
fn reorder_and_position_failures_preserve_exact_source() {
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::ReorderRegion {
            container_id: container_id("container:primary"),
            region_id: region_id("center"),
            panel_instance_ids: vec![instance_id("instance:chat")],
        },
        LayoutMutationRejectionCode::IncompleteReorder,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::ReorderRegion {
            container_id: container_id("container:primary"),
            region_id: region_id("center"),
            panel_instance_ids: vec![instance_id("instance:chat"), instance_id("instance:chat")],
        },
        LayoutMutationRejectionCode::DuplicateReorderMember,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::ReorderRegion {
            container_id: container_id("container:primary"),
            region_id: region_id("center"),
            panel_instance_ids: vec![
                instance_id("instance:chat"),
                instance_id("instance:activity"),
            ],
        },
        LayoutMutationRejectionCode::ForeignReorderMember,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:tool-2"),
            panel_definition_id: definition_id("panel:tool"),
            container_id: container_id("container:primary"),
            region_id: region_id("right"),
            insertion_index: 1,
        },
        LayoutMutationRejectionCode::InvalidInsertionIndex,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:tool"),
            target_container_id: container_id("container:primary"),
            target_region_id: region_id("center"),
            insertion_index: 0,
        },
        LayoutMutationRejectionCode::MoveTargetUnchanged,
    );
}

#[test]
fn sizing_collapse_overflow_and_invalid_source_fail_unchanged() {
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::SetSizingSlot {
            container_id: container_id("container:primary"),
            sizing_slot_id: slot_id("right-width"),
            ratio: ratio(600_000),
        },
        LayoutMutationRejectionCode::InvalidSizingRatio,
    );
    assert_rejected(
        document(),
        LayoutRevision::new(7),
        LayoutMutationCommand::SetRegionCollapsed {
            container_id: container_id("container:primary"),
            region_id: region_id("center"),
            collapsed: true,
        },
        LayoutMutationRejectionCode::UnsupportedCollapse,
    );

    let source = document();
    let overflow = LayoutDocument::new(
        LayoutRevision::new(u64::MAX),
        source.containers().iter().cloned(),
        source.panel_instances().iter().cloned(),
    );
    assert_rejected(
        overflow,
        LayoutRevision::new(u64::MAX),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
        LayoutMutationRejectionCode::RevisionOverflow,
    );

    let source = document();
    let invalid = LayoutDocument::new(
        source.revision(),
        [LayoutContainer::new(
            container_id("container:primary"),
            schema_id("schema:workspace"),
            [
                source.containers()[0].regions()[0].clone(),
                RegionState::new(region_id("right"), [], None, Some(false)),
            ],
            source.containers()[0].sizing_slots().iter().cloned(),
        )],
        source.panel_instances().iter().cloned(),
    );
    assert_rejected(
        invalid,
        LayoutRevision::new(7),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
        LayoutMutationRejectionCode::InvalidCurrentDocument,
    );
}

#[test]
fn rejection_envelope_is_strict_and_round_trips() {
    let source = document();
    let request = LayoutMutationRequest::new(
        request_id("request:strict-rejection"),
        LayoutRevision::new(6),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
    );
    let rejection = LayoutMutationEngine::new(&registry())
        .apply(&source, &request)
        .unwrap_err();
    let encoded = serde_json::to_value(&rejection).unwrap();
    assert_eq!(
        serde_json::from_value::<longhorn_layout::LayoutMutationRejection>(encoded.clone())
            .unwrap(),
        rejection
    );
    let mut unknown = encoded;
    unknown
        .as_object_mut()
        .unwrap()
        .insert("future".into(), true.into());
    assert!(serde_json::from_value::<longhorn_layout::LayoutMutationRejection>(unknown).is_err());
}

fn assert_rejected(
    source: LayoutDocument,
    expected_revision: LayoutRevision,
    command: LayoutMutationCommand,
    expected_code: LayoutMutationRejectionCode,
) {
    let before = serde_json::to_vec(&source).unwrap();
    let request =
        LayoutMutationRequest::new(request_id("request:rejection"), expected_revision, command);
    let rejection = LayoutMutationEngine::new(&registry())
        .apply(&source, &request)
        .unwrap_err();

    assert_eq!(rejection.code(), expected_code);
    assert_eq!(rejection.current_revision(), source.revision());
    assert_eq!(rejection.authoritative_document(), &source);
    assert_eq!(serde_json::to_vec(&source).unwrap(), before);
}
