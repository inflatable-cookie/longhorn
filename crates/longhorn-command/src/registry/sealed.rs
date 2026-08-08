//! Sealed command registry authority.

use longhorn_core::{CommandCapabilityId, CommandContextId, CommandId};

use crate::{
    CommandArgumentError, CommandArguments, CommandCapabilityDefinition, CommandContextDefinition,
    CommandDefinition, CommandLimits, CommandRegistryDigest, CommandRegistryGeneration,
    CommandSearchError, CommandSearchHit, CommandSurface, search::search_records,
};

use super::CommandDiscoveryRecord;
/// Mutable pre-seal collection of command declarations.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRegistry {
    pub(crate) generation: CommandRegistryGeneration,
    pub(crate) digest: CommandRegistryDigest,
    pub(crate) limits: CommandLimits,
    pub(crate) commands: Vec<CommandDefinition>,
    pub(crate) contexts: Vec<CommandContextDefinition>,
    pub(crate) capabilities: Vec<CommandCapabilityDefinition>,
    pub(crate) discovery: Vec<CommandDiscoveryRecord>,
}

impl CommandRegistry {
    /// Returns the monotonic host generation.
    #[must_use]
    pub const fn generation(&self) -> CommandRegistryGeneration {
        self.generation
    }

    /// Returns the digest of canonical sealed content.
    #[must_use]
    pub fn digest(&self) -> &CommandRegistryDigest {
        &self.digest
    }

    /// Returns the explicit limits bound into this registry.
    #[must_use]
    pub const fn limits(&self) -> CommandLimits {
        self.limits
    }

    /// Returns commands in stable id order.
    pub fn commands(&self) -> impl ExactSizeIterator<Item = &CommandDefinition> {
        self.commands.iter()
    }

    /// Returns contexts in stable id order.
    pub fn contexts(&self) -> impl ExactSizeIterator<Item = &CommandContextDefinition> {
        self.contexts.iter()
    }

    /// Returns one registered context.
    #[must_use]
    pub fn context(&self, id: &CommandContextId) -> Option<&CommandContextDefinition> {
        self.contexts
            .binary_search_by(|context| context.id.cmp(id))
            .ok()
            .map(|index| &self.contexts[index])
    }

    /// Returns capabilities in stable id order.
    pub fn capabilities(&self) -> impl ExactSizeIterator<Item = &CommandCapabilityDefinition> {
        self.capabilities.iter()
    }

    /// Returns one registered command capability.
    #[must_use]
    pub fn capability(&self, id: &CommandCapabilityId) -> Option<&CommandCapabilityDefinition> {
        self.capabilities
            .binary_search_by(|capability| capability.id.cmp(id))
            .ok()
            .map(|index| &self.capabilities[index])
    }

    /// Returns one command declaration.
    #[must_use]
    pub fn command(&self, id: &CommandId) -> Option<&CommandDefinition> {
        self.commands
            .binary_search_by(|command| command.id.cmp(id))
            .ok()
            .map(|index| &self.commands[index])
    }

    /// Returns records visible on one surface in stable command-id order.
    pub fn discovery(
        &self,
        surface: CommandSurface,
    ) -> impl Iterator<Item = &CommandDiscoveryRecord> {
        self.discovery
            .iter()
            .filter(move |record| record.visibility.contains(surface))
    }

    /// Runs deterministic bounded search over records visible on one surface.
    pub fn search(
        &self,
        surface: CommandSurface,
        query: &str,
    ) -> Result<Vec<CommandSearchHit>, CommandSearchError> {
        if query.len() > self.limits.maximum_search_query_bytes {
            return Err(CommandSearchError::new(
                self.limits.maximum_search_query_bytes,
                query.len(),
            ));
        }
        Ok(search_records(self.discovery(surface), query))
    }

    /// Structurally validates arguments for one registered command.
    #[must_use]
    pub fn validate_arguments(
        &self,
        command_id: &CommandId,
        input: &serde_json::Value,
    ) -> Option<Result<CommandArguments, CommandArgumentError>> {
        self.command(command_id)
            .map(|command| command.arguments.validate(input))
    }
}
