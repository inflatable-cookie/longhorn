use longhorn_core::{ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize};
use longhorn_native_content_prototype::{
    AttachGeneration, ContentSizeDecision, ContentSizeProposal, CoordinationError, DesiredPresence,
    DesiredVisibility, FocusIntent, InputRoutingMode, NativeContentMechanism,
    NativeContentOperation, NativeContentRevision, ObservationUpdate, ObservedGeometry,
    decide_content_size,
};

use super::support::{attached_observation, coordinator, desired_update, viewport};

#[test]
fn nucleus_child_trace_maps_viewport_to_child_bounds_and_converges() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    assert!(matches!(
        plan.operations()[0].operation(),
        NativeContentOperation::Attach {
            mechanism: NativeContentMechanism::ChildView,
            ..
        }
    ));
    assert!(matches!(
        plan.operations()[1].operation(),
        NativeContentOperation::SetChildBounds { .. }
    ));
    assert!(plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetInputRouting {
            mode: InputRoutingMode::NativeDirect
        }
    )));

    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(
                NativeContentMechanism::ChildView,
                1,
                InputRoutingMode::NativeDirect,
            ),
        )
        .unwrap();
    assert!(coordinator.plan().unwrap().is_empty());

    let hidden = desired_update(
        1,
        DesiredPresence::Present,
        DesiredVisibility::Hidden {
            reason: "overlay:menu".parse().unwrap(),
        },
        FocusIntent::Unchanged,
        InputRoutingMode::NativeDirect,
        2000,
        viewport(),
    );
    coordinator
        .update_desired(NativeContentRevision::INITIAL, hidden)
        .unwrap();
    let hidden_plan = coordinator.plan().unwrap();
    assert_eq!(hidden_plan.operations().len(), 1);
    assert!(matches!(
        hidden_plan.operations()[0].operation(),
        NativeContentOperation::Hide { reason } if reason.as_str() == "overlay:menu"
    ));
}

#[test]
fn soundcheck_trace_uses_content_size_and_non_mutating_consumer_decision() {
    let coordinator = coordinator(NativeContentMechanism::IsolatedWindow);
    let plan = coordinator.plan().unwrap();
    assert!(plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetIsolatedContentSize { size }
            if *size == PhysicalSize::new(640, 360)
    )));
    assert!(!plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetChildBounds { .. }
            | NativeContentOperation::SetBackingViewport { .. }
    )));

    let original = coordinator.desired().clone();
    let proposal = ContentSizeProposal::new(
        coordinator.desired().generation(),
        coordinator.desired().revision(),
        ClientSize::new(801.0, 601.0).unwrap(),
    );
    let receipt = decide_content_size(
        coordinator.desired(),
        proposal,
        ContentSizeDecision::Constrained {
            size: ClientSize::new(800.0, 600.0).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        receipt.accepted_size(),
        Some(ClientSize::new(800.0, 600.0).unwrap())
    );
    assert_eq!(coordinator.desired(), &original);
}

#[test]
fn jetstream_trace_keeps_storage_bounds_distinct_from_viewport_clip() {
    let mut coordinator = coordinator(NativeContentMechanism::BackingSurface);
    let plan = coordinator.plan().unwrap();
    assert!(plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetBackingViewport { .. }
    )));
    assert!(!plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetChildBounds { .. }
            | NativeContentOperation::SetIsolatedContentSize { .. }
    )));

    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(
                NativeContentMechanism::BackingSurface,
                1,
                InputRoutingMode::RendererForwarded,
            ),
        )
        .unwrap();
    let unknown_visibility = coordinator.plan().unwrap();
    assert_eq!(unknown_visibility.operations().len(), 1);
    assert!(matches!(
        unknown_visibility.operations()[0].operation(),
        NativeContentOperation::Show
    ));

    coordinator
        .admit_observation(
            NativeContentRevision::new(1),
            ObservationUpdate::new(
                coordinator.desired().generation(),
                longhorn_native_content_prototype::AttachmentLifecycle::Attached,
                longhorn_native_content_prototype::ObservedReadiness::Ready,
                longhorn_native_content_prototype::EffectiveVisibility::Unknown,
                longhorn_native_content_prototype::EffectiveFocus::Unknown,
                ObservedGeometry::BackingSurface {
                    storage_bounds: PhysicalRect::new(
                        PhysicalPoint::new(0, 0),
                        PhysicalSize::new(2400, 1600),
                    ),
                    clip: super::support::physical_viewport(2000),
                },
                Some(InputRoutingMode::RendererForwarded),
            ),
        )
        .unwrap();
    let unknown_visibility = coordinator.plan().unwrap();
    assert_eq!(unknown_visibility.operations().len(), 1);
    assert!(matches!(
        unknown_visibility.operations()[0].operation(),
        NativeContentOperation::Show
    ));
}

#[test]
fn content_size_proposals_are_capability_revision_and_generation_bound() {
    let isolated = coordinator(NativeContentMechanism::IsolatedWindow);
    let size = ClientSize::new(640.0, 480.0).unwrap();
    let rejected = decide_content_size(
        isolated.desired(),
        ContentSizeProposal::new(
            AttachGeneration::INITIAL,
            NativeContentRevision::INITIAL,
            size,
        ),
        ContentSizeDecision::Rejected {
            code: "policy:size-rejected".parse().unwrap(),
        },
    )
    .unwrap();
    assert_eq!(rejected.accepted_size(), None);

    assert!(matches!(
        decide_content_size(
            isolated.desired(),
            ContentSizeProposal::new(
                AttachGeneration::new(2),
                NativeContentRevision::INITIAL,
                size,
            ),
            ContentSizeDecision::Accepted,
        ),
        Err(CoordinationError::FutureGeneration { .. })
    ));
    assert!(matches!(
        decide_content_size(
            isolated.desired(),
            ContentSizeProposal::new(
                AttachGeneration::INITIAL,
                NativeContentRevision::new(9),
                size,
            ),
            ContentSizeDecision::Accepted,
        ),
        Err(CoordinationError::StaleRevision { .. })
    ));

    let child = coordinator(NativeContentMechanism::ChildView);
    assert_eq!(
        decide_content_size(
            child.desired(),
            ContentSizeProposal::new(
                AttachGeneration::INITIAL,
                NativeContentRevision::INITIAL,
                size,
            ),
            ContentSizeDecision::Accepted,
        ),
        Err(CoordinationError::ContentSizeRequestsUnsupported)
    );
}
