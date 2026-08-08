//! Admission and execution ports.

use std::{error::Error, fmt};

use longhorn_core::{CommandContextId, CommandId, CommandRequestId, CommandRouteId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AdmittedCommandInvocation, CommandExecutorOutcome};

use crate::{
    CommandArgumentError, CommandArguments, CommandAvailability, CommandAvailabilityReason,
    CommandAvailabilityReasonCode, CommandAvailabilityRecord, CommandAvailabilitySnapshot,
    CommandCapabilitySnapshot, CommandContextSnapshot, CommandDefinition, CommandEvidence,
    CommandRegistry, CommandRegistryGeneration,
};

/// Consumer failure while loading fresh command admission facts.
/// Consumer failure while loading fresh command admission facts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandSourceFailure {
    pub(crate) evidence: CommandEvidence,
}

impl CommandSourceFailure {
    /// Constructs a bounded consumer-owned source failure.
    #[must_use]
    pub const fn new(evidence: CommandEvidence) -> Self {
        Self { evidence }
    }

    /// Returns opaque consumer-owned evidence.
    #[must_use]
    pub const fn evidence(&self) -> &CommandEvidence {
        &self.evidence
    }
}

/// Consumer port for reloading the current ordered hot-context path.
pub trait CommandContextSource {
    /// Loads fresh current context facts.
    fn current_context(&mut self) -> Result<CommandContextSnapshot, CommandSourceFailure>;
}

/// Consumer port for reloading current command capability facts.
pub trait CommandCapabilitySource {
    /// Loads fresh current command capabilities.
    fn current_capabilities(&mut self) -> Result<CommandCapabilitySnapshot, CommandSourceFailure>;
}

/// Consumer port for evaluating product-owned current command availability.
pub trait CommandAvailabilitySource {
    /// Evaluates one statically eligible command against fresh facts.
    fn availability(
        &mut self,
        command: &CommandDefinition,
        context: &CommandContextSnapshot,
        capabilities: &CommandCapabilitySnapshot,
    ) -> Result<CommandAvailability, CommandSourceFailure>;
}

/// Consumer port for one already-admitted renderer-local or typed-domain route.
pub trait CommandExecutor {
    /// Executes an admitted invocation and returns a typed terminal posture.
    fn execute(&mut self, invocation: &AdmittedCommandInvocation) -> CommandExecutorOutcome;
}

