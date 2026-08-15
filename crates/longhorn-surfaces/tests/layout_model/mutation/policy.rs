use longhorn_surfaces::{
    LayoutMutationCommand, LayoutMutationEngine, LayoutMutationRejectionCode,
    LayoutMutationRequest, SurfaceDocument,
};

use crate::support::*;

#[test]
fn singleton_one_per_surface_bounded_and_multiple_are_enforced() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = two_surface_document();

    let singleton = request(
        "request:singleton",
        &source,
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:chat-2"),
            panel_definition_id: definition_id("panel:chat"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("center"),
            insertion_index: 0,
        },
    );
    assert_eq!(
        engine.apply(&source, &singleton).unwrap_err().code(),
        LayoutMutationRejectionCode::InstancePolicyExceeded
    );

    let activity = request(
        "request:activity-secondary",
        &source,
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:activity-2"),
            panel_definition_id: definition_id("panel:activity"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("left"),
            insertion_index: 0,
        },
    );
    let activity_receipt = engine.apply(&source, &activity).unwrap();
    let activity_again = request(
        "request:activity-secondary-2",
        activity_receipt.authoritative_document(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:activity-3"),
            panel_definition_id: definition_id("panel:activity"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("left"),
            insertion_index: 1,
        },
    );
    assert_eq!(
        engine
            .apply(activity_receipt.authoritative_document(), &activity_again)
            .unwrap_err()
            .code(),
        LayoutMutationRejectionCode::InstancePolicyExceeded
    );

    let bounded_one = request(
        "request:bounded-1",
        &source,
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:bounded-1"),
            panel_definition_id: definition_id("panel:bounded"),
            surface_id: surface_id("surface:primary"),
            region_id: region_id("center"),
            insertion_index: 2,
        },
    );
    let bounded_one = engine.apply(&source, &bounded_one).unwrap();
    let bounded_same_surface = request(
        "request:bounded-same",
        bounded_one.authoritative_document(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:bounded-2"),
            panel_definition_id: definition_id("panel:bounded"),
            surface_id: surface_id("surface:primary"),
            region_id: region_id("center"),
            insertion_index: 3,
        },
    );
    assert_eq!(
        engine
            .apply(bounded_one.authoritative_document(), &bounded_same_surface)
            .unwrap_err()
            .code(),
        LayoutMutationRejectionCode::InstancePolicyExceeded
    );

    let bounded_two = request(
        "request:bounded-2",
        bounded_one.authoritative_document(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:bounded-2"),
            panel_definition_id: definition_id("panel:bounded"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("center"),
            insertion_index: 0,
        },
    );
    let bounded_two = engine
        .apply(bounded_one.authoritative_document(), &bounded_two)
        .unwrap();
    let bounded_three = request(
        "request:bounded-3",
        bounded_two.authoritative_document(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:bounded-3"),
            panel_definition_id: definition_id("panel:bounded"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("center"),
            insertion_index: 1,
        },
    );
    assert_eq!(
        engine
            .apply(bounded_two.authoritative_document(), &bounded_three)
            .unwrap_err()
            .code(),
        LayoutMutationRejectionCode::InstancePolicyExceeded
    );

    let tool_one = request(
        "request:tool-2",
        &source,
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:tool-2"),
            panel_definition_id: definition_id("panel:tool"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("right"),
            insertion_index: 0,
        },
    );
    let tool_one = engine.apply(&source, &tool_one).unwrap();
    let tool_two = request(
        "request:tool-3",
        tool_one.authoritative_document(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:tool-3"),
            panel_definition_id: definition_id("panel:tool"),
            surface_id: surface_id("surface:secondary"),
            region_id: region_id("right"),
            insertion_index: 1,
        },
    );
    engine
        .apply(tool_one.authoritative_document(), &tool_two)
        .unwrap();
}

fn request(
    id: &str,
    document: &SurfaceDocument,
    command: LayoutMutationCommand,
) -> LayoutMutationRequest {
    LayoutMutationRequest::new(request_id(id), document.revision(), command)
}
