use std::{error::Error, fmt};

use longhorn_core::{CommandContextId, CommandId, CommandRequestId, CommandRouteId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CommandArgumentError, CommandArguments, CommandAvailability, CommandAvailabilityReason,
    CommandAvailabilityReasonCode, CommandAvailabilityRecord, CommandAvailabilitySnapshot,
    CommandCapabilitySnapshot, CommandContextSnapshot, CommandDefinition, CommandEvidence,
    CommandRegistry, CommandRegistryGeneration,
};

/// Consumer failure while loading fresh command admission facts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandSourceFailure {
    evidence: CommandEvidence,
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
    phase: CommandFailurePhase,
    code: CommandFailureCode,
    evidence: Option<CommandEvidence>,
}

impl CommandFailure {
    fn source(phase: CommandFailurePhase, failure: CommandSourceFailure) -> Self {
        Self {
            phase,
            code: CommandFailureCode::SourceFailed,
            evidence: Some(failure.evidence),
        }
    }

    const fn invalid(phase: CommandFailurePhase, code: CommandFailureCode) -> Self {
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
pub struct CommandAvailabilityProjectionError(CommandFailure);

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
    request_id: CommandRequestId,
    registry_generation: CommandRegistryGeneration,
    context_revision: crate::CommandContextRevision,
    matched_context_id: CommandContextId,
    command_id: CommandId,
    route: CommandRouteId,
    arguments: CommandArguments,
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
    request_id: CommandRequestId,
    outcome: CommandExecutionOutcome,
}

impl CommandExecutionResult {
    const fn new(request_id: CommandRequestId, outcome: CommandExecutionOutcome) -> Self {
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

/// Pure fresh-admission engine over one immutable command registry.
#[derive(Clone, Copy, Debug)]
pub struct CommandAdmissionEngine<'registry> {
    registry: &'registry CommandRegistry,
}

impl<'registry> CommandAdmissionEngine<'registry> {
    /// Binds fresh admission to one sealed registry generation.
    #[must_use]
    pub const fn new(registry: &'registry CommandRegistry) -> Self {
        Self { registry }
    }

    /// Projects complete current availability from freshly loaded facts.
    pub fn project_availability(
        &self,
        context_source: &mut impl CommandContextSource,
        capability_source: &mut impl CommandCapabilitySource,
        availability_source: &mut impl CommandAvailabilitySource,
    ) -> Result<CommandAvailabilitySnapshot, CommandAvailabilityProjectionError> {
        let context = context_source
            .current_context()
            .map_err(|failure| projection_source(CommandFailurePhase::Context, failure))?;
        self.validate_context(&context)
            .map_err(CommandAvailabilityProjectionError)?;
        let capabilities = capability_source
            .current_capabilities()
            .map_err(|failure| projection_source(CommandFailurePhase::Capability, failure))?;
        self.validate_capabilities(&capabilities)
            .map_err(CommandAvailabilityProjectionError)?;

        let mut records = Vec::with_capacity(self.registry.commands().len());
        for command in self.registry.commands() {
            let availability = match self.static_admission(command, &context, &capabilities) {
                Ok(_) => availability_source
                    .availability(command, &context, &capabilities)
                    .map_err(|failure| {
                        projection_source(CommandFailurePhase::Availability, failure)
                    })?,
                Err(availability) => availability,
            };
            records.push(CommandAvailabilityRecord::new(
                command.id.clone(),
                availability,
            ));
        }
        Ok(CommandAvailabilitySnapshot::new(
            self.registry.generation(),
            context.revision(),
            records,
        ))
    }

    /// Revalidates one request against fresh facts without calling an executor.
    pub fn admit(
        &self,
        request: CommandExecutionRequest,
        context_source: &mut impl CommandContextSource,
        capability_source: &mut impl CommandCapabilitySource,
        availability_source: &mut impl CommandAvailabilitySource,
    ) -> Result<AdmittedCommandInvocation, CommandExecutionResult> {
        let request_id = request.request_id.clone();
        if request.registry_generation != self.registry.generation() {
            return Err(CommandExecutionResult::new(
                request_id,
                CommandExecutionOutcome::StaleRegistry {
                    expected: request.registry_generation,
                    actual: self.registry.generation(),
                },
            ));
        }
        let Some(command) = self.registry.command(&request.command_id) else {
            return Err(CommandExecutionResult::new(
                request_id,
                CommandExecutionOutcome::UnknownCommand,
            ));
        };
        let arguments = match command.arguments.validate(&request.arguments) {
            Ok(arguments) => arguments,
            Err(error) => {
                return Err(CommandExecutionResult::new(
                    request_id,
                    CommandExecutionOutcome::InvalidArguments { error },
                ));
            }
        };

        let context = match context_source.current_context() {
            Ok(context) => context,
            Err(failure) => {
                return Err(failed_result(
                    request_id,
                    CommandFailure::source(CommandFailurePhase::Context, failure),
                ));
            }
        };
        if let Err(failure) = self.validate_context(&context) {
            return Err(failed_result(request_id, failure));
        }
        let capabilities = match capability_source.current_capabilities() {
            Ok(capabilities) => capabilities,
            Err(failure) => {
                return Err(failed_result(
                    request_id,
                    CommandFailure::source(CommandFailurePhase::Capability, failure),
                ));
            }
        };
        if let Err(failure) = self.validate_capabilities(&capabilities) {
            return Err(failed_result(request_id, failure));
        }
        let matched_context_id = match self.static_admission(command, &context, &capabilities) {
            Ok(context_id) => context_id,
            Err(availability) => {
                return Err(CommandExecutionResult::new(
                    request_id,
                    CommandExecutionOutcome::Unavailable { availability },
                ));
            }
        };
        let availability = match availability_source.availability(command, &context, &capabilities)
        {
            Ok(availability) => availability,
            Err(failure) => {
                return Err(failed_result(
                    request_id,
                    CommandFailure::source(CommandFailurePhase::Availability, failure),
                ));
            }
        };
        if !availability.is_available() {
            return Err(CommandExecutionResult::new(
                request_id,
                CommandExecutionOutcome::Unavailable { availability },
            ));
        }

        Ok(AdmittedCommandInvocation {
            request_id,
            registry_generation: self.registry.generation(),
            context_revision: context.revision(),
            matched_context_id,
            command_id: command.id.clone(),
            route: command.route.clone(),
            arguments,
        })
    }

    /// Admits then synchronously dispatches one invocation through the injected port.
    pub fn execute(
        &self,
        request: CommandExecutionRequest,
        context_source: &mut impl CommandContextSource,
        capability_source: &mut impl CommandCapabilitySource,
        availability_source: &mut impl CommandAvailabilitySource,
        executor: &mut impl CommandExecutor,
    ) -> CommandExecutionResult {
        let invocation = match self.admit(
            request,
            context_source,
            capability_source,
            availability_source,
        ) {
            Ok(invocation) => invocation,
            Err(result) => return result,
        };
        let outcome = executor.execute(&invocation);
        Self::complete(&invocation, outcome)
    }

    /// Maps a terminal from an asynchronous or otherwise external admitted route.
    #[must_use]
    pub fn complete(
        invocation: &AdmittedCommandInvocation,
        outcome: CommandExecutorOutcome,
    ) -> CommandExecutionResult {
        let outcome = match outcome {
            CommandExecutorOutcome::Succeeded { evidence } => {
                CommandExecutionOutcome::Succeeded { evidence }
            }
            CommandExecutorOutcome::Unauthorized { evidence } => {
                CommandExecutionOutcome::Unauthorized { evidence }
            }
            CommandExecutorOutcome::Cancelled { evidence } => {
                CommandExecutionOutcome::Cancelled { evidence }
            }
            CommandExecutorOutcome::Rejected { evidence } => {
                CommandExecutionOutcome::Rejected { evidence }
            }
            CommandExecutorOutcome::Failed { evidence } => CommandExecutionOutcome::Failed {
                failure: CommandFailure {
                    phase: CommandFailurePhase::Executor,
                    code: CommandFailureCode::ExecutorFailed,
                    evidence,
                },
            },
            CommandExecutorOutcome::Indeterminate { evidence } => {
                CommandExecutionOutcome::Indeterminate { evidence }
            }
        };
        CommandExecutionResult::new(invocation.request_id.clone(), outcome)
    }

    fn validate_context(&self, context: &CommandContextSnapshot) -> Result<(), CommandFailure> {
        let path = context.path_slice();
        if path.len() > self.registry.limits().maximum_context_depth {
            return Err(CommandFailure::invalid(
                CommandFailurePhase::Context,
                CommandFailureCode::InvalidContextSnapshot,
            ));
        }
        for (index, context_id) in path.iter().enumerate() {
            let Some(definition) = self.registry.context(context_id) else {
                return Err(CommandFailure::invalid(
                    CommandFailurePhase::Context,
                    CommandFailureCode::InvalidContextSnapshot,
                ));
            };
            if index == 0 {
                if definition.parent_id.is_some() {
                    return Err(CommandFailure::invalid(
                        CommandFailurePhase::Context,
                        CommandFailureCode::InvalidContextSnapshot,
                    ));
                }
            } else if definition.parent_id.as_ref() != path.get(index - 1) {
                return Err(CommandFailure::invalid(
                    CommandFailurePhase::Context,
                    CommandFailureCode::InvalidContextSnapshot,
                ));
            }
        }
        Ok(())
    }

    fn validate_capabilities(
        &self,
        capabilities: &CommandCapabilitySnapshot,
    ) -> Result<(), CommandFailure> {
        if capabilities
            .capabilities()
            .any(|capability_id| self.registry.capability(capability_id).is_none())
        {
            return Err(CommandFailure::invalid(
                CommandFailurePhase::Capability,
                CommandFailureCode::UnknownCapabilityFact,
            ));
        }
        Ok(())
    }

    fn static_admission(
        &self,
        command: &CommandDefinition,
        context: &CommandContextSnapshot,
        capabilities: &CommandCapabilitySnapshot,
    ) -> Result<CommandContextId, CommandAvailability> {
        if command
            .required_capabilities
            .iter()
            .any(|capability_id| !capabilities.contains(capability_id))
        {
            return Err(CommandAvailability::unsupported(
                CommandAvailabilityReason::new(
                    CommandAvailabilityReasonCode::MissingCapability,
                    None,
                ),
            ));
        }
        context
            .path_slice()
            .iter()
            .rev()
            .find(|context_id| command.allowed_contexts.contains(context_id))
            .cloned()
            .ok_or_else(|| {
                CommandAvailability::unavailable(CommandAvailabilityReason::new(
                    CommandAvailabilityReasonCode::ContextNotAllowed,
                    None,
                ))
            })
    }
}

fn projection_source(
    phase: CommandFailurePhase,
    failure: CommandSourceFailure,
) -> CommandAvailabilityProjectionError {
    CommandAvailabilityProjectionError(CommandFailure::source(phase, failure))
}

fn failed_result(request_id: CommandRequestId, failure: CommandFailure) -> CommandExecutionResult {
    CommandExecutionResult::new(request_id, CommandExecutionOutcome::Failed { failure })
}
