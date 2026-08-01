use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode,
    ScaleFactor, WindowId,
};
use longhorn_native_content_prototype::{
    AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent,
    InputRoutingMode, MechanismCapabilities, NativeContentCoordinator, NativeContentIslandId,
    NativeContentKindId, NativeContentMechanism, ObservationUpdate, ObservedGeometry,
    ObservedReadiness,
};

pub fn viewport() -> ClientRect {
    ClientRect::new(
        ClientPoint::new(10.25, 20.5).unwrap(),
        ClientSize::new(320.0, 180.0).unwrap(),
    )
}

pub fn physical_viewport(scale: u32) -> PhysicalRect {
    match scale {
        1000 => PhysicalRect::new(PhysicalPoint::new(10, 21), PhysicalSize::new(320, 180)),
        2000 => PhysicalRect::new(PhysicalPoint::new(21, 41), PhysicalSize::new(640, 360)),
        _ => panic!("unsupported test scale"),
    }
}

pub fn capabilities(mechanism: NativeContentMechanism) -> MechanismCapabilities {
    match mechanism {
        NativeContentMechanism::ChildView => {
            MechanismCapabilities::new(mechanism, false, DetachPolicy::Reversible, true, true)
        }
        NativeContentMechanism::IsolatedWindow => MechanismCapabilities::new(
            mechanism,
            true,
            DetachPolicy::OwnerProcessTermination,
            true,
            true,
        ),
        NativeContentMechanism::BackingSurface => MechanismCapabilities::new(
            mechanism,
            false,
            DetachPolicy::ProcessLifetime,
            false,
            false,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn desired_update(
    generation: u64,
    presence: DesiredPresence,
    visibility: DesiredVisibility,
    focus: FocusIntent,
    input: InputRoutingMode,
    scale: u32,
    viewport: ClientRect,
) -> DesiredUpdate {
    DesiredUpdate::new(
        AttachGeneration::new(generation),
        WindowId::new("window:main").unwrap(),
        viewport,
        ScaleFactor::from_thousandths(scale).unwrap(),
        RoundingMode::Nearest,
        presence,
        visibility,
        focus,
        input,
    )
}

pub fn coordinator(mechanism: NativeContentMechanism) -> NativeContentCoordinator {
    let input = match mechanism {
        NativeContentMechanism::ChildView | NativeContentMechanism::IsolatedWindow => {
            InputRoutingMode::NativeDirect
        }
        NativeContentMechanism::BackingSurface => InputRoutingMode::RendererForwarded,
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
            DesiredPresence::Present,
            DesiredVisibility::Visible,
            focus,
            input,
            2000,
            viewport(),
        ),
    );
    NativeContentCoordinator::new(desired)
}

pub fn attached_observation(
    mechanism: NativeContentMechanism,
    generation: u64,
    input: InputRoutingMode,
) -> ObservationUpdate {
    let geometry = match mechanism {
        NativeContentMechanism::ChildView => ObservedGeometry::ChildBounds {
            bounds: physical_viewport(2000),
        },
        NativeContentMechanism::IsolatedWindow => ObservedGeometry::IsolatedContent {
            size: physical_viewport(2000).size(),
        },
        NativeContentMechanism::BackingSurface => ObservedGeometry::BackingSurface {
            storage_bounds: PhysicalRect::new(
                PhysicalPoint::new(0, 0),
                PhysicalSize::new(1600, 1000),
            ),
            clip: physical_viewport(2000),
        },
    };
    let observable = mechanism != NativeContentMechanism::BackingSurface;
    ObservationUpdate::new(
        AttachGeneration::new(generation),
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
        Some(input),
    )
}
