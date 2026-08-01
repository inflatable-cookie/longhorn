use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode,
    ScaleFactor,
};
use longhorn_native_content_prototype::{
    DesiredPresence, DesiredVisibility, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, NativeContentMechanism, NativeContentOperation, NativeContentRevision,
    ObservationUpdate, ObservedGeometry, ObservedReadiness, ViewportConversionError,
    viewport_to_physical,
};

use super::support::{coordinator, desired_update, viewport};

#[test]
fn client_viewport_converts_at_one_and_two_x_with_explicit_rounding() {
    let input = ClientRect::new(
        ClientPoint::new(-10.5, 2.25).unwrap(),
        ClientSize::new(100.5, 50.25).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            input,
            ScaleFactor::from_thousandths(1000).unwrap(),
            RoundingMode::Nearest,
        )
        .unwrap(),
        PhysicalRect::new(PhysicalPoint::new(-11, 2), PhysicalSize::new(101, 50))
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
}

#[test]
fn rounding_zero_and_overflow_are_explicit() {
    let fractional = ClientRect::new(
        ClientPoint::new(1.25, -1.25).unwrap(),
        ClientSize::new(2.25, 0.0).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            fractional,
            ScaleFactor::from_thousandths(1000).unwrap(),
            RoundingMode::Floor,
        )
        .unwrap(),
        PhysicalRect::new(PhysicalPoint::new(1, -2), PhysicalSize::new(2, 0))
    );
    assert_eq!(
        viewport_to_physical(
            fractional,
            ScaleFactor::from_thousandths(1000).unwrap(),
            RoundingMode::Ceil,
        )
        .unwrap(),
        PhysicalRect::new(PhysicalPoint::new(2, -1), PhysicalSize::new(3, 0))
    );

    let coordinate_overflow = ClientRect::new(
        ClientPoint::new(f64::MAX, 0.0).unwrap(),
        ClientSize::new(1.0, 1.0).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            coordinate_overflow,
            ScaleFactor::from_thousandths(2000).unwrap(),
            RoundingMode::Nearest,
        ),
        Err(ViewportConversionError::CoordinateOverflow)
    );
    let extent_overflow = ClientRect::new(
        ClientPoint::new(0.0, 0.0).unwrap(),
        ClientSize::new(f64::MAX, 1.0).unwrap(),
    );
    assert_eq!(
        viewport_to_physical(
            extent_overflow,
            ScaleFactor::from_thousandths(2000).unwrap(),
            RoundingMode::Nearest,
        ),
        Err(ViewportConversionError::ExtentOverflow)
    );
}

#[test]
fn backing_storage_size_does_not_drive_clip_convergence() {
    let mut coordinator = coordinator(NativeContentMechanism::BackingSurface);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            ObservationUpdate::new(
                coordinator.desired().generation(),
                longhorn_native_content_prototype::AttachmentLifecycle::Attached,
                ObservedReadiness::Ready,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::BackingSurface {
                    storage_bounds: PhysicalRect::new(
                        PhysicalPoint::new(0, 0),
                        PhysicalSize::new(4096, 2160),
                    ),
                    clip: super::support::physical_viewport(2000),
                },
                Some(InputRoutingMode::RendererForwarded),
            ),
        )
        .unwrap();
    let initial = coordinator.plan().unwrap();
    assert!(!initial.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetBackingViewport { .. }
    )));

    let moved = ClientRect::new(
        ClientPoint::new(30.0, 40.0).unwrap(),
        ClientSize::new(320.0, 180.0).unwrap(),
    );
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                DesiredPresence::Present,
                DesiredVisibility::Visible,
                FocusIntent::Unchanged,
                InputRoutingMode::RendererForwarded,
                2000,
                moved,
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    assert!(plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetBackingViewport { clip }
            if *clip == PhysicalRect::new(
                PhysicalPoint::new(60, 80),
                PhysicalSize::new(640, 360)
            )
    )));
}

#[test]
fn unknown_visibility_and_focus_remain_operations_not_inferences() {
    let mut coordinator = coordinator(NativeContentMechanism::ChildView);
    coordinator
        .admit_observation(
            NativeContentRevision::INITIAL,
            ObservationUpdate::new(
                coordinator.desired().generation(),
                longhorn_native_content_prototype::AttachmentLifecycle::Attached,
                ObservedReadiness::Ready,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::ChildBounds {
                    bounds: super::support::physical_viewport(2000),
                },
                Some(InputRoutingMode::NativeDirect),
            ),
        )
        .unwrap();
    let plan = coordinator.plan().unwrap();
    assert!(
        plan.operations()
            .iter()
            .any(|step| matches!(step.operation(), NativeContentOperation::Show))
    );
    assert!(
        plan.operations()
            .iter()
            .any(|step| matches!(step.operation(), NativeContentOperation::RequestFocus))
    );

    let zero = ClientRect::new(
        ClientPoint::new(0.0, 0.0).unwrap(),
        ClientSize::new(0.0, 0.0).unwrap(),
    );
    coordinator
        .update_desired(
            NativeContentRevision::INITIAL,
            desired_update(
                1,
                DesiredPresence::Present,
                DesiredVisibility::Hidden {
                    reason: "layout:collapsed".parse().unwrap(),
                },
                FocusIntent::ReleaseIfOwned,
                InputRoutingMode::Disabled,
                2000,
                zero,
            ),
        )
        .unwrap();
    let zero_plan = coordinator.plan().unwrap();
    assert!(zero_plan.operations().iter().any(|step| matches!(
        step.operation(),
        NativeContentOperation::SetChildBounds { bounds } if bounds.size().is_empty()
    )));
    assert!(
        zero_plan
            .operations()
            .iter()
            .any(|step| matches!(step.operation(), NativeContentOperation::Hide { .. }))
    );
}

#[test]
fn fixture_viewport_remains_finite() {
    assert_eq!(viewport().size(), ClientSize::new(320.0, 180.0).unwrap());
}
