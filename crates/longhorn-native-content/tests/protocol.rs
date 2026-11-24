//! Protocol authority, session, correlation, and projection evidence.

use longhorn_core::{
    ClientPoint, ClientRect, ClientSize, NativeContentRevision, RoundingMode, ScaleFactor, WindowId,
};
use longhorn_native_content::{
    AttachGeneration, AttachmentLifecycle, ContentSizeDecision, ContentSizeProposal,
    CoordinationError, DesiredPresence, DesiredState, DesiredUpdate, DesiredVisibility,
    DetachPolicy, EffectiveFocus, EffectiveVisibility, FocusIntent, InputRoutingMode,
    MechanismCapabilities, NativeContentAuthorityEpoch, NativeContentChangeProjection,
    NativeContentConnectRequest, NativeContentConnectResult,
    NativeContentContentSizeDecisionRequest, NativeContentContentSizeDecisionResult,
    NativeContentCoordinator, NativeContentDesiredUpdateRequest, NativeContentDesiredUpdateResult,
    NativeContentIslandId, NativeContentKindId, NativeContentMechanism, NativeContentProtocolHost,
    NativeContentProtocolRejection, NativeContentProtocolVersion, NativeContentRejectionCode,
    NativeContentRequestId, NativeContentRetryClass, NativeContentSnapshotRequest,
    NativeContentSnapshotResult, ObservationUpdate, ObservedGeometry, ObservedReadiness,
    PlanStepId, StepExecution,
};

fn request_id(value: &str) -> NativeContentRequestId {
    NativeContentRequestId::new(value).unwrap()
}

fn generation(value: u64) -> AttachGeneration {
    AttachGeneration::new(value).unwrap()
}

fn viewport(x: f64) -> ClientRect {
    ClientRect::new(
        ClientPoint::new(x, 20.0).unwrap(),
        ClientSize::new(320.0, 180.0).unwrap(),
    )
}

fn desired_update(generation: u64, x: f64) -> DesiredUpdate {
    DesiredUpdate::new(
        self::generation(generation),
        WindowId::new("window:main").unwrap(),
        viewport(x),
        ScaleFactor::from_thousandths(2000).unwrap(),
        RoundingMode::Nearest,
        DesiredPresence::Present,
        DesiredVisibility::Visible,
        FocusIntent::Request,
        InputRoutingMode::NativeDirect,
    )
}

fn host() -> NativeContentProtocolHost {
    let desired = DesiredState::new(
        NativeContentIslandId::new("island:fixture").unwrap(),
        NativeContentKindId::new("fixture:child").unwrap(),
        MechanismCapabilities::new(
            NativeContentMechanism::ChildView,
            InputRoutingMode::NativeDirect,
            false,
            DetachPolicy::Reversible,
            true,
            true,
        ),
        desired_update(1, 10.0),
    )
    .unwrap();
    NativeContentProtocolHost::new(
        NativeContentAuthorityEpoch::new(7).unwrap(),
        NativeContentCoordinator::new(desired),
    )
}

fn connect(
    host: &mut NativeContentProtocolHost,
    id: &str,
) -> longhorn_native_content::NativeContentSnapshot {
    match host.connect(NativeContentConnectRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id(id),
        island_id: NativeContentIslandId::new("island:fixture").unwrap(),
    }) {
        NativeContentConnectResult::Connected {
            request_id: echoed,
            snapshot,
        } => {
            assert_eq!(echoed.as_str(), id);
            *snapshot
        }
        NativeContentConnectResult::Rejected { rejection, .. } => {
            panic!("connect rejected: {rejection:?}")
        }
    }
}

#[test]
fn renderer_epoch_advances_without_changing_attach_generation() {
    let mut host = host();
    let first = connect(&mut host, "request:connect-1");
    let second = connect(&mut host, "request:connect-2");

    assert_eq!(first.cursor.authority_epoch.get(), 7);
    assert_eq!(second.cursor.authority_epoch.get(), 7);
    assert_eq!(first.cursor.client_epoch.get(), 1);
    assert_eq!(second.cursor.client_epoch.get(), 2);
    assert_eq!(first.cursor.attach_generation.get(), 1);
    assert_eq!(second.cursor.attach_generation.get(), 1);

    let stale = host.snapshot(NativeContentSnapshotRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:stale-snapshot"),
        island_id: NativeContentIslandId::new("island:fixture").unwrap(),
        authority_epoch: first.cursor.authority_epoch,
        client_epoch: first.cursor.client_epoch,
    });
    assert!(matches!(
        stale,
        NativeContentSnapshotResult::Rejected { rejection, .. }
            if rejection.code == NativeContentRejectionCode::StaleClientEpoch
    ));
}

#[test]
fn stale_renderer_mutation_is_rejected_without_state_change() {
    let mut host = host();
    let old = connect(&mut host, "request:old");
    let current = connect(&mut host, "request:current");
    let before = host.coordinator().clone();

    let result = host.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:update-old"),
        island_id: NativeContentIslandId::new("island:fixture").unwrap(),
        authority_epoch: old.cursor.authority_epoch,
        client_epoch: old.cursor.client_epoch,
        expected_desired_revision: NativeContentRevision::INITIAL,
        update: desired_update(1, 40.0),
    });
    assert!(matches!(
        result,
        NativeContentDesiredUpdateResult::Rejected { rejection, .. }
            if rejection.code == NativeContentRejectionCode::StaleClientEpoch
    ));
    assert_eq!(host.coordinator(), &before);

    let committed = host.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:update-current"),
        island_id: NativeContentIslandId::new("island:fixture").unwrap(),
        authority_epoch: current.cursor.authority_epoch,
        client_epoch: current.cursor.client_epoch,
        expected_desired_revision: NativeContentRevision::INITIAL,
        update: desired_update(1, 40.0),
    });
    assert!(matches!(
        committed,
        NativeContentDesiredUpdateResult::Committed {
            request_id,
            event,
            ..
        } if request_id.as_str() == "request:update-current"
            && matches!(&event.change, NativeContentChangeProjection::DesiredUpdated { request_id, .. } if request_id.as_str() == "request:update-current")
    ));
}

#[test]
fn replacement_authority_fences_old_requests_even_when_session_and_revision_match() {
    let mut original = host();
    let old = connect(&mut original, "request:original").cursor;
    let mut replacement = NativeContentProtocolHost::new(
        old.authority_epoch.checked_next().unwrap(),
        original.coordinator().clone(),
    );
    let current = connect(&mut replacement, "request:replacement").cursor;
    assert_eq!(old.client_epoch, current.client_epoch);
    assert_eq!(old.desired_revision, current.desired_revision);

    let snapshot = replacement.snapshot(NativeContentSnapshotRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:old-snapshot"),
        island_id: old.island_id.clone(),
        authority_epoch: old.authority_epoch,
        client_epoch: old.client_epoch,
    });
    let update = replacement.update_desired(NativeContentDesiredUpdateRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:old-update"),
        island_id: old.island_id.clone(),
        authority_epoch: old.authority_epoch,
        client_epoch: old.client_epoch,
        expected_desired_revision: old.desired_revision,
        update: desired_update(1, 42.0),
    });
    let decision = replacement.decide_content_size(NativeContentContentSizeDecisionRequest {
        protocol_version: NativeContentProtocolVersion::CURRENT,
        request_id: request_id("request:old-decision"),
        island_id: old.island_id,
        authority_epoch: old.authority_epoch,
        client_epoch: old.client_epoch,
        proposal: ContentSizeProposal::new(
            old.attach_generation,
            old.desired_revision,
            viewport(0.0).size(),
        ),
        decision: ContentSizeDecision::Accepted,
    });
    let (
        NativeContentSnapshotResult::Rejected {
            rejection: snapshot,
            ..
        },
        NativeContentDesiredUpdateResult::Rejected {
            rejection: update, ..
        },
        NativeContentContentSizeDecisionResult::Rejected {
            rejection: decision,
            ..
        },
    ) = (snapshot, update, decision)
    else {
        panic!("replacement accepted an old authority request");
    };
    for rejection in [snapshot, update, decision] {
        assert_eq!(
            rejection.code,
            NativeContentRejectionCode::AuthorityEpochMismatch
        );
        assert_eq!(rejection.retry, NativeContentRetryClass::Refresh);
    }
    assert_eq!(replacement.coordinator(), original.coordinator());
}

#[test]
fn retry_class_distinguishes_stale_evidence_from_permanent_coordination_errors() {
    for error in [
        CoordinationError::RevisionOverflow,
        CoordinationError::GenerationOverflow,
        CoordinationError::UnsupportedInputRouting {
            supported: InputRoutingMode::NativeDirect,
            supplied: InputRoutingMode::RendererForwarded,
        },
        CoordinationError::ContentSizeRequestsUnsupported,
    ] {
        assert_eq!(
            NativeContentProtocolRejection::from(error).retry,
            NativeContentRetryClass::Never
        );
    }
    assert_eq!(
        NativeContentProtocolRejection::from(CoordinationError::StaleRevision {
            current: NativeContentRevision::new(1),
            supplied: NativeContentRevision::INITIAL,
        })
        .retry,
        NativeContentRetryClass::Refresh
    );
}

#[test]
fn trusted_observation_proposal_and_apply_receipt_use_current_cursor() {
    let mut host = host();
    let connected = connect(&mut host, "request:connect");
    let plan = host.coordinator().plan().unwrap();
    let (_, apply_event) = host
        .complete_apply(
            request_id("request:apply"),
            &plan,
            [StepExecution::Applied {
                step: PlanStepId::new(1).unwrap(),
            }],
        )
        .unwrap();
    let apply_event = apply_event.unwrap();
    assert_eq!(
        apply_event.cursor.client_epoch,
        connected.cursor.client_epoch
    );
    assert!(matches!(
        apply_event.change,
        NativeContentChangeProjection::ApplyCompleted { request_id, .. }
            if request_id.as_str() == "request:apply"
    ));

    let (_, observation_event) = host
        .admit_observation(
            Some(request_id("request:observe")),
            NativeContentRevision::INITIAL,
            ObservationUpdate::new(
                generation(1),
                AttachmentLifecycle::Attaching,
                ObservedReadiness::NotReady,
                EffectiveVisibility::Unknown,
                EffectiveFocus::Unknown,
                ObservedGeometry::Unknown,
                None,
            ),
        )
        .unwrap();
    let observation_event = observation_event.unwrap();
    assert_eq!(observation_event.cursor.observed_revision.get(), 1);
    assert!(matches!(
        observation_event.change,
        NativeContentChangeProjection::ObservationAdmitted { .. }
    ));

    let unsupported = host.publish_content_size_proposal(
        request_id("request:proposal"),
        ContentSizeProposal::new(
            generation(1),
            NativeContentRevision::new(1),
            ClientSize::new(800.0, 600.0).unwrap(),
        ),
    );
    assert!(unsupported.is_err());
}

#[test]
fn protocol_serialization_contains_no_product_or_native_payload() {
    let mut host = host();
    let snapshot = connect(&mut host, "request:connect");
    let json = serde_json::to_string(&snapshot)
        .unwrap()
        .to_ascii_lowercase();
    for forbidden in [
        "url",
        "navigation",
        "plugin",
        "midi",
        "camera",
        "gpu",
        "raw_handle",
        "tauri_label",
    ] {
        assert!(
            !json.contains(forbidden),
            "found forbidden token {forbidden}"
        );
    }
}
