use longhorn_bridge::{
    AuthorityEpoch, BridgeAuthorityCursorDecision, BridgeAuthorityRequirement,
    BridgeConnectionMachine, BridgeConnectionState, BridgeLifecycleErrorCode,
    BridgeRequiredAuthority, BridgeRetryClass, BridgeRetryLimit, BridgeStreamCursor,
    BridgeStreamSequence,
};

use super::support::{Clock, LinearBackoff, domain, receipt, session};

#[test]
fn ready_requires_negotiation_and_declared_authority() {
    let clock = Clock::new(10);
    let mut machine = BridgeConnectionMachine::new(BridgeRetryLimit::new(2).unwrap());

    assert_eq!(
        machine.connect(&clock).unwrap().current().state(),
        BridgeConnectionState::Connecting
    );
    assert_eq!(
        machine.transport_ready(&clock).unwrap().current().state(),
        BridgeConnectionState::Negotiating
    );

    let requirement = BridgeAuthorityRequirement::new(
        domain("example.workspace"),
        BridgeRequiredAuthority::Writable,
    );
    let denied = machine
        .accept_negotiation(
            &receipt("session:one", 1, false),
            std::slice::from_ref(&requirement),
            &clock,
        )
        .unwrap_err();
    assert_eq!(
        denied.code(),
        BridgeLifecycleErrorCode::RequiredAuthorityUnavailable
    );
    assert_eq!(machine.status().state(), BridgeConnectionState::Negotiating);
    assert!(machine.current_session_id().is_none());

    let accepted = machine
        .accept_negotiation(&receipt("session:one", 1, true), &[requirement], &clock)
        .unwrap();
    assert_eq!(accepted.current().state(), BridgeConnectionState::Ready);
    assert_eq!(accepted.session_id(), Some(&session("session:one")));
}

#[test]
fn reconnect_invalidates_session_and_authority_epoch() {
    let clock = Clock::new(100);
    let backoff = LinearBackoff;
    let mut machine = BridgeConnectionMachine::new(BridgeRetryLimit::new(2).unwrap());
    machine.connect(&clock).unwrap();
    machine.transport_ready(&clock).unwrap();
    machine
        .accept_negotiation(&receipt("session:one", 2, true), &[], &clock)
        .unwrap();

    let current = cursor("session:one", 2);
    assert_eq!(
        machine.classify_cursor(&current),
        BridgeAuthorityCursorDecision::Current
    );

    let reconnect = machine
        .reconnect(BridgeRetryClass::AfterReconnect, &clock, &backoff)
        .unwrap();
    assert_eq!(
        reconnect.current().state(),
        BridgeConnectionState::Reconnecting
    );
    assert_eq!(reconnect.reconnect().unwrap().attempt().get(), 1);
    assert_eq!(reconnect.reconnect().unwrap().not_before().get(), 125);
    assert!(machine.current_session_id().is_none());
    assert_eq!(
        machine.classify_cursor(&current),
        BridgeAuthorityCursorDecision::SupersededSession
    );

    clock.set(125);
    machine.transport_ready(&clock).unwrap();
    machine
        .accept_negotiation(&receipt("session:two", 3, true), &[], &clock)
        .unwrap();
    assert_eq!(
        machine.classify_cursor(&current),
        BridgeAuthorityCursorDecision::SupersededSession
    );
    assert_eq!(
        machine.classify_cursor(&cursor("session:two", 2)),
        BridgeAuthorityCursorDecision::StaleAuthority
    );
    assert_eq!(
        machine.classify_cursor(&cursor("session:two", 4)),
        BridgeAuthorityCursorDecision::RefreshAuthority
    );
}

#[test]
fn reconnect_exhaustion_becomes_offline_and_terminal_paths_are_explicit() {
    let clock = Clock::new(0);
    let mut machine = BridgeConnectionMachine::new(BridgeRetryLimit::new(1).unwrap());
    machine.connect(&clock).unwrap();
    machine.transport_ready(&clock).unwrap();
    machine
        .reconnect(BridgeRetryClass::AfterReconnect, &clock, &LinearBackoff)
        .unwrap();
    assert_eq!(
        machine.transport_ready(&clock).unwrap_err().code(),
        BridgeLifecycleErrorCode::RetryNotDue
    );
    clock.set(25);
    machine.transport_ready(&clock).unwrap();
    assert_eq!(
        machine
            .reconnect(BridgeRetryClass::AfterReconnect, &clock, &LinearBackoff)
            .unwrap()
            .current()
            .state(),
        BridgeConnectionState::Offline
    );
    assert_eq!(
        machine.close(&clock).unwrap().current().state(),
        BridgeConnectionState::Closed
    );
    assert_eq!(
        machine.connect(&clock).unwrap_err().code(),
        BridgeLifecycleErrorCode::InvalidTransition
    );
}

#[test]
fn never_retry_class_goes_offline_without_backoff() {
    let clock = Clock::new(0);
    let mut machine = BridgeConnectionMachine::new(BridgeRetryLimit::new(2).unwrap());
    machine.connect(&clock).unwrap();
    machine.transport_ready(&clock).unwrap();
    assert_eq!(
        machine
            .reconnect(BridgeRetryClass::Never, &clock, &LinearBackoff)
            .unwrap()
            .current()
            .state(),
        BridgeConnectionState::Offline
    );
}

#[test]
fn degrade_mismatch_unauthorized_failure_and_shutdown_paths_are_explicit() {
    let clock = Clock::new(0);

    let mut degraded = BridgeConnectionMachine::new(BridgeRetryLimit::new(0).unwrap());
    degraded.connect(&clock).unwrap();
    degraded.transport_ready(&clock).unwrap();
    degraded
        .accept_negotiation(&receipt("session:ready", 1, true), &[], &clock)
        .unwrap();
    assert_eq!(
        degraded
            .degrade(
                longhorn_bridge::BridgeConnectionReason::CapabilityChanged,
                &clock,
            )
            .unwrap()
            .current()
            .state(),
        BridgeConnectionState::Degraded
    );

    let mut incompatible = negotiating(&clock);
    assert_eq!(
        incompatible.incompatible(&clock).unwrap().current().state(),
        BridgeConnectionState::Incompatible
    );
    assert_eq!(
        incompatible.close(&clock).unwrap().current().state(),
        BridgeConnectionState::Closed
    );

    let mut unauthorized = negotiating(&clock);
    assert_eq!(
        unauthorized.unauthorized(&clock).unwrap().current().state(),
        BridgeConnectionState::Unauthorized
    );

    let mut failed = BridgeConnectionMachine::new(BridgeRetryLimit::new(0).unwrap());
    failed.connect(&clock).unwrap();
    assert_eq!(
        failed.fail(&clock).unwrap().current().state(),
        BridgeConnectionState::Failed
    );
}

fn cursor(session_id: &str, epoch: u64) -> BridgeStreamCursor {
    BridgeStreamCursor::new(
        session(session_id),
        domain("example.workspace"),
        AuthorityEpoch::new(epoch).unwrap(),
        BridgeStreamSequence::new(0),
    )
}

fn negotiating(clock: &Clock) -> BridgeConnectionMachine {
    let mut machine = BridgeConnectionMachine::new(BridgeRetryLimit::new(0).unwrap());
    machine.connect(clock).unwrap();
    machine.transport_ready(clock).unwrap();
    machine
}
