use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, NativeContentRequestId, NativeContentRevision,
    PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode, ScaleFactor, VisibilityReasonId,
    WindowId,
};
use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, DesiredPresence, DesiredState, DesiredUpdate,
    DesiredVisibility, EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode,
    MechanismCapabilities, NativeContentAuthorityEpoch, NativeContentConnectRequest,
    NativeContentConnectResult, NativeContentDesiredUpdateRequest,
    NativeContentDesiredUpdateResult, NativeContentIslandId, NativeContentKindId,
    NativeContentMechanism, NativeContentProtocolHost, NativeContentProtocolVersion,
    NativeContentSnapshot, NativeContentSnapshotRequest, NativeContentSnapshotResult,
    ObservationUpdate, ObservedGeometry, ObservedReadiness,
};
use serde_json::{Value, json};

pub fn artifact_trace(capabilities: MechanismCapabilities) -> Value {
    let mechanism = capabilities.mechanism();
    let input = capabilities.active_input_routing();
    let focus = if mechanism == NativeContentMechanism::BackingSurface {
        FocusIntent::Unchanged
    } else {
        FocusIntent::Request
    };
    let initial = DesiredUpdate::new(
        AttachGeneration::INITIAL,
        WindowId::new("window:main").unwrap(),
        viewport(10.25),
        ScaleFactor::from_thousandths(2000).unwrap(),
        RoundingMode::Nearest,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        focus,
        input,
    );
    let desired = DesiredState::new(
        NativeContentIslandId::new("island:artifact-proof").unwrap(),
        NativeContentKindId::new("fixture:native-content").unwrap(),
        capabilities,
        initial,
    )
    .unwrap();
    let mut host = NativeContentProtocolHost::new(
        NativeContentAuthorityEpoch::new(1).unwrap(),
        longhorn_native_content::NativeContentCoordinator::new(desired),
    );

    let connected = host.connect(NativeContentConnectRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: NativeContentRequestId::new("request:connect").unwrap(),
        island_id: NativeContentIslandId::new("island:artifact-proof").unwrap(),
    });
    let NativeContentConnectResult::Connected {
        snapshot: connected,
        ..
    } = connected
    else {
        panic!("artifact fixture failed to connect")
    };
    let client_epoch = host.client_epoch().unwrap();

    host.admit_observation(
        Some(NativeContentRequestId::new("request:observe").unwrap()),
        NativeContentRevision::INITIAL,
        attached_observation(mechanism, input, capabilities),
    )
    .unwrap();
    let observed = current_snapshot(&host, client_epoch, "request:observed");

    let hidden = host.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: NativeContentRequestId::new("request:hidden").unwrap(),
        island_id: NativeContentIslandId::new("island:artifact-proof").unwrap(),
        client_epoch,
        expected_desired_revision: NativeContentRevision::INITIAL,
        update: DesiredUpdate::new(
            AttachGeneration::INITIAL,
            WindowId::new("window:main").unwrap(),
            viewport(24.5),
            ScaleFactor::from_thousandths(1500).unwrap(),
            RoundingMode::Nearest,
            DesiredPresence::Present,
            DesiredVisibility::Hidden {
                reason: VisibilityReasonId::new("consumer_overlay").unwrap(),
            },
            focus,
            input,
        ),
    });
    let NativeContentDesiredUpdateResult::Committed {
        snapshot: hidden, ..
    } = hidden
    else {
        panic!("artifact fixture desired update was rejected")
    };

    let snapshots = vec![*connected, observed, *hidden];
    let public_trace = snapshots.iter().map(project).collect::<Vec<_>>();
    json!({
        "rendererFixture": { "snapshots": snapshots },
        "publicTrace": public_trace,
    })
}

fn viewport(x: f64) -> ClientRect {
    ClientRect::new(
        ClientPoint::new(x, 20.5).unwrap(),
        ClientSize::new(320.0, 180.0).unwrap(),
    )
}

fn attached_observation(
    mechanism: NativeContentMechanism,
    input: InputRoutingMode,
    capabilities: MechanismCapabilities,
) -> ObservationUpdate {
    let bounds = PhysicalRect::new(PhysicalPoint::new(21, 41), PhysicalSize::new(640, 360));
    let geometry = match mechanism {
        NativeContentMechanism::ChildView => ObservedGeometry::ChildBounds { bounds },
        NativeContentMechanism::IsolatedWindow => ObservedGeometry::IsolatedContent {
            size: bounds.size(),
        },
        NativeContentMechanism::BackingSurface => ObservedGeometry::BackingSurface {
            storage_bounds: PhysicalRect::new(
                PhysicalPoint::new(0, 0),
                PhysicalSize::new(1920, 1080),
            ),
            clip: bounds,
        },
    };
    ObservationUpdate::new(
        AttachGeneration::INITIAL,
        AttachmentLifecycle::Attached,
        ObservedReadiness::Ready,
        if capabilities.observes_visibility() {
            EffectiveVisibility::Visible
        } else {
            EffectiveVisibility::Unknown
        },
        if capabilities.observes_focus() {
            EffectiveFocus::Focused
        } else {
            EffectiveFocus::Unknown
        },
        geometry,
        Some(input),
    )
}

fn current_snapshot(
    host: &NativeContentProtocolHost,
    client_epoch: longhorn_native_content::NativeContentClientEpoch,
    request_id: &str,
) -> NativeContentSnapshot {
    let snapshot = host.snapshot(NativeContentSnapshotRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: NativeContentRequestId::new(request_id).unwrap(),
        island_id: NativeContentIslandId::new("island:artifact-proof").unwrap(),
        client_epoch,
    });
    let NativeContentSnapshotResult::Ready { snapshot, .. } = snapshot else {
        panic!("artifact fixture snapshot was rejected")
    };
    *snapshot
}

fn project(snapshot: &NativeContentSnapshot) -> Value {
    let value = serde_json::to_value(snapshot).unwrap();
    json!({
        "cursor": {
            "generation": value["cursor"]["attach_generation"],
            "desiredRevision": value["cursor"]["desired_revision"],
            "observedRevision": value["cursor"]["observed_revision"],
        },
        "mechanism": value["desired"]["capabilities"]["mechanism"],
        "desired": {
            "viewport": value["desired"]["viewport"],
            "scale": value["desired"]["scale"],
            "visibility": value["desired"]["visibility"],
            "focus": value["desired"]["focus"],
            "inputRouting": value["desired"]["input_routing"],
        },
        "observed": {
            "lifecycle": value["observed"]["lifecycle"],
            "readiness": value["observed"]["readiness"],
            "visibility": value["observed"]["visibility"],
            "focus": value["observed"]["focus"],
            "geometry": value["observed"]["geometry"]["kind"],
            "inputRouting": value["observed"]["input_routing"],
        },
    })
}
