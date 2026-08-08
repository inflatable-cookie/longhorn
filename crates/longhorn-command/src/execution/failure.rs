//! Command failure and availability projection errors.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::CommandEvidence;

use super::CommandSourceFailure;
/// Phase that failed while admitting or executing one command.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandFailurePhase {
    /// Current context could not be loaded or validated.
    Context,
    /// Current capability facts could not be loaded or validated.
    Capability,
    /// Product-owned availability could not be evaluated.
    Availability,
    /// The consumer executor reported failure.
    Executor,
}

/// Stable shared category for admission or execution failure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandFailureCode {
    /// A consumer source failed while loading current facts.
    SourceFailed,
    /// The current context snapshot contradicts the sealed context tree.
    InvalidContextSnapshot,
    /// Current capability facts name an unregistered command capability.
    UnknownCapabilityFact,
    /// The consumer executor reported a definite failure.
    ExecutorFailed,
}

/// Bounded failure produced during command admission or execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandFailure {
    pub(crate) phase: CommandFailurePhase,
    pub(crate) code: CommandFailureCode,
    pub(crate) evidence: Option<CommandEvidence>,
}

impl CommandFailure {
    pub(crate) fn source(phase: CommandFailurePhase, failure: CommandSourceFailure) -> Self {
        Self {
            phase,
            code: CommandFailureCode::SourceFailed,
            evidence: Some(failure.evidence),
        }
    }

    pub(crate) const fn invalid(phase: CommandFailurePhase, code: CommandFailureCode) -> Self {
        Self {
            phase,
            code,
            evidence: None,
        }
    }

    /// Returns the failed phase.
    #[must_use]
    pub const fn phase(&self) -> CommandFailurePhase {
        self.phase
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn code(&self) -> CommandFailureCode {
        self.code
    }

    /// Returns optional opaque consumer evidence.
    #[must_use]
    pub const fn evidence(&self) -> Option<&CommandEvidence> {
        self.evidence.as_ref()
    }
}

/// Error while projecting a complete fresh availability snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CommandAvailabilityProjectionError(pub(crate) CommandFailure);

impl CommandAvailabilityProjectionError {
    /// Returns the bounded failure.
    #[must_use]
    pub const fn failure(&self) -> &CommandFailure {
        &self.0
    }
}

impl fmt::Display for CommandAvailabilityProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "command availability projection failed in {:?}: {:?}",
            self.0.phase, self.0.code
        )
    }
}

impl Error for CommandAvailabilityProjectionError {}
