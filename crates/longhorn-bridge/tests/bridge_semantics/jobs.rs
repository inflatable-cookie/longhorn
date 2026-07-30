use longhorn_bridge::{
    BridgeCancellationOutcome, BridgeCancellationReceipt, BridgeCancellationRequest,
    BridgeFailurePhase, BridgeJobTerminalDecision, BridgeJobTerminalEvent,
    BridgeJobTerminalOutcome, BridgeJobTracker, BridgeProgressDecision, BridgeProgressEvent,
    BridgeRetryClass,
};

use crate::support::{FailureDetail, context, failure, job_id, request_id};

#[test]
fn progress_cancel_and_terminal_metadata_round_trip_with_exact_correlation() {
    let progress = BridgeProgressEvent::new(request_id("request:scan"), job_id("job:scan"), 25_u8);
    let cancellation = BridgeCancellationRequest::new(
        context("request:cancel"),
        request_id("request:scan"),
        job_id("job:scan"),
    );
    let receipt = BridgeCancellationReceipt::<FailureDetail>::new(
        request_id("request:cancel"),
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeCancellationOutcome::Accepted,
    );
    let terminal = BridgeJobTerminalEvent::<u64, FailureDetail>::new(
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeJobTerminalOutcome::Succeeded(12),
    );

    let progress_json = serde_json::to_string(&progress).unwrap();
    let cancellation_json = serde_json::to_string(&cancellation).unwrap();
    let receipt_json = serde_json::to_string(&receipt).unwrap();
    let terminal_json = serde_json::to_string(&terminal).unwrap();
    assert_eq!(
        serde_json::from_str::<BridgeProgressEvent<u8>>(&progress_json).unwrap(),
        progress
    );
    assert_eq!(
        serde_json::from_str::<BridgeCancellationRequest>(&cancellation_json).unwrap(),
        cancellation
    );
    assert_eq!(
        serde_json::from_str::<BridgeCancellationReceipt<FailureDetail>>(&receipt_json).unwrap(),
        receipt
    );
    assert_eq!(
        serde_json::from_str::<BridgeJobTerminalEvent<u64, FailureDetail>>(&terminal_json).unwrap(),
        terminal
    );
}

#[test]
fn cancellation_acceptance_never_claims_terminal_completion() {
    let receipt = BridgeCancellationReceipt::<FailureDetail>::new(
        request_id("request:cancel"),
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeCancellationOutcome::Accepted,
    );
    assert!(matches!(
        receipt.outcome(),
        BridgeCancellationOutcome::Accepted
    ));

    let rejected = BridgeCancellationReceipt::new(
        request_id("request:cancel-rejected"),
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeCancellationOutcome::Rejected(failure(
            BridgeRetryClass::Never,
            BridgeFailurePhase::Authorization,
        )),
    );
    assert!(matches!(
        rejected.outcome(),
        BridgeCancellationOutcome::Rejected(_)
    ));
}

#[test]
fn job_tracker_accepts_only_correlated_progress_and_one_terminal() {
    let mut tracker = BridgeJobTracker::new(request_id("request:scan"), job_id("job:scan"));
    let progress = BridgeProgressEvent::new(request_id("request:scan"), job_id("job:scan"), 50_u8);
    let foreign = BridgeProgressEvent::new(request_id("request:other"), job_id("job:scan"), 75_u8);
    let terminal = BridgeJobTerminalEvent::<(), FailureDetail>::new(
        request_id("request:scan"),
        job_id("job:scan"),
        BridgeJobTerminalOutcome::Cancelled,
    );

    assert_eq!(
        tracker.classify_progress(&progress),
        BridgeProgressDecision::Accept
    );
    assert_eq!(
        tracker.classify_progress(&foreign),
        BridgeProgressDecision::IgnoreWrongCorrelation
    );
    assert_eq!(
        tracker.classify_terminal(&terminal),
        BridgeJobTerminalDecision::Accept
    );
    assert!(tracker.is_terminal());
    assert_eq!(
        tracker.classify_progress(&progress),
        BridgeProgressDecision::IgnoreAfterTerminal
    );
    assert_eq!(
        tracker.classify_terminal(&terminal),
        BridgeJobTerminalDecision::IgnoreAlreadyTerminal
    );
}
