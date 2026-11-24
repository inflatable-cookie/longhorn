use longhorn_core::WireCounter;
use std::{error::Error, fmt};

use longhorn_core::BridgeCredentialRef;
use serde::{Deserialize, Serialize};

/// Lifecycle ownership of one optional service host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeServiceOwnership {
    /// The consumer injected a locally spawned service Longhorn may supervise.
    OwnedLocal,
    /// The consumer injected an already-running local service.
    ExternalLocal,
    /// The consumer injected a remote host with external lifecycle ownership.
    ExternalRemote,
}

/// Observable state of one injected optional service.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeServiceState {
    /// No service has been spawned or attached.
    Absent,
    /// A consumer-owned local spawn was accepted.
    Starting,
    /// An external local or remote attach was accepted.
    Attaching,
    /// The service has not yet reported ready.
    AwaitingReadiness,
    /// The injected service reported ready.
    Ready,
    /// An owned local restart was accepted.
    Restarting,
    /// A connection-only retry was accepted.
    Reconnecting,
    /// An owned local service stopped.
    Stopped,
    /// The injected adapter reported a coded failure.
    Failed,
}

/// Consumer-injected supervision operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeServiceAction {
    /// Spawn a consumer-selected local executable.
    Spawn,
    /// Attach to a consumer-selected local or remote host.
    Attach,
    /// Probe readiness without acquiring or replacing a service.
    CheckReadiness,
    /// Restart an owned local service.
    Restart,
    /// Reconnect to an existing local or remote service.
    Reconnect,
    /// Shut down an owned local service.
    Shutdown,
}

/// Stable adapter failure category. No arbitrary message or credential material is admitted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeServiceFailureCode {
    /// The consumer-owned spawn failed.
    SpawnFailed,
    /// The consumer-owned attach failed.
    AttachFailed,
    /// Readiness did not complete successfully.
    ReadinessFailed,
    /// An owned local service exited unexpectedly.
    ServiceExited,
    /// A requested restart failed.
    RestartFailed,
    /// A requested reconnect failed.
    ReconnectFailed,
    /// A requested shutdown failed.
    ShutdownFailed,
}

/// Bounded supervision result returned by an injected adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeServiceOutcome {
    /// The requested asynchronous operation was accepted.
    Accepted,
    /// Readiness was positively observed.
    Ready,
    /// The service is alive but not yet ready.
    NotReady,
    /// An owned local service stopped.
    Stopped,
    /// The adapter reported a stable redacted failure.
    Failed(BridgeServiceFailureCode),
}

/// Checked request passed to a consumer-injected supervisor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeServiceRequest {
    action: BridgeServiceAction,
    credential_ref: Option<BridgeCredentialRef>,
}

impl BridgeServiceRequest {
    /// Constructs a request containing only an opaque credential reference.
    #[must_use]
    pub const fn new(
        action: BridgeServiceAction,
        credential_ref: Option<BridgeCredentialRef>,
    ) -> Self {
        Self {
            action,
            credential_ref,
        }
    }

    /// Returns the requested supervision action.
    #[must_use]
    pub const fn action(&self) -> BridgeServiceAction {
        self.action
    }

    /// Returns the opaque consumer credential lookup reference.
    #[must_use]
    pub const fn credential_ref(&self) -> Option<&BridgeCredentialRef> {
        self.credential_ref.as_ref()
    }
}

/// Consumer-implemented optional service supervision seam.
pub trait BridgeServiceSupervisor {
    /// Performs one already-admitted action and returns a bounded observation.
    fn perform(&mut self, request: &BridgeServiceRequest) -> BridgeServiceOutcome;
}

/// Monotonic supervision receipt generation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeServiceGeneration(WireCounter);

impl BridgeServiceGeneration {
    fn next(self) -> Result<Self, BridgeSupervisionError> {
        self.0
            .checked_add(1)
            .map(Self)
            .map_err(|_| BridgeSupervisionError::GenerationExhausted)
    }

    /// Returns the serialized generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Monotonic service incarnation.
///
/// One incarnation covers one service attempt. It advances only when a commit
/// starts an attempt (spawn, attach, restart or reconnect), never for a
/// readiness observation of the running attempt and never for the attempt's
/// end.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct BridgeServiceIncarnation(WireCounter);

impl BridgeServiceIncarnation {
    fn next(self) -> Result<Self, BridgeSupervisionError> {
        self.0
            .checked_add(1)
            .map(Self)
            .map_err(|_| BridgeSupervisionError::GenerationExhausted)
    }

    /// Returns the incarnation counter value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// One admitted supervision action bound to the incarnation that admitted it.
///
/// A machine-local token, not a wire shape: [`BridgeServiceMachine::begin`]
/// returns it and [`BridgeServiceMachine::commit`] accepts its observation only
/// while that incarnation is still current.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BridgeServiceAdmission {
    incarnation: BridgeServiceIncarnation,
    action: BridgeServiceAction,
}

impl BridgeServiceAdmission {
    /// Returns the service incarnation that admitted the action.
    #[must_use]
    pub const fn incarnation(self) -> BridgeServiceIncarnation {
        self.incarnation
    }

    /// Returns the admitted action.
    #[must_use]
    pub const fn action(self) -> BridgeServiceAction {
        self.action
    }
}

/// One committed and observable optional-service transition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeServiceTransitionReceipt {
    generation: BridgeServiceGeneration,
    ownership: BridgeServiceOwnership,
    action: BridgeServiceAction,
    previous: BridgeServiceState,
    current: BridgeServiceState,
    outcome: BridgeServiceOutcome,
}

impl BridgeServiceTransitionReceipt {
    /// Returns the service generation the transition belongs to.
    #[must_use]
    pub const fn generation(self) -> BridgeServiceGeneration {
        self.generation
    }

    /// Returns the ownership under which the transition was committed.
    #[must_use]
    pub const fn ownership(self) -> BridgeServiceOwnership {
        self.ownership
    }

    /// Returns the action that produced the transition.
    #[must_use]
    pub const fn action(self) -> BridgeServiceAction {
        self.action
    }

    /// Returns the service state this transition left.
    #[must_use]
    pub const fn previous(self) -> BridgeServiceState {
        self.previous
    }

    /// Returns current service state.
    #[must_use]
    pub const fn current(self) -> BridgeServiceState {
        self.current
    }

    /// Returns the redacted adapter observation.
    #[must_use]
    pub const fn outcome(self) -> BridgeServiceOutcome {
        self.outcome
    }
}

/// Pure ownership and state validator around an injected supervisor port.
#[derive(Clone, Debug)]
pub struct BridgeServiceMachine {
    ownership: BridgeServiceOwnership,
    state: BridgeServiceState,
    generation: BridgeServiceGeneration,
    incarnation: BridgeServiceIncarnation,
}

impl BridgeServiceMachine {
    /// Constructs an absent optional-service machine.
    #[must_use]
    pub const fn new(ownership: BridgeServiceOwnership) -> Self {
        Self {
            ownership,
            state: BridgeServiceState::Absent,
            generation: BridgeServiceGeneration(WireCounter::ZERO),
            incarnation: BridgeServiceIncarnation(WireCounter::ZERO),
        }
    }

    /// Returns current observable service state.
    #[must_use]
    pub const fn state(&self) -> BridgeServiceState {
        self.state
    }

    /// Executes one admitted request through the injected port and commits its observation.
    pub fn execute(
        &mut self,
        supervisor: &mut impl BridgeServiceSupervisor,
        request: BridgeServiceRequest,
    ) -> Result<BridgeServiceTransitionReceipt, BridgeSupervisionError> {
        let admission = self.begin(request.action())?;
        let outcome = supervisor.perform(&request);
        self.commit(admission, outcome)
    }

    /// Admits one action whose observation arrives after the call returns.
    ///
    /// The returned admission carries the service incarnation that admitted it,
    /// so the observation must be committed through [`Self::commit`]. A commit
    /// that starts a new attempt in between supersedes the admission and the
    /// delayed observation is rejected instead of being reported for the later
    /// service incarnation. Readiness observations of one attempt share its
    /// incarnation, so overlapping probes of that attempt all stay committable.
    pub fn begin(
        &self,
        action: BridgeServiceAction,
    ) -> Result<BridgeServiceAdmission, BridgeSupervisionError> {
        self.admit(action)?;
        Ok(BridgeServiceAdmission {
            incarnation: self.incarnation,
            action,
        })
    }

    /// Commits the observation for an action admitted by [`Self::begin`].
    ///
    /// Rejects the observation when a commit started a new service attempt
    /// after admission, or when the action is no longer valid in the current
    /// state. Neither a readiness observation nor the end of an attempt changes
    /// the incarnation, so a probe and a shutdown admitted together both land.
    pub fn commit(
        &mut self,
        admission: BridgeServiceAdmission,
        outcome: BridgeServiceOutcome,
    ) -> Result<BridgeServiceTransitionReceipt, BridgeSupervisionError> {
        if admission.incarnation != self.incarnation {
            return Err(BridgeSupervisionError::StaleObservation {
                admitted: admission.incarnation,
                current: self.incarnation,
            });
        }
        self.admit(admission.action)?;
        self.apply(admission.action, outcome)
    }

    /// Commits an adapter observation that is current when the call is made.
    ///
    /// This admits and commits in one step, so it cannot carry an observation
    /// that started before the call. Use [`Self::begin`] and [`Self::commit`]
    /// for an observation that completes later.
    pub fn observe(
        &mut self,
        action: BridgeServiceAction,
        outcome: BridgeServiceOutcome,
    ) -> Result<BridgeServiceTransitionReceipt, BridgeSupervisionError> {
        let admission = self.begin(action)?;
        self.commit(admission, outcome)
    }

    fn apply(
        &mut self,
        action: BridgeServiceAction,
        outcome: BridgeServiceOutcome,
    ) -> Result<BridgeServiceTransitionReceipt, BridgeSupervisionError> {
        let current = transition_state(action, outcome)?;
        let generation = self.generation.next()?;
        let incarnation = if starts_incarnation(current) {
            self.incarnation.next()?
        } else {
            self.incarnation
        };
        let receipt = BridgeServiceTransitionReceipt {
            generation,
            ownership: self.ownership,
            action,
            previous: self.state,
            current,
            outcome,
        };
        self.state = current;
        self.generation = generation;
        self.incarnation = incarnation;
        Ok(receipt)
    }

    fn admit(&self, action: BridgeServiceAction) -> Result<(), BridgeSupervisionError> {
        let owned = self.ownership == BridgeServiceOwnership::OwnedLocal;
        let admitted = match action {
            BridgeServiceAction::Spawn => {
                owned
                    && matches!(
                        self.state,
                        BridgeServiceState::Absent
                            | BridgeServiceState::Stopped
                            | BridgeServiceState::Failed
                    )
            }
            BridgeServiceAction::Attach => {
                !owned
                    && matches!(
                        self.state,
                        BridgeServiceState::Absent
                            | BridgeServiceState::Stopped
                            | BridgeServiceState::Failed
                    )
            }
            BridgeServiceAction::CheckReadiness => matches!(
                self.state,
                BridgeServiceState::Starting
                    | BridgeServiceState::Attaching
                    | BridgeServiceState::AwaitingReadiness
                    | BridgeServiceState::Restarting
                    | BridgeServiceState::Reconnecting
                    | BridgeServiceState::Ready
            ),
            BridgeServiceAction::Restart => {
                owned
                    && matches!(
                        self.state,
                        BridgeServiceState::Ready | BridgeServiceState::Failed
                    )
            }
            BridgeServiceAction::Reconnect => matches!(
                self.state,
                BridgeServiceState::Ready
                    | BridgeServiceState::Failed
                    | BridgeServiceState::Stopped
            ),
            BridgeServiceAction::Shutdown => {
                owned
                    && !matches!(
                        self.state,
                        BridgeServiceState::Absent | BridgeServiceState::Stopped
                    )
            }
        };
        if admitted {
            Ok(())
        } else if matches!(
            action,
            BridgeServiceAction::Spawn
                | BridgeServiceAction::Restart
                | BridgeServiceAction::Shutdown
        ) && !owned
        {
            Err(BridgeSupervisionError::LifecycleNotOwned)
        } else {
            Err(BridgeSupervisionError::InvalidTransition {
                state: self.state,
                action,
            })
        }
    }
}

fn transition_state(
    action: BridgeServiceAction,
    outcome: BridgeServiceOutcome,
) -> Result<BridgeServiceState, BridgeSupervisionError> {
    let current = match outcome {
        BridgeServiceOutcome::Failed(_) => BridgeServiceState::Failed,
        BridgeServiceOutcome::Ready if matches!(action, BridgeServiceAction::CheckReadiness) => {
            BridgeServiceState::Ready
        }
        BridgeServiceOutcome::NotReady if matches!(action, BridgeServiceAction::CheckReadiness) => {
            BridgeServiceState::AwaitingReadiness
        }
        BridgeServiceOutcome::Stopped if action == BridgeServiceAction::Shutdown => {
            BridgeServiceState::Stopped
        }
        BridgeServiceOutcome::Accepted => match action {
            BridgeServiceAction::Spawn => BridgeServiceState::Starting,
            BridgeServiceAction::Attach => BridgeServiceState::Attaching,
            BridgeServiceAction::Restart => BridgeServiceState::Restarting,
            BridgeServiceAction::Reconnect => BridgeServiceState::Reconnecting,
            BridgeServiceAction::CheckReadiness | BridgeServiceAction::Shutdown => {
                return Err(BridgeSupervisionError::InvalidObservation { action, outcome });
            }
        },
        _ => return Err(BridgeSupervisionError::InvalidObservation { action, outcome }),
    };
    Ok(current)
}

/// Whether a committed state starts a fresh service attempt, and so a fresh
/// incarnation. Readiness observations and an attempt's end do not.
fn starts_incarnation(current: BridgeServiceState) -> bool {
    matches!(
        current,
        BridgeServiceState::Starting
            | BridgeServiceState::Attaching
            | BridgeServiceState::Restarting
            | BridgeServiceState::Reconnecting
    )
}

/// Supervision ownership or transition failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeSupervisionError {
    /// Longhorn cannot stop or replace an externally owned service.
    LifecycleNotOwned,
    /// The requested action is not valid in current state.
    InvalidTransition {
        /// Current service state.
        state: BridgeServiceState,
        /// Rejected action.
        action: BridgeServiceAction,
    },
    /// The adapter returned an outcome invalid for the action.
    InvalidObservation {
        /// Requested action.
        action: BridgeServiceAction,
        /// Rejected observation.
        outcome: BridgeServiceOutcome,
    },
    /// A supervision receipt generation or incarnation counter exhausted its
    /// integer domain.
    GenerationExhausted,
    /// The observation belongs to an incarnation a later commit superseded.
    StaleObservation {
        /// Incarnation that admitted the observation.
        admitted: BridgeServiceIncarnation,
        /// Incarnation current when the observation arrived.
        current: BridgeServiceIncarnation,
    },
}

impl fmt::Display for BridgeSupervisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LifecycleNotOwned => {
                formatter.write_str("bridge service lifecycle is externally owned")
            }
            Self::InvalidTransition { state, action } => {
                write!(
                    formatter,
                    "service action {action:?} is invalid in {state:?}"
                )
            }
            Self::InvalidObservation { action, outcome } => {
                write!(
                    formatter,
                    "service outcome {outcome:?} is invalid for {action:?}"
                )
            }
            Self::GenerationExhausted => {
                formatter.write_str("bridge supervision counter exhausted")
            }
            Self::StaleObservation { admitted, current } => write!(
                formatter,
                "service observation admitted at incarnation {} arrived at incarnation {}",
                admitted.get(),
                current.get()
            ),
        }
    }
}

impl Error for BridgeSupervisionError {}
