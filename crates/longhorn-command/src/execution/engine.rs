//! Command admission and execution engine.

use longhorn_core::CommandContextId;

use crate::{
    CommandAvailability, CommandAvailabilityReason, CommandAvailabilityReasonCode,
    CommandAvailabilityRecord, CommandAvailabilitySnapshot, CommandCapabilitySnapshot,
    CommandContextSnapshot, CommandDefinition, CommandRegistry,
};

use super::{
    AdmittedCommandInvocation, CommandAvailabilityProjectionError, CommandAvailabilitySource,
    CommandCapabilitySource, CommandContextSource, CommandExecutionOutcome,
    CommandExecutionRequest, CommandExecutionResult, CommandExecutor, CommandExecutorOutcome,
    CommandFailure, CommandFailureCode, CommandFailurePhase, failed_result, projection_source,
};
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

    pub(crate) fn validate_context(
        &self,
        context: &CommandContextSnapshot,
    ) -> Result<(), CommandFailure> {
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

    pub(crate) fn validate_capabilities(
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

    pub(crate) fn static_admission(
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
