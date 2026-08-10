use std::error::Error;

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, NativeContentIslandId, NativeContentKindId,
    NativeContentRevision, RoundingMode, ScaleFactor, WindowId,
};
use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, ContentSizeDecision, ContentSizeProposal,
    DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility, DetachPolicy, EffectiveFocus,
    EffectiveVisibility, FocusIntent, InputRoutingMode, MechanismCapabilities,
    NativeContentAuthorityEpoch, NativeContentConnectRequest, NativeContentConnectResult,
    NativeContentContentSizeDecisionRequest, NativeContentCoordinator,
    NativeContentDesiredUpdateRequest, NativeContentMechanism, NativeContentProtocolHost,
    NativeContentProtocolVersion, NativeContentSnapshotRequest, ObservationUpdate,
    ObservedGeometry, ObservedReadiness, PlanStepId, StepExecution,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let mut host = host()?;
    let connect = host.connect(NativeContentConnectRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:connect"),
        island_id: id("island:fixture"),
    });
    let client_epoch = match &connect {
        NativeContentConnectResult::Connected { snapshot, .. } => snapshot.cursor.client_epoch,
        NativeContentConnectResult::Rejected { rejection, .. } => {
            return Err(format!("fixture connect rejected: {rejection:?}").into());
        }
    };
    let snapshot = host.snapshot(NativeContentSnapshotRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:snapshot"),
        island_id: id("island:fixture"),
        client_epoch,
    });
    let update = host.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:update"),
        island_id: id("island:fixture"),
        client_epoch,
        expected_desired_revision: NativeContentRevision::INITIAL,
        update: desired_update(1, 24.0)?,
    });
    let plan = host.coordinator().plan()?;
    let (_, apply_event) = host.complete_apply(
        id("request:apply"),
        &plan,
        [StepExecution::Applied {
            step: PlanStepId::new(1)?,
        }],
    )?;
    let (_, observation_event) = host.admit_observation(
        Some(id("request:observe")),
        NativeContentRevision::INITIAL,
        ObservationUpdate::new(
            AttachGeneration::INITIAL,
            AttachmentLifecycle::Attaching,
            ObservedReadiness::NotReady,
            EffectiveVisibility::Unknown,
            EffectiveFocus::Unknown,
            ObservedGeometry::Unknown,
            None,
        ),
    )?;
    let proposal = ContentSizeProposal::new(
        AttachGeneration::INITIAL,
        NativeContentRevision::new(1),
        ClientSize::new(800.0, 600.0)?,
    );
    let proposal_event = host
        .publish_content_size_proposal(id("request:proposal"), proposal)?
        .ok_or("connected fixture host must project proposal")?;
    let decision = host.decide_content_size(NativeContentContentSizeDecisionRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:decision"),
        island_id: id("island:fixture"),
        client_epoch,
        proposal,
        decision: ContentSizeDecision::Constrained {
            size: ClientSize::new(768.0, 576.0)?,
        },
    });
    let stale_revision = host.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:stale-revision"),
        island_id: id("island:fixture"),
        client_epoch,
        expected_desired_revision: NativeContentRevision::INITIAL,
        update: desired_update(1, 48.0)?,
    });
    let remount = host.connect(NativeContentConnectRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:remount"),
        island_id: id("island:fixture"),
    });
    let stale_session = host.snapshot(NativeContentSnapshotRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: id("request:stale-session"),
        island_id: id("island:fixture"),
        client_epoch,
    });
    let future_version: NativeContentProtocolVersion = serde_json::from_value(json!(2))?;
    let incompatible = host.connect(NativeContentConnectRequest {
        protocol_version: future_version,
        request_id: id("request:future-protocol"),
        island_id: id("island:fixture"),
    });

    // No fixture category exercised the destroy receipt, so its validator was
    // proved by nothing. A standalone coordinator is enough — the protocol host
    // does not expose the call.
    let mut destroying = NativeContentCoordinator::new(desired_state()?);
    let host_destroy = destroying.host_destroyed(
        &WindowId::new("window:main")?,
        NativeContentRevision::INITIAL,
    )?;

    let fixture = json!({
        "protocolVersion": 1,
        "connect": to_value(connect)?,
        "snapshot": to_value(snapshot)?,
        "desiredUpdate": to_value(update)?,
        "applyEvent": to_value(apply_event)?,
        "observationEvent": to_value(observation_event)?,
        "proposalEvent": to_value(proposal_event)?,
        "decision": to_value(decision)?,
        "staleRevision": to_value(stale_revision)?,
        "remount": to_value(remount)?,
        "staleSession": to_value(stale_session)?,
        "incompatible": to_value(incompatible)?,
        "hostDestroy": to_value(host_destroy)?,
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownMechanism": "browser_plugin",
            "unknownChangeKind": "product_payload",
            "unknownUpdateStatus": "uncertain",
            "unknownRejectionCode": "future_rejection"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

fn host() -> Result<NativeContentProtocolHost, Box<dyn Error>> {
    Ok(NativeContentProtocolHost::new(
        NativeContentAuthorityEpoch::new(9)?,
        NativeContentCoordinator::new(desired_state()?),
    ))
}

fn desired_state() -> Result<DesiredState, Box<dyn Error>> {
    let desired = DesiredState::new(
        id::<NativeContentIslandId>("island:fixture"),
        id::<NativeContentKindId>("fixture:isolated"),
        MechanismCapabilities::new(
            NativeContentMechanism::IsolatedWindow,
            InputRoutingMode::NativeDirect,
            true,
            DetachPolicy::OwnerProcessTermination,
            true,
            true,
        ),
        desired_update(1, 12.0)?,
    )?;
    Ok(desired)
}

fn desired_update(generation: u64, x: f64) -> Result<DesiredUpdate, Box<dyn Error>> {
    Ok(DesiredUpdate::new(
        AttachGeneration::new(generation)?,
        WindowId::new("window:main")?,
        ClientRect::new(ClientPoint::new(x, 16.0)?, ClientSize::new(640.0, 360.0)?),
        ScaleFactor::from_thousandths(2000)?,
        RoundingMode::Nearest,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
    ))
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value
        .parse()
        .expect("native-content fixture id must be valid")
}
