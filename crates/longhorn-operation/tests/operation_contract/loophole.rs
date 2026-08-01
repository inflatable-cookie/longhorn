use longhorn_operation::OperationState;

use super::support::*;

#[test]
fn loophole_queue_uses_registration_order_without_sharing_scheduler_policy() {
    let mut catalogue = catalogue("authority:loophole", 11);
    catalogue
        .register(scoped_registration(
            &catalogue,
            "render:mix-a",
            "loophole.render",
            "project:alpha",
            "Export Mix A",
            OperationState::Queued,
        ))
        .unwrap();
    catalogue
        .register(scoped_registration(
            &catalogue,
            "render:mix-b",
            "loophole.render",
            "project:alpha",
            "Export Mix B",
            OperationState::Queued,
        ))
        .unwrap();

    assert_eq!(
        catalogue
            .project()
            .active()
            .iter()
            .map(|operation| operation.operation_id().as_str())
            .collect::<Vec<_>>(),
        vec!["render:mix-a", "render:mix-b"]
    );

    catalogue
        .transition(transition(
            &catalogue,
            "render:mix-a",
            OperationState::Running,
        ))
        .unwrap();
    catalogue
        .transition(transition(
            &catalogue,
            "render:mix-b",
            OperationState::Cancelled,
        ))
        .unwrap();
    catalogue
        .transition(transition(
            &catalogue,
            "render:mix-a",
            OperationState::Succeeded,
        ))
        .unwrap();

    let projection = catalogue.project();
    assert!(projection.active().is_empty());
    assert_eq!(
        projection
            .recent()
            .iter()
            .map(|operation| operation.operation_id().as_str())
            .collect::<Vec<_>>(),
        vec!["render:mix-a", "render:mix-b"]
    );
    assert_eq!(projection.recent()[0].sequence().get(), 1);
    assert_eq!(projection.recent()[1].sequence().get(), 2);
    assert!(
        projection.recent()[0].last_changed_catalogue_revision()
            > projection.recent()[1].last_changed_catalogue_revision()
    );
}
