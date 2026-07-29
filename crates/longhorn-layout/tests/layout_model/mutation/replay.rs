use longhorn_core::LayoutRevision;
use longhorn_layout::{
    BoundedLayoutReplayStore, LayoutMutationCommand, LayoutMutationEngine,
    LayoutMutationRejectionCode, LayoutMutationRequest, LayoutReplayStoreError,
};

use crate::support::*;

#[test]
fn replay_requires_an_explicit_store_and_matches_exact_request_content() {
    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = document();
    let request = activate_request("request:replay", source.revision());
    let first_without_store = engine.apply(&source, &request).unwrap();
    assert_eq!(
        engine
            .apply(first_without_store.authoritative_document(), &request)
            .unwrap_err()
            .code(),
        LayoutMutationRejectionCode::StaleRevision
    );

    let mut store = BoundedLayoutReplayStore::new(2).unwrap();
    let first = engine
        .apply_with_replay(&source, &request, &mut store)
        .unwrap();
    let replayed = engine
        .apply_with_replay(first.authoritative_document(), &request, &mut store)
        .unwrap();
    assert_eq!(replayed, first);
    assert_eq!(store.len(), 1);

    let conflict = LayoutMutationRequest::new(
        request_id("request:replay"),
        source.revision(),
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:chat"),
        },
    );
    let rejection = engine
        .apply_with_replay(first.authoritative_document(), &conflict, &mut store)
        .unwrap_err();
    assert_eq!(
        rejection.code(),
        LayoutMutationRejectionCode::RequestIdConflict
    );
    assert_eq!(
        rejection.authoritative_document(),
        first.authoritative_document()
    );
}

#[test]
fn replay_store_is_finite_and_evicts_oldest_success() {
    assert_eq!(
        BoundedLayoutReplayStore::new(0).unwrap_err(),
        LayoutReplayStoreError::ZeroCapacity
    );
    assert!(matches!(
        BoundedLayoutReplayStore::new(4_097).unwrap_err(),
        LayoutReplayStoreError::ExceedsHardMaximum { .. }
    ));

    let registry = registry();
    let engine = LayoutMutationEngine::new(&registry);
    let source = document();
    let mut store = BoundedLayoutReplayStore::new(1).unwrap();
    let first_request = activate_request("request:first", source.revision());
    let first = engine
        .apply_with_replay(&source, &first_request, &mut store)
        .unwrap();
    let second_request = LayoutMutationRequest::new(
        request_id("request:second"),
        first.committed_revision(),
        LayoutMutationCommand::SetRegionCollapsed {
            container_id: container_id("container:primary"),
            region_id: region_id("right"),
            collapsed: true,
        },
    );
    let second = engine
        .apply_with_replay(first.authoritative_document(), &second_request, &mut store)
        .unwrap();
    assert_eq!(store.len(), 1);
    assert_eq!(
        engine
            .apply_with_replay(second.authoritative_document(), &first_request, &mut store)
            .unwrap_err()
            .code(),
        LayoutMutationRejectionCode::StaleRevision
    );
}

fn activate_request(id: &str, revision: LayoutRevision) -> LayoutMutationRequest {
    LayoutMutationRequest::new(
        request_id(id),
        revision,
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:tool"),
        },
    )
}
