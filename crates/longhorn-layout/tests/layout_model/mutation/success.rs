use longhorn_core::LayoutRevision;
use longhorn_layout::{
    LayoutContainer, LayoutDocument, LayoutMutationCommand, LayoutMutationEngine,
    LayoutMutationOutcome, LayoutMutationRequest,
};

use crate::support::*;

#[test]
fn create_commits_one_revision_activates_and_round_trips() {
    let registry = registry();
    let source = document();
    let request = LayoutMutationRequest::new(
        request_id("request:create"),
        source.revision(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:tool-2"),
            panel_definition_id: definition_id("panel:tool"),
            container_id: container_id("container:primary"),
            region_id: region_id("right"),
            insertion_index: 0,
        },
    );

    let receipt = LayoutMutationEngine::new(&registry)
        .apply(&source, &request)
        .unwrap();
    let target = receipt
        .authoritative_document()
        .container(&container_id("container:primary"))
        .unwrap()
        .region(&region_id("right"))
        .unwrap();

    assert_eq!(source.revision(), LayoutRevision::new(7));
    assert_eq!(receipt.previous_revision(), LayoutRevision::new(7));
    assert_eq!(receipt.committed_revision(), LayoutRevision::new(8));
    assert_eq!(
        target.panel_instance_ids(),
        &[instance_id("instance:tool-2")]
    );
    assert_eq!(
        target.active_panel_instance_id(),
        Some(&instance_id("instance:tool-2"))
    );
    assert!(matches!(
        receipt.outcome(),
        LayoutMutationOutcome::PanelCreated {
            insertion_index: 0,
            ..
        }
    ));

    let request_json = serde_json::to_vec(&request).unwrap();
    let receipt_json = serde_json::to_vec(&receipt).unwrap();
    assert_eq!(
        serde_json::from_slice::<LayoutMutationRequest>(&request_json).unwrap(),
        request
    );
    assert_eq!(
        serde_json::from_slice::<longhorn_layout::LayoutMutationReceipt>(&receipt_json).unwrap(),
        receipt
    );
}

#[test]
fn close_uses_former_index_then_previous_final_fallback() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = document();
    let close_first = LayoutMutationRequest::new(
        request_id("request:close-first"),
        source.revision(),
        LayoutMutationCommand::ClosePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
    );
    let first = engine.apply(&source, &close_first).unwrap();
    let center = first.authoritative_document().containers()[0]
        .region(&region_id("center"))
        .unwrap();
    assert_eq!(center.panel_instance_ids(), &[instance_id("instance:tool")]);
    assert_eq!(
        center.active_panel_instance_id(),
        Some(&instance_id("instance:tool"))
    );

    let activate_last = LayoutMutationRequest::new(
        request_id("request:activate-last"),
        source.revision(),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    );
    let activated = engine.apply(&source, &activate_last).unwrap();
    let close_last = LayoutMutationRequest::new(
        request_id("request:close-last"),
        activated.committed_revision(),
        LayoutMutationCommand::ClosePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    );
    let last = engine
        .apply(activated.authoritative_document(), &close_last)
        .unwrap();
    let center = last.authoritative_document().containers()[0]
        .region(&region_id("center"))
        .unwrap();
    assert_eq!(center.panel_instance_ids(), &[instance_id("instance:chat")]);
    assert_eq!(
        center.active_panel_instance_id(),
        Some(&instance_id("instance:chat"))
    );
}

#[test]
fn activate_and_complete_reorder_preserve_requested_active_member() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = document();
    let activate = LayoutMutationRequest::new(
        request_id("request:activate"),
        source.revision(),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    );
    let activated = engine.apply(&source, &activate).unwrap();
    let reorder = LayoutMutationRequest::new(
        request_id("request:reorder"),
        activated.committed_revision(),
        LayoutMutationCommand::ReorderRegion {
            container_id: container_id("container:primary"),
            region_id: region_id("center"),
            panel_instance_ids: vec![instance_id("instance:tool"), instance_id("instance:chat")],
        },
    );
    let reordered = engine
        .apply(activated.authoritative_document(), &reorder)
        .unwrap();
    let center = reordered.authoritative_document().containers()[0]
        .region(&region_id("center"))
        .unwrap();

    assert_eq!(
        center.panel_instance_ids(),
        &[instance_id("instance:tool"), instance_id("instance:chat")]
    );
    assert_eq!(
        center.active_panel_instance_id(),
        Some(&instance_id("instance:tool"))
    );
}

#[test]
fn move_is_one_atomic_remove_insert_with_exact_active_fallback() {
    let registry = registry();
    let source = document();
    let request = LayoutMutationRequest::new(
        request_id("request:move"),
        source.revision(),
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:chat"),
            target_container_id: container_id("container:primary"),
            target_region_id: region_id("right"),
            insertion_index: 0,
        },
    );
    let receipt = LayoutMutationEngine::new(&registry)
        .apply(&source, &request)
        .unwrap();
    let container = &receipt.authoritative_document().containers()[0];
    let center = container.region(&region_id("center")).unwrap();
    let right = container.region(&region_id("right")).unwrap();

    assert_eq!(center.panel_instance_ids(), &[instance_id("instance:tool")]);
    assert_eq!(
        center.active_panel_instance_id(),
        Some(&instance_id("instance:tool"))
    );
    assert_eq!(right.panel_instance_ids(), &[instance_id("instance:chat")]);
    assert_eq!(
        right.active_panel_instance_id(),
        Some(&instance_id("instance:chat"))
    );
}

#[test]
fn move_crosses_container_without_importing_host_identity() {
    let registry = registry();
    let source = two_container_document();
    let request = LayoutMutationRequest::new(
        request_id("request:cross-container"),
        source.revision(),
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:chat"),
            target_container_id: container_id("container:secondary"),
            target_region_id: region_id("center"),
            insertion_index: 0,
        },
    );
    let receipt = LayoutMutationEngine::new(&registry)
        .apply(&source, &request)
        .unwrap();
    let primary = receipt
        .authoritative_document()
        .container(&container_id("container:primary"))
        .unwrap();
    let secondary = receipt
        .authoritative_document()
        .container(&container_id("container:secondary"))
        .unwrap();

    assert_eq!(
        primary
            .region(&region_id("center"))
            .unwrap()
            .panel_instance_ids(),
        &[instance_id("instance:tool")]
    );
    assert_eq!(
        secondary
            .region(&region_id("center"))
            .unwrap()
            .panel_instance_ids(),
        &[instance_id("instance:chat")]
    );
}

#[test]
fn sizing_and_collapse_commit_explicit_values() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = document();
    let sizing = LayoutMutationRequest::new(
        request_id("request:sizing"),
        source.revision(),
        LayoutMutationCommand::SetSizingSlot {
            container_id: container_id("container:primary"),
            sizing_slot_id: slot_id("right-width"),
            ratio: ratio(400_000),
        },
    );
    let sized = engine.apply(&source, &sizing).unwrap();
    let collapse = LayoutMutationRequest::new(
        request_id("request:collapse"),
        sized.committed_revision(),
        LayoutMutationCommand::SetRegionCollapsed {
            container_id: container_id("container:primary"),
            region_id: region_id("right"),
            collapsed: true,
        },
    );
    let collapsed = engine
        .apply(sized.authoritative_document(), &collapse)
        .unwrap();
    let container = &collapsed.authoritative_document().containers()[0];

    assert_eq!(
        container
            .sizing_slot(&slot_id("right-width"))
            .unwrap()
            .ratio(),
        ratio(400_000)
    );
    assert_eq!(
        container.region(&region_id("right")).unwrap().collapsed(),
        Some(true)
    );
    assert_eq!(collapsed.committed_revision(), LayoutRevision::new(9));
}

#[test]
fn structural_input_permutations_produce_one_receipt() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let canonical = document();
    let source_container = &canonical.containers()[0];
    let permuted = LayoutDocument::new(
        canonical.revision(),
        [LayoutContainer::new(
            source_container.id().clone(),
            source_container.schema_id().clone(),
            source_container.regions().iter().rev().cloned(),
            source_container.sizing_slots().iter().rev().cloned(),
        )],
        canonical.panel_instances().iter().rev().cloned(),
    );
    let request = LayoutMutationRequest::new(
        request_id("request:permutation"),
        canonical.revision(),
        LayoutMutationCommand::SetSizingSlot {
            container_id: container_id("container:primary"),
            sizing_slot_id: slot_id("left-width"),
            ratio: ratio(300_000),
        },
    );

    assert_eq!(
        engine.apply(&canonical, &request).unwrap(),
        engine.apply(&permuted, &request).unwrap()
    );
}

#[test]
fn request_and_command_envelopes_reject_unknown_fields() {
    let request = LayoutMutationRequest::new(
        request_id("request:strict"),
        LayoutRevision::new(7),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
    );
    let mut request_value = serde_json::to_value(request).unwrap();
    request_value
        .as_object_mut()
        .unwrap()
        .insert("future".into(), true.into());
    assert!(serde_json::from_value::<LayoutMutationRequest>(request_value).is_err());

    let mut command_value = serde_json::to_value(LayoutMutationCommand::ActivatePanel {
        panel_instance_id: instance_id("instance:chat"),
    })
    .unwrap();
    command_value
        .as_object_mut()
        .unwrap()
        .insert("future".into(), true.into());
    assert!(serde_json::from_value::<LayoutMutationCommand>(command_value).is_err());
}
