//! Contract evidence for pure native-content coordination.

use std::{fs, path::Path};

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, NativeContentRevision, PhysicalPoint, PhysicalRect,
    PhysicalSize, RoundingMode, ScaleFactor, WindowId,
};
use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, ContentSizeDecision, ContentSizeProposal,
    CoordinationError, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility,
    DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent, HostDestroyOutcome,
    InputRoutingMode, MechanismCapabilities, NativeContentCoordinator, NativeContentFailureCode,
    NativeContentIslandId, NativeContentKindId, NativeContentMechanism, NativeContentOperation,
    ObservationUpdate, ObservedGeometry, ObservedReadiness, OperationOutcome, PlanStepId,
    PositiveCounterError, ReceiptError, StepExecution, ViewportConversionError,
    viewport_to_physical,
};

fn generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

fn viewport() -> ClientRect {
    ClientRect::new(
        ClientPoint::new(10.25, 20.5).unwrap(),
        ClientSize::new(320.0, 180.0).unwrap(),
    )
}

fn physical_viewport() -> PhysicalRect {
    PhysicalRect::new(PhysicalPoint::new(21, 41), PhysicalSize::new(640, 360))
}

fn capabilities(mechanism: NativeContentMechanism) -> MechanismCapabilities {
    match mechanism {
        NativeContentMechanism::ChildView => MechanismCapabilities::new(
            mechanism,
            InputRoutingMode::NativeDirect,
            false,
            DetachPolicy::Reversible,
            true,
            true,
        ),
        NativeContentMechanism::IsolatedWindow => MechanismCapabilities::new(
            mechanism,
            InputRoutingMode::NativeDirect,
            true,
            DetachPolicy::OwnerProcessTermination,
            true,
            true,
        ),
        NativeContentMechanism::BackingSurface => MechanismCapabilities::new(
            mechanism,
            InputRoutingMode::RendererForwarded,
            false,
            DetachPolicy::ProcessLifetime,
            false,
            false,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn desired_update(
    generation: u64,
    host: &str,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    input: InputRoutingMode,
    viewport: ClientRect,
) -> DesiredUpdate {
    DesiredUpdate::new(
        self::generation(generation),
        WindowId::new(host).unwrap(),
        viewport,
        ScaleFactor::from_thousandths(2000).unwrap(),
        RoundingMode::Nearest,
        presence,
        visibility,
        focus,
        input,
    )
}

fn coordinator(mechanism: NativeContentMechanism) -> NativeContentCoordinator {
    let input = if mechanism == NativeContentMechanism::BackingSurface {
        InputRoutingMode::RendererForwarded
    } else {
        InputRoutingMode::NativeDirect
    };
    let focus = if mechanism == NativeContentMechanism::BackingSurface {
        FocusIntent::Unchanged
    } else {
        FocusIntent::Request
    };
    let desired = DesiredState::new(
        NativeContentIslandId::new("island:fixture").unwrap(),
        NativeContentKindId::new("fixture:content").unwrap(),
        capabilities(mechanism),
        desired_update(
            1,
            "window:main",
            DesiredPresence::Present,
            DesiredVisibility::Visible,
            focus,
            input,
            viewport(),
        ),
    )
    .unwrap();
    NativeContentCoordinator::new(desired)
}

fn attached_observation(mechanism: NativeContentMechanism, generation: u64) -> ObservationUpdate {
    let geometry = match mechanism {
        NativeContentMechanism::ChildView => ObservedGeometry::ChildBounds {
            bounds: physical_viewport(),
        },
        NativeContentMechanism::IsolatedWindow => ObservedGeometry::IsolatedContent {
            size: physical_viewport().size(),
        },
        NativeContentMechanism::BackingSurface => ObservedGeometry::BackingSurface {
            storage_bounds: PhysicalRect::new(
                PhysicalPoint::new(0, 0),
                PhysicalSize::new(4096, 2160),
            ),
            clip: physical_viewport(),
        },
    };
    let observable = mechanism != NativeContentMechanism::BackingSurface;
    ObservationUpdate::new(
        self::generation(generation),
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        if observable {
            EffectiveVisibility::Visible
        } else {
            EffectiveVisibility::Unknown
        },
        if observable {
            EffectiveFocus::Focused
        } else {
            EffectiveFocus::Unknown
        },
        geometry,
        Some(if mechanism == NativeContentMechanism::BackingSurface {
            InputRoutingMode::RendererForwarded
        } else {
            InputRoutingMode::NativeDirect
        }),
    )
}

#[test]
fn counters_and_domain_ids_reject_invalid_serialized_values() {
    assert_eq!(
        AttachGeneration::new(0),
        Err(PositiveCounterError::AttachGenerationZero)
    );
    assert_eq!(PlanStepId::new(0), Err(PositiveCounterError::PlanStepZero));
    assert!(serde_json::from_str::<AttachGeneration>("0").is_err());
    assert!(serde_json::from_str::<PlanStepId>("0").is_err());
    assert!(
        AttachGeneration::new(u64::MAX)
            .unwrap()
            .checked_next()
            .is_err()
    );
    assert!(NativeContentIslandId::new("island:browser-1").is_ok());
    assert!(NativeContentIslandId::new("Uppercase").is_err());
    assert!(NativeContentFailureCode::new("x".repeat(129)).is_err());
    assert!(NativeContentRevision::new(u64::MAX).checked_next().is_err());

    let desired = coordinator(NativeContentMechanism::ChildView)
        .desired()
        .clone();
    let mut json = serde_json::to_value(desired).unwrap();
    json["input_routing"] = serde_json::json!("renderer_forwarded");
    assert!(serde_json::from_value::<DesiredState>(json).is_err());
}

#[test]
fn viewport_conversion_is_explicit_and_checked() {
    let input = ClientRect::new(
        ClientPoint::new(-10.5, 2.25).unwrap(),
        ClientSize::new(100.5, 50.25).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            input,
            ScaleFactor::from_thousandths(2000).unwrap(),
            RoundingMode::Nearest,
        )
        .unwrap(),
        PhysicalRect::new(PhysicalPoint::new(-21, 5), PhysicalSize::new(201, 101))
    );
    let overflow = ClientRect::new(
        ClientPoint::new(f64::MAX, 0.0).unwrap(),
        ClientSize::new(1.0, 1.0).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            overflow,
            ScaleFactor::from_thousandths(2000).unwrap(),
            RoundingMode::Nearest,
        ),
        Err(ViewportConversionError::CoordinateOverflow)
    );
    assert!(ScaleFactor::from_thousandths(0).is_err());
    assert!(ClientSize::new(f64::NAN, 1.0).is_err());
}

#[test]
fn three_shapes_share_one_vocabulary_without_geometry_collapse() {
    let child = coordinator(NativeContentMechanism::ChildView)
        .plan()
        .unwrap();
    assert!(matches!(
        child.operations()[0].operation(),
        NativeContentOperation::Attach {
            mechanism: NativeContentMechanism::ChildView,
            ..
        }
    ));
    assert!(child.operations().iter().any(|step| matches!(step.operation(), NativeContentOperation::SetChildBounds { bounds } if *bounds == physical_viewport())));

    let isolated = coordinator(NativeContentMechanism::IsolatedWindow)
        .plan()
        .unwrap();
    assert!(isolated.operations().iter().any(|step| matches!(step.operation(), NativeContentOperation::SetIsolatedContentSize { size } if *size == PhysicalSize::new(640, 360))));
    assert!(!isolated.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetChildBounds { .. }
            | NativeContentOperation::SetBackingViewport { .. }
    )));

    let backing = coordinator(NativeContentMechanism::BackingSurface)
        .plan()
        .unwrap();
    assert!(backing.operations().iter().any(|step| matches!(step.operation(), NativeContentOperation::SetBackingViewport { clip } if *clip == physical_viewport())));
    assert!(backing.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetInputRouting {
            mode: InputRoutingMode::RendererForwarded
        }
    )));
}

#[test]
fn fresh_observation_converges_observable_shapes() {
    for mechanism in [
        NativeContentMechanism::ChildView,
        NativeContentMechanism::IsolatedWindow,
    ] {
        let mut coordinator = coordinator(mechanism);
        coordinator
            .admit_observation(
                NativeContentRevision::INITIAL,
                attached_observation(mechanism, 1),
            )
            .unwrap();
        assert!(coordinator.plan().unwrap().is_empty());
    }
}

#[test]
fn backing_storage_bounds_do_not_drive_viewport_convergence() {
    let mut coordinator = coordinator(NativeContentMechanism::BackingSurface);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::BackingSurface, 1),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    assert_eq!(plan.operations().len(), 1);
    assert!(matches!(
        plan.operations()[0].operation(),
        NativeContentOperation::Show
    ));
}

#[test]
fn invalid_desired_input_host_and_generation_changes_are_atomic() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    let before = coordinator.clone();
    let unsupported = desired_update(
        1,
        "window:main",
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::RendererForwarded,
        viewport(),
    );
    assert!(matches!(
        coordinator.update_desired(NativeContentRevision::INITIAL, unsupported),
        Err(CoordinationError::UnsupportedInputRouting { .. })
    ));
    assert_eq!(coordinator, before);

    let host_change = desired_update(
        1,
        "window:other",
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
        viewport(),
    );
    assert_eq!(
        coordinator.update_desired(NativeContentRevision::INITIAL, host_change),
        Err(CoordinationError::HostChangeRequiresGeneration)
    );
    assert_eq!(coordinator, before);

    let gap = desired_update(
        3,
        "window:main",
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
        viewport(),
    );
    assert!(matches!(
        coordinator.update_desired(NativeContentRevision::INITIAL, gap),
        Err(CoordinationError::GenerationGap { .. })
    ));
    assert_eq!(coordinator, before);
}

#[test]
fn stale_revision_and_overflowing_geometry_leave_exact_state_unchanged() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                "window:main",
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
                InputRoutingMode::NativeDirect,
                viewport(),
            ),
        )
        .unwrap();
    let before_stale = coordinator.clone();
    assert!(matches!(
        coordinator.update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                "window:main",
                DesiredPresence::Absent,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
                InputRoutingMode::Disabled,
                viewport(),
            ),
        ),
        Err(CoordinationError::StaleRevision { .. })
    ));
    assert_eq!(coordinator, before_stale);

    let overflow = ClientRect::new(
        ClientPoint::new(f64::MAX, 0.0).unwrap(),
        ClientSize::new(1.0, 1.0).unwrap(),
    );
    coordinator
        .update_desired(
            NativeContentRevision::new(1),
            desired_update(
                1,
                "window:main",
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
                InputRoutingMode::NativeDirect,
                overflow,
            ),
        )
        .unwrap();
    let before_plan = coordinator.clone();
    assert_eq!(
        coordinator.plan(),
        Err(CoordinationError::ViewportConversion(
            ViewportConversionError::CoordinateOverflow
        ))
    );
    assert_eq!(coordinator, before_plan);
}

#[test]
fn invalid_observation_capabilities_leave_exact_state_unchanged() {
    let mut backing = coordinator(NativeContentMechanism::BackingSurface);
    let before = backing.clone();
    let false_visibility = ObservationUpdate::new(
        generation(1),
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        EffectiveVisibility::Visible,
        EffectiveFocus::Unknown,
        ObservedGeometry::Unknown,
        Some(InputRoutingMode::RendererForwarded),
    );
    assert_eq!(
        backing.admit_observation(NativeContentRevision::INITIAL, false_visibility),
        Err(CoordinationError::UnsupportedVisibilityObservation)
    );
    assert_eq!(backing, before);
}

#[test]
fn stale_and_future_observations_leave_state_unchanged() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                2,
                "window:main",
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
                InputRoutingMode::NativeDirect,
                viewport(),
            ),
        )
        .unwrap();
    let before = coordinator.clone();
    assert!(matches!(
        coordinator.admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::ChildView, 1)
        ),
        Err(CoordinationError::StaleGeneration { .. })
    ));
    assert_eq!(coordinator, before);
    assert!(matches!(
        coordinator.admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::ChildView, 3)
        ),
        Err(CoordinationError::FutureGeneration { .. })
    ));
    assert_eq!(coordinator, before);
}

#[test]
fn host_destroy_is_explicit_idempotent_and_blocks_late_events() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::ChildView, 1),
        )
        .unwrap();
    let first = coordinator
        .host_destroyed(
            &WindowId::new("window:main").unwrap(),
            NativeContentRevision::new(1),
        )
        .unwrap();
    assert_eq!(first.outcome(), HostDestroyOutcome::Invalidated);
    assert_eq!(
        coordinator.observed().lifecycle(),
        AttachmentLifecycle::Absent
    );
    assert_eq!(
        coordinator.plan(),
        Err(CoordinationError::InvalidatedGeneration(generation(1)))
    );

    let second = coordinator
        .host_destroyed(
            &WindowId::new("window:main").unwrap(),
            NativeContentRevision::new(2),
        )
        .unwrap();
    assert_eq!(second.outcome(), HostDestroyOutcome::AlreadyInvalidated);
    assert_eq!(
        second.current_observed_revision(),
        NativeContentRevision::new(2)
    );
    let before = coordinator.clone();
    assert_eq!(
        coordinator.admit_observation(
            NativeContentRevision::new(2),
            attached_observation(NativeContentMechanism::ChildView, 1)
        ),
        Err(CoordinationError::InvalidatedGeneration(generation(1)))
    );
    assert_eq!(coordinator, before);

    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                2,
                "window:replacement",
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Request,
                InputRoutingMode::NativeDirect,
                viewport(),
            ),
        )
        .unwrap();
    assert_eq!(coordinator.invalidated_generation(), None);
    assert!(
        matches!(coordinator.plan().unwrap().operations()[0].operation(), NativeContentOperation::Attach { host_window_id, .. } if host_window_id.as_str() == "window:replacement")
    );
}

#[test]
fn detach_is_explicit_and_not_reissued_while_detaching() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::ChildView, 1),
        )
        .unwrap();
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                "window:main",
                DesiredPresence::Absent,
                DesiredVisibility::Hidden {
                    reason: "shutdown".parse().unwrap(),
                },
                FocusIntent::ReleaseIfOwned,
                InputRoutingMode::Disabled,
                viewport(),
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    assert_eq!(plan.operations().len(), 1);
    assert!(matches!(
        plan.operations()[0].operation(),
        NativeContentOperation::Detach {
            policy: DetachPolicy::Reversible
        }
    ));

    coordinator
        .admit_observation(
            NativeContentRevision::new(1),
            ObservationUpdate::new(
                generation(1),
                AttachmentLifecycle::Detaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Hidden,
                EffectiveFocus::Unfocused,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .unwrap();
    assert!(coordinator.plan().unwrap().is_empty());
    coordinator
        .admit_observation(
            NativeContentRevision::new(2),
            ObservationUpdate::absent(generation(1)),
        )
        .unwrap();
    assert!(coordinator.plan().unwrap().is_empty());
}

#[test]
fn partial_apply_receipt_preserves_failure_and_dependency_causality() {
    let coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    let first = PlanStepId::new(1).unwrap();
    let second = PlanStepId::new(2).unwrap();
    let receipt = coordinator
        .receipt(
            &plan,
            [
                StepExecution::Applied { step: first },
                StepExecution::Failed {
                    step: second,
                    code: NativeContentFailureCode::new("native:bounds-rejected").unwrap(),
                },
            ],
        )
        .unwrap();
    assert_eq!(receipt.steps()[0].outcome(), &OperationOutcome::Applied);
    assert!(
        matches!(receipt.steps()[1].outcome(), OperationOutcome::Failed { code } if code.as_str() == "native:bounds-rejected")
    );
    for step in &receipt.steps()[2..] {
        assert!(matches!(
            step.outcome(),
            OperationOutcome::DependencySkipped { .. }
        ));
    }
}

#[test]
fn malformed_and_stale_completion_reports_fail_closed() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    let unknown = PlanStepId::new(99).unwrap();
    assert_eq!(
        coordinator.receipt(&plan, [StepExecution::Applied { step: unknown }]),
        Err(ReceiptError::UnknownStep(unknown))
    );
    let first = PlanStepId::new(1).unwrap();
    assert_eq!(
        coordinator.receipt(
            &plan,
            [
                StepExecution::Applied { step: first },
                StepExecution::Applied { step: first }
            ]
        ),
        Err(ReceiptError::DuplicateStep(first))
    );

    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                "window:main",
                DesiredPresence::Present,
                DesiredVisibility::Hidden {
                    reason: "overlay:menu".parse().unwrap(),
                },
                FocusIntent::Unchanged,
                InputRoutingMode::NativeDirect,
                viewport(),
            ),
        )
        .unwrap();
    assert!(matches!(
        coordinator.receipt(&plan, []),
        Err(ReceiptError::StaleDesiredPlan { .. })
    ));
}

#[test]
fn observation_change_and_host_invalidation_stale_existing_plans() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    let plan = coordinator.plan().unwrap();
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            attached_observation(NativeContentMechanism::ChildView, 1),
        )
        .unwrap();
    assert!(matches!(
        coordinator.receipt(&plan, []),
        Err(ReceiptError::StaleObservedPlan { .. })
    ));
    let fresh = coordinator.plan().unwrap();
    coordinator
        .host_destroyed(
            &WindowId::new("window:main").unwrap(),
            NativeContentRevision::new(1),
        )
        .unwrap();
    assert!(matches!(
        coordinator.receipt(&fresh, []),
        Err(ReceiptError::StaleObservedPlan { .. } | ReceiptError::InvalidGeneration { .. })
    ));
}

#[test]
fn content_size_proposals_are_capability_revision_generation_and_invalidation_bound() {
    let mut isolated = coordinator(NativeContentMechanism::IsolatedWindow);
    let original = isolated.clone();
    let size = ClientSize::new(801.0, 601.0).unwrap();
    let receipt = isolated
        .decide_content_size(
            ContentSizeProposal::new(generation(1), NativeContentRevision::INITIAL, size),
            ContentSizeDecision::Constrained {
                size: ClientSize::new(800.0, 600.0).unwrap(),
            },
        )
        .unwrap();
    assert_eq!(
        receipt.accepted_size(),
        Some(ClientSize::new(800.0, 600.0).unwrap())
    );
    assert_eq!(isolated, original);
    assert!(matches!(
        isolated.decide_content_size(
            ContentSizeProposal::new(generation(2), NativeContentRevision::INITIAL, size),
            ContentSizeDecision::Accepted
        ),
        Err(CoordinationError::FutureGeneration { .. })
    ));

    let child = coordinator(NativeContentMechanism::ChildView);
    assert_eq!(
        child.decide_content_size(
            ContentSizeProposal::new(generation(1), NativeContentRevision::INITIAL, size),
            ContentSizeDecision::Accepted
        ),
        Err(CoordinationError::ContentSizeRequestsUnsupported)
    );

    isolated
        .host_destroyed(
            &WindowId::new("window:main").unwrap(),
            NativeContentRevision::INITIAL,
        )
        .unwrap();
    assert_eq!(
        isolated.decide_content_size(
            ContentSizeProposal::new(generation(1), NativeContentRevision::INITIAL, size),
            ContentSizeDecision::Accepted
        ),
        Err(CoordinationError::InvalidatedGeneration(generation(1)))
    );
}

#[test]
fn public_evidence_contains_no_product_payload_or_native_handle() {
    let plan = coordinator(NativeContentMechanism::ChildView)
        .plan()
        .unwrap();
    let json = serde_json::to_string(&plan).unwrap();
    for forbidden in [
        "payload",
        "url",
        "plugin",
        "midi",
        "camera",
        "raw_handle",
        "nsview",
    ] {
        assert!(
            !json.to_ascii_lowercase().contains(forbidden),
            "found forbidden token {forbidden}"
        );
    }
}

#[test]
fn manifest_and_source_keep_framework_and_native_edges_absent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .to_ascii_lowercase();
    for forbidden in ["tauri", "wgpu", "poodle", "svelte", "vst3", "clap"] {
        assert!(
            !manifest.contains(forbidden),
            "manifest contains {forbidden}"
        );
    }
    assert!(manifest.contains("longhorn-core.workspace = true"));
    assert!(manifest.contains("serde.workspace = true"));

    let mut source = String::new();
    collect_rs(&root.join("src"), &mut source);
    let source = source.to_ascii_lowercase();
    for forbidden in [
        "tauri::",
        "wgpu::",
        "poodle",
        "svelte",
        "vst3",
        "rawwindowhandle",
        "nsview",
    ] {
        assert!(!source.contains(forbidden), "source contains {forbidden}");
    }
}

fn collect_rs(path: &Path, output: &mut String) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push_str(&fs::read_to_string(path).unwrap());
        }
    }
}
