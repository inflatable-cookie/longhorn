use longhorn_bridge::{
    BridgeServiceAction, BridgeServiceMachine, BridgeServiceOutcome, BridgeServiceOwnership,
    BridgeServiceRequest, BridgeServiceState, BridgeServiceSupervisor, BridgeSupervisionError,
};
use longhorn_core::BridgeCredentialRef;

struct Supervisor {
    outcomes: Vec<BridgeServiceOutcome>,
}

impl BridgeServiceSupervisor for Supervisor {
    fn perform(&mut self, request: &BridgeServiceRequest) -> BridgeServiceOutcome {
        assert!(
            request.credential_ref().is_none()
                || request.credential_ref().unwrap().as_str() == "credential:workspace"
        );
        self.outcomes.remove(0)
    }
}

#[test]
fn owned_local_spawn_readiness_restart_and_shutdown_are_receipted() {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
    let mut supervisor = Supervisor {
        outcomes: vec![
            BridgeServiceOutcome::Accepted,
            BridgeServiceOutcome::NotReady,
            BridgeServiceOutcome::Ready,
            BridgeServiceOutcome::Accepted,
            BridgeServiceOutcome::Stopped,
        ],
    };

    let spawn = machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(
                BridgeServiceAction::Spawn,
                Some(BridgeCredentialRef::new("credential:workspace").unwrap()),
            ),
        )
        .unwrap();
    assert_eq!(spawn.current(), BridgeServiceState::Starting);
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::AwaitingReadiness
    );
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::Ready
    );
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Restart, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::Restarting
    );
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Shutdown, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::Stopped
    );
}

#[test]
fn external_remote_attach_can_reconnect_but_cannot_stop_or_replace_host() {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::ExternalRemote);
    let mut supervisor = Supervisor {
        outcomes: vec![
            BridgeServiceOutcome::Accepted,
            BridgeServiceOutcome::Ready,
            BridgeServiceOutcome::Accepted,
        ],
    };
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::Attach, None),
        )
        .unwrap();
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
        )
        .unwrap();

    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Restart, None),
            )
            .unwrap_err(),
        BridgeSupervisionError::LifecycleNotOwned
    );
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Shutdown, None),
            )
            .unwrap_err(),
        BridgeSupervisionError::LifecycleNotOwned
    );
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Reconnect, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::Reconnecting
    );
}

#[test]
fn external_local_attach_and_failure_are_observable() {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::ExternalLocal);
    let mut supervisor = Supervisor {
        outcomes: vec![
            BridgeServiceOutcome::Accepted,
            BridgeServiceOutcome::Failed(
                longhorn_bridge::BridgeServiceFailureCode::ReadinessFailed,
            ),
        ],
    };
    assert_eq!(
        machine
            .execute(
                &mut supervisor,
                BridgeServiceRequest::new(BridgeServiceAction::Attach, None),
            )
            .unwrap()
            .current(),
        BridgeServiceState::Attaching
    );
    let failure = machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
        )
        .unwrap();
    assert_eq!(failure.current(), BridgeServiceState::Failed);
    assert_eq!(
        failure.outcome(),
        BridgeServiceOutcome::Failed(longhorn_bridge::BridgeServiceFailureCode::ReadinessFailed,)
    );
}

#[test]
fn credential_api_serializes_only_an_opaque_reference() {
    let request = BridgeServiceRequest::new(
        BridgeServiceAction::Attach,
        Some(BridgeCredentialRef::new("credential:workspace").unwrap()),
    );
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "action": "attach",
            "credentialRef": "credential:workspace"
        })
    );
}
