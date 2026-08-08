//! Execution request, admission, and outcome types.

use longhorn_core::{CommandContextId, CommandId, CommandRequestId, CommandRouteId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CommandArgumentError, CommandArguments, CommandAvailability, CommandEvidence,
    CommandRegistryGeneration,
};

use super::CommandFailure;
/// Structurally unchecked command invocation request.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandExecutionRequest {
    /// Correlation identity.
    pub request_id: CommandRequestId,
    /// Registry generation observed by the caller.
    pub registry_generation: CommandRegistryGeneration,
    /// Semantic command identity.
    pub command_id: CommandId,
    /// Raw value admitted only through the registered structural schema.
    pub arguments: Value,
}

/// Fully checked invocation passed to a consumer executor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AdmittedCommandInvocation {
    pub(crate) request_id: CommandRequestId,
    pub(crate) registry_generation: CommandRegistryGeneration,
    pub(crate) context_revision: crate::CommandContextRevision,
    pub(crate) matched_context_id: CommandContextId,
    pub(crate) command_id: CommandId,
    pub(crate) route: CommandRouteId,
    pub(crate) arguments: CommandArguments,
}

impl AdmittedCommandInvocation {
    /// Returns request correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &CommandRequestId {
        &self.request_id
    }

    /// Returns the registry generation checked at admission.
    #[must_use]
    pub const fn registry_generation(&self) -> CommandRegistryGeneration {
        self.registry_generation
    }

    /// Returns the fresh consumer context revision checked at admission.
    #[must_use]
    pub const fn context_revision(&self) -> crate::CommandContextRevision {
        self.context_revision
    }

    /// Returns the most-specific admitted context.
    #[must_use]
    pub const fn matched_context_id(&self) -> &CommandContextId {
        &self.matched_context_id
    }

    /// Returns semantic command identity.
    #[must_use]
    pub const fn command_id(&self) -> &CommandId {
        &self.command_id
    }

    /// Returns the opaque consumer route.
    #[must_use]
    pub const fn route(&self) -> &CommandRouteId {
        &self.route
    }

    /// Returns normalized bounded arguments.
    #[must_use]
    pub const fn arguments(&self) -> &CommandArguments {
        &self.arguments
    }
}

/// Terminal posture returned by an injected consumer executor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CommandExecutorOutcome {
    /// Execution succeeded.
    Succeeded {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Product authorization rejected the invocation.
    Unauthorized {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Execution was cancelled.
    Cancelled {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Product semantic validation definitively rejected the invocation.
    Rejected {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Execution definitively failed.
    Failed {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// The executor cannot prove whether an authoritative effect occurred.
    Indeterminate {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
}

/// Typed command outcome spanning pre-admission and executor terminals.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CommandExecutionOutcome {
    /// Execution succeeded.
    Succeeded {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// The semantic command id is absent from the sealed registry.
    UnknownCommand,
    /// The caller observed a different registry generation.
    StaleRegistry {
        /// Generation supplied by the caller.
        expected: CommandRegistryGeneration,
        /// Current sealed generation.
        actual: CommandRegistryGeneration,
    },
    /// Structural argument validation failed.
    InvalidArguments {
        /// Exact structural failure.
        error: CommandArgumentError,
    },
    /// Fresh context, capability, or product facts reject execution.
    Unavailable {
        /// Fresh rejection posture.
        availability: CommandAvailability,
    },
    /// Product authorization rejected the invocation.
    Unauthorized {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Execution was cancelled.
    Cancelled {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Product semantic validation definitively rejected the invocation.
    Rejected {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
    /// Admission or execution definitively failed.
    Failed {
        /// Checked failure phase and category.
        failure: CommandFailure,
    },
    /// The executor cannot prove whether an authoritative effect occurred.
    Indeterminate {
        /// Optional bounded consumer evidence.
        evidence: Option<CommandEvidence>,
    },
}

/// Request-correlated command execution result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandExecutionResult {
    pub(crate) request_id: CommandRequestId,
    pub(crate) outcome: CommandExecutionOutcome,
}

impl CommandExecutionResult {
    pub(crate) const fn new(
        request_id: CommandRequestId,
        outcome: CommandExecutionOutcome,
    ) -> Self {
        Self {
            request_id,
            outcome,
        }
    }

    /// Returns request correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &CommandRequestId {
        &self.request_id
    }

    /// Returns the typed terminal outcome.
    #[must_use]
    pub const fn outcome(&self) -> &CommandExecutionOutcome {
        &self.outcome
    }
}
