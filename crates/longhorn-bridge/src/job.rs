use longhorn_core::{BridgeJobId, BridgeRequestId};
use serde::{Deserialize, Serialize};

use crate::BridgeFailure;

/// Optional request-correlated progress update with consumer-owned payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeProgressEvent<P> {
    request_id: BridgeRequestId,
    job_id: BridgeJobId,
    progress: P,
}

impl<P> BridgeProgressEvent<P> {
    /// Constructs a progress update for one initiating request and job.
    #[must_use]
    pub const fn new(request_id: BridgeRequestId, job_id: BridgeJobId, progress: P) -> Self {
        Self {
            request_id,
            job_id,
            progress,
        }
    }

    /// Returns the initiating request identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns the optional job identity.
    #[must_use]
    pub const fn job_id(&self) -> &BridgeJobId {
        &self.job_id
    }

    /// Returns consumer-owned progress detail.
    #[must_use]
    pub const fn progress(&self) -> &P {
        &self.progress
    }
}

/// Typed terminal for one optional bridge job.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeJobTerminalOutcome<S, D> {
    /// The job completed successfully.
    Succeeded(S),
    /// The job ended with a stable coded failure.
    Failed(BridgeFailure<D>),
    /// The authority confirms terminal cancellation.
    Cancelled,
}

/// Optional request-correlated terminal event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeJobTerminalEvent<S, D> {
    request_id: BridgeRequestId,
    job_id: BridgeJobId,
    outcome: BridgeJobTerminalOutcome<S, D>,
}

impl<S, D> BridgeJobTerminalEvent<S, D> {
    /// Constructs a terminal event for one initiating request and job.
    #[must_use]
    pub const fn new(
        request_id: BridgeRequestId,
        job_id: BridgeJobId,
        outcome: BridgeJobTerminalOutcome<S, D>,
    ) -> Self {
        Self {
            request_id,
            job_id,
            outcome,
        }
    }

    /// Returns the initiating request identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns the optional job identity.
    #[must_use]
    pub const fn job_id(&self) -> &BridgeJobId {
        &self.job_id
    }

    /// Returns the typed terminal.
    #[must_use]
    pub const fn outcome(&self) -> &BridgeJobTerminalOutcome<S, D> {
        &self.outcome
    }
}

/// Result of requesting cancellation without overstating termination.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeCancellationOutcome<D> {
    /// The authority accepted the request; work may still be running.
    Accepted,
    /// The targeted job was already terminal.
    AlreadyTerminal,
    /// The authority does not know the targeted request or job.
    Unknown,
    /// The authority rejected cancellation with a coded failure.
    Rejected(BridgeFailure<D>),
}

/// Reply to one cancellation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeCancellationReceipt<D> {
    request_id: BridgeRequestId,
    target_request_id: BridgeRequestId,
    job_id: BridgeJobId,
    outcome: BridgeCancellationOutcome<D>,
}

impl<D> BridgeCancellationReceipt<D> {
    /// Constructs an exact cancellation receipt.
    #[must_use]
    pub const fn new(
        request_id: BridgeRequestId,
        target_request_id: BridgeRequestId,
        job_id: BridgeJobId,
        outcome: BridgeCancellationOutcome<D>,
    ) -> Self {
        Self {
            request_id,
            target_request_id,
            job_id,
            outcome,
        }
    }

    /// Returns the cancellation request identity.
    #[must_use]
    pub const fn request_id(&self) -> &BridgeRequestId {
        &self.request_id
    }

    /// Returns the initiating request targeted by cancellation.
    #[must_use]
    pub const fn target_request_id(&self) -> &BridgeRequestId {
        &self.target_request_id
    }

    /// Returns the targeted optional job.
    #[must_use]
    pub const fn job_id(&self) -> &BridgeJobId {
        &self.job_id
    }

    /// Returns cancellation admission, not inferred termination.
    #[must_use]
    pub const fn outcome(&self) -> &BridgeCancellationOutcome<D> {
        &self.outcome
    }
}

/// Classification of one progress event against a tracked job.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeProgressDecision {
    /// Accept the correlated progress update.
    Accept,
    /// Ignore progress for another request or job.
    IgnoreWrongCorrelation,
    /// Ignore progress after the tracked job became terminal.
    IgnoreAfterTerminal,
}

/// Classification of one terminal event against a tracked job.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeJobTerminalDecision {
    /// Accept the terminal and close the tracked job.
    Accept,
    /// Ignore a terminal for another request or job.
    IgnoreWrongCorrelation,
    /// Ignore a duplicate or later terminal.
    IgnoreAlreadyTerminal,
}

/// Pure correlation and terminal-state tracker for one optional job.
#[derive(Clone, Debug)]
pub struct BridgeJobTracker {
    request_id: BridgeRequestId,
    job_id: BridgeJobId,
    terminal: bool,
}

impl BridgeJobTracker {
    /// Starts tracking one initiating request and job.
    #[must_use]
    pub const fn new(request_id: BridgeRequestId, job_id: BridgeJobId) -> Self {
        Self {
            request_id,
            job_id,
            terminal: false,
        }
    }

    /// Classifies progress without interpreting its payload.
    #[must_use]
    pub fn classify_progress<P>(&self, event: &BridgeProgressEvent<P>) -> BridgeProgressDecision {
        if event.request_id != self.request_id || event.job_id != self.job_id {
            BridgeProgressDecision::IgnoreWrongCorrelation
        } else if self.terminal {
            BridgeProgressDecision::IgnoreAfterTerminal
        } else {
            BridgeProgressDecision::Accept
        }
    }

    /// Accepts at most one correctly correlated terminal.
    pub fn classify_terminal<S, D>(
        &mut self,
        event: &BridgeJobTerminalEvent<S, D>,
    ) -> BridgeJobTerminalDecision {
        if event.request_id != self.request_id || event.job_id != self.job_id {
            BridgeJobTerminalDecision::IgnoreWrongCorrelation
        } else if self.terminal {
            BridgeJobTerminalDecision::IgnoreAlreadyTerminal
        } else {
            self.terminal = true;
            BridgeJobTerminalDecision::Accept
        }
    }

    /// Returns whether a correctly correlated terminal has been accepted.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }
}
