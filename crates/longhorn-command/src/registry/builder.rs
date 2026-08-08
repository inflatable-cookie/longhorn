//! Unsealed registry builder.

use std::collections::BTreeMap;

use longhorn_core::{CommandCapabilityId, CommandContextId, CommandId};

use crate::{
    CommandCapabilityDefinition, CommandContextDefinition, CommandDefinition, CommandLimits,
    CommandRegistryDigest, CommandRegistryError, CommandRegistryGeneration,
};

use super::{
    CommandDiscoveryRecord, CommandRegistry, canonicalize_command, compute_digest, insert_unique,
    validate_commands, validate_contexts, validate_limits,
};
/// Mutable pre-seal collection of command declarations.

#[derive(Clone, Debug)]
pub struct CommandRegistryBuilder {
    pub(crate) generation: CommandRegistryGeneration,
    pub(crate) limits: CommandLimits,
    pub(crate) commands: BTreeMap<CommandId, CommandDefinition>,
    pub(crate) contexts: BTreeMap<CommandContextId, CommandContextDefinition>,
    pub(crate) capabilities: BTreeMap<CommandCapabilityId, CommandCapabilityDefinition>,
}

impl CommandRegistryBuilder {
    /// Starts one command registry generation with explicit limits.
    #[must_use]
    pub fn new(generation: CommandRegistryGeneration, limits: CommandLimits) -> Self {
        Self {
            generation,
            limits,
            commands: BTreeMap::new(),
            contexts: BTreeMap::new(),
            capabilities: BTreeMap::new(),
        }
    }

    /// Registers one context declaration.
    pub fn register_context(
        &mut self,
        definition: CommandContextDefinition,
    ) -> Result<(), CommandRegistryError> {
        insert_unique(
            &mut self.contexts,
            definition.id.clone(),
            definition,
            "context",
        )
    }

    /// Registers one composition capability.
    pub fn register_capability(
        &mut self,
        definition: CommandCapabilityDefinition,
    ) -> Result<(), CommandRegistryError> {
        insert_unique(
            &mut self.capabilities,
            definition.id.clone(),
            definition,
            "capability",
        )
    }

    /// Registers one command declaration.
    pub fn register_command(
        &mut self,
        definition: CommandDefinition,
    ) -> Result<(), CommandRegistryError> {
        insert_unique(
            &mut self.commands,
            definition.id.clone(),
            definition,
            "command",
        )
    }

    /// Validates, canonicalizes, and seals this registry generation.
    pub fn seal(self) -> Result<CommandRegistry, CommandRegistryError> {
        validate_limits(&self)?;
        validate_contexts(&self)?;
        validate_commands(&self)?;

        let contexts: Vec<_> = self.contexts.into_values().collect();
        let capabilities: Vec<_> = self.capabilities.into_values().collect();
        let mut commands: Vec<_> = self
            .commands
            .into_values()
            .map(canonicalize_command)
            .collect();
        commands.sort_by(|left, right| left.id.cmp(&right.id));
        let discovery = commands.iter().map(CommandDiscoveryRecord::from).collect();

        let mut registry = CommandRegistry {
            generation: self.generation,
            digest: CommandRegistryDigest::placeholder(),
            limits: self.limits,
            commands,
            contexts,
            capabilities,
            discovery,
        };
        registry.digest = compute_digest(&registry)?;
        Ok(registry)
    }
}
