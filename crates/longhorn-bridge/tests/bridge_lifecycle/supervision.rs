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

/// A machine already running its first incarnation.
fn ready_machine() -> BridgeServiceMachine {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
    let mut supervisor = Supervisor {
        outcomes: vec![BridgeServiceOutcome::Accepted, BridgeServiceOutcome::Ready],
    };
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::Spawn, None),
        )
        .unwrap();
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
        )
        .unwrap();
    assert_eq!(machine.state(), BridgeServiceState::Ready);
    machine
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
fn delayed_readiness_cannot_report_ready_for_a_superseded_service() {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
    let mut supervisor = Supervisor {
        outcomes: vec![BridgeServiceOutcome::Accepted, BridgeServiceOutcome::Ready],
    };
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::Spawn, None),
        )
        .unwrap();
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
        )
        .unwrap();
    assert_eq!(machine.state(), BridgeServiceState::Ready);

    // A readiness probe is admitted against the ready service.
    let probe = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();

    // The consumer restarts the owned service before the probe reports.
    assert_eq!(
        machine
            .observe(BridgeServiceAction::Restart, BridgeServiceOutcome::Accepted)
            .unwrap()
            .current(),
        BridgeServiceState::Restarting
    );
    let after_restart = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();

    // The delayed probe cannot claim readiness for the earlier incarnation.
    assert_eq!(
        machine
            .commit(probe, BridgeServiceOutcome::Ready)
            .unwrap_err(),
        BridgeSupervisionError::StaleObservation {
            admitted: probe.incarnation(),
            current: after_restart.incarnation(),
        }
    );
    assert_eq!(machine.state(), BridgeServiceState::Restarting);

    // A probe admitted after the restart still reports readiness.
    assert_eq!(
        machine
            .commit(after_restart, BridgeServiceOutcome::Ready)
            .unwrap()
            .current(),
        BridgeServiceState::Ready
    );
}

#[test]
fn delayed_shutdown_cannot_stop_a_service_restarted_after_admission() {
    let mut machine = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
    let mut supervisor = Supervisor {
        outcomes: vec![BridgeServiceOutcome::Accepted, BridgeServiceOutcome::Ready],
    };
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::Spawn, None),
        )
        .unwrap();
    machine
        .execute(
            &mut supervisor,
            BridgeServiceRequest::new(BridgeServiceAction::CheckReadiness, None),
        )
        .unwrap();

    let shutdown = machine.begin(BridgeServiceAction::Shutdown).unwrap();
    machine
        .observe(BridgeServiceAction::Restart, BridgeServiceOutcome::Accepted)
        .unwrap();

    assert!(matches!(
        machine.commit(shutdown, BridgeServiceOutcome::Stopped),
        Err(BridgeSupervisionError::StaleObservation { .. })
    ));
    assert_eq!(machine.state(), BridgeServiceState::Restarting);
}

#[test]
fn overlapping_readiness_probes_of_one_incarnation_both_commit() {
    let mut machine = ready_machine();
    let first = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();
    let second = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();
    assert_eq!(first.incarnation(), second.incarnation());

    assert_eq!(
        machine
            .commit(first, BridgeServiceOutcome::Ready)
            .unwrap()
            .current(),
        BridgeServiceState::Ready
    );
    assert_eq!(
        machine
            .commit(second, BridgeServiceOutcome::Ready)
            .unwrap()
            .current(),
        BridgeServiceState::Ready
    );
}

#[test]
fn a_shutdown_admitted_beside_a_probe_still_stops_the_service() {
    let mut machine = ready_machine();
    let probe = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();
    let shutdown = machine.begin(BridgeServiceAction::Shutdown).unwrap();
    assert_eq!(probe.incarnation(), shutdown.incarnation());

    // The probe reports first, then the shutdown that was already admitted.
    assert_eq!(
        machine
            .commit(probe, BridgeServiceOutcome::Ready)
            .unwrap()
            .current(),
        BridgeServiceState::Ready
    );
    assert_eq!(
        machine
            .commit(shutdown, BridgeServiceOutcome::Stopped)
            .unwrap()
            .current(),
        BridgeServiceState::Stopped
    );
    assert_eq!(machine.state(), BridgeServiceState::Stopped);
}

#[test]
fn a_probe_observation_cannot_resurrect_a_stopped_service() {
    let mut machine = ready_machine();
    let probe = machine.begin(BridgeServiceAction::CheckReadiness).unwrap();
    let shutdown = machine.begin(BridgeServiceAction::Shutdown).unwrap();
    machine
        .commit(shutdown, BridgeServiceOutcome::Stopped)
        .unwrap();

    assert_eq!(
        machine
            .commit(probe, BridgeServiceOutcome::Ready)
            .unwrap_err(),
        BridgeSupervisionError::InvalidTransition {
            state: BridgeServiceState::Stopped,
            action: BridgeServiceAction::CheckReadiness,
        }
    );
    assert_eq!(machine.state(), BridgeServiceState::Stopped);
}

#[test]
fn admission_checks_the_same_ownership_and_state_rules_as_execution() {
    let machine = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
    assert_eq!(
        machine.begin(BridgeServiceAction::Shutdown).unwrap_err(),
        BridgeSupervisionError::InvalidTransition {
            state: BridgeServiceState::Absent,
            action: BridgeServiceAction::Shutdown,
        }
    );

    let external = BridgeServiceMachine::new(BridgeServiceOwnership::ExternalRemote);
    assert_eq!(
        external.begin(BridgeServiceAction::Restart).unwrap_err(),
        BridgeSupervisionError::LifecycleNotOwned
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
