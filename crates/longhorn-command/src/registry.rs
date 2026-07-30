use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use longhorn_core::{CommandCapabilityId, CommandContextId, CommandId};
use serde::{Deserialize, Serialize};

use crate::{
    CommandArgumentError, CommandArguments, CommandCapabilityDefinition, CommandContextDefinition,
    CommandDefinition, CommandLimits, CommandRegistryDigest, CommandRegistryError,
    CommandRegistryErrorCode, CommandRegistryGeneration, CommandSearchError, CommandSearchHit,
    CommandSurface, CommandTextInputPolicy, CommandVisibility, error::registry_error,
    search::search_records,
};

/// Stable framework-neutral discovery projection for one command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandDiscoveryRecord {
    /// Stable semantic command identity.
    pub id: CommandId,
    /// Consumer-owned display label.
    pub label: String,
    /// Optional consumer-owned description.
    pub description: Option<String>,
    /// Ordered discovery category path.
    pub category_path: Vec<longhorn_core::CommandCategoryId>,
    /// Consumer-owned search keywords.
    pub keywords: Vec<crate::CommandKeyword>,
    /// Optional consumer-owned icon resolver token.
    pub icon: Option<String>,
    /// Shared discovery visibility.
    pub visibility: CommandVisibility,
    /// Editable-text admission posture.
    pub text_input_policy: CommandTextInputPolicy,
    /// Closed structural argument schema.
    pub arguments: crate::CommandArgumentSchema,
}

impl From<&CommandDefinition> for CommandDiscoveryRecord {
    fn from(command: &CommandDefinition) -> Self {
        Self {
            id: command.id.clone(),
            label: command.label.clone(),
            description: command.description.clone(),
            category_path: command.category_path.clone(),
            keywords: command.keywords.clone(),
            icon: command.icon.clone(),
            visibility: command.visibility,
            text_input_policy: command.text_input_policy,
            arguments: command.arguments.clone(),
        }
    }
}

/// Mutable pre-seal collection of command declarations.
#[derive(Clone, Debug)]
pub struct CommandRegistryBuilder {
    generation: CommandRegistryGeneration,
    limits: CommandLimits,
    commands: BTreeMap<CommandId, CommandDefinition>,
    contexts: BTreeMap<CommandContextId, CommandContextDefinition>,
    capabilities: BTreeMap<CommandCapabilityId, CommandCapabilityDefinition>,
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

/// Validated immutable declarations for one host command registry generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRegistry {
    generation: CommandRegistryGeneration,
    digest: CommandRegistryDigest,
    limits: CommandLimits,
    commands: Vec<CommandDefinition>,
    contexts: Vec<CommandContextDefinition>,
    capabilities: Vec<CommandCapabilityDefinition>,
    discovery: Vec<CommandDiscoveryRecord>,
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

fn insert_unique<K, V>(
    map: &mut BTreeMap<K, V>,
    id: K,
    value: V,
    category: &str,
) -> Result<(), CommandRegistryError>
where
    K: Ord + fmt::Display,
{
    if map.contains_key(&id) {
        return Err(registry_error(
            CommandRegistryErrorCode::DuplicateId,
            format!("duplicate command {category} {id}"),
        ));
    }
    map.insert(id, value);
    Ok(())
}

fn validate_limits(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
    if !builder.limits.is_valid() {
        return Err(registry_error(
            CommandRegistryErrorCode::InvalidLimits,
            "command registry limits are zero or exceed defensive ceilings",
        ));
    }
    check_count(
        "commands",
        builder.commands.len(),
        builder.limits.maximum_commands,
    )?;
    check_count(
        "contexts",
        builder.contexts.len(),
        builder.limits.maximum_contexts,
    )?;
    check_count(
        "capabilities",
        builder.capabilities.len(),
        builder.limits.maximum_capabilities,
    )
}

fn check_count(category: &str, actual: usize, maximum: usize) -> Result<(), CommandRegistryError> {
    if actual > maximum {
        return Err(registry_error(
            CommandRegistryErrorCode::LimitExceeded,
            format!("registered {actual} {category}; maximum is {maximum}"),
        ));
    }
    Ok(())
}

fn validate_contexts(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
    let root = builder
        .contexts
        .get(&CommandContextId::new("global").map_err(|error| {
            registry_error(
                CommandRegistryErrorCode::InvalidContextRoot,
                error.to_string(),
            )
        })?);
    if root.is_none_or(|root| root.parent_id.is_some()) {
        return Err(registry_error(
            CommandRegistryErrorCode::InvalidContextRoot,
            "context graph must contain parentless global root",
        ));
    }
    for context in builder.contexts.values() {
        if context.id.as_str() != "global" && context.parent_id.is_none() {
            return Err(registry_error(
                CommandRegistryErrorCode::InvalidContextRoot,
                format!("non-global context {} has no parent", context.id),
            ));
        }
        let mut visited = BTreeSet::new();
        let mut current = context;
        let mut depth = 1;
        loop {
            if !visited.insert(current.id.clone()) {
                return Err(registry_error(
                    CommandRegistryErrorCode::ContextCycle,
                    format!("context graph contains a cycle through {}", current.id),
                ));
            }
            if depth > builder.limits.maximum_context_depth {
                return Err(registry_error(
                    CommandRegistryErrorCode::ContextDepthExceeded,
                    format!(
                        "context path from {} exceeds depth {}",
                        context.id, builder.limits.maximum_context_depth
                    ),
                ));
            }
            let Some(parent_id) = &current.parent_id else {
                if current.id.as_str() != "global" {
                    return Err(registry_error(
                        CommandRegistryErrorCode::InvalidContextRoot,
                        format!("context path from {} does not reach global", context.id),
                    ));
                }
                break;
            };
            current = builder.contexts.get(parent_id).ok_or_else(|| {
                registry_error(
                    CommandRegistryErrorCode::MissingReference,
                    format!(
                        "context {} references unknown parent {parent_id}",
                        current.id
                    ),
                )
            })?;
            depth += 1;
        }
    }
    Ok(())
}

fn validate_commands(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
    for command in builder.commands.values() {
        validate_text(
            "command label",
            &command.label,
            builder.limits.maximum_label_bytes,
        )?;
        if let Some(description) = &command.description {
            validate_text(
                "command description",
                description,
                builder.limits.maximum_description_bytes,
            )?;
        }
        if let Some(icon) = &command.icon {
            validate_text(
                "command icon token",
                icon,
                builder.limits.maximum_label_bytes,
            )?;
        }
        check_count(
            "command categories",
            command.category_path.len(),
            builder.limits.maximum_categories_per_command,
        )?;
        check_unique("category", &command.category_path)?;
        check_count(
            "command keywords",
            command.keywords.len(),
            builder.limits.maximum_keywords_per_command,
        )?;
        for keyword in &command.keywords {
            validate_text(
                "command keyword",
                keyword.as_str(),
                builder.limits.maximum_keyword_bytes,
            )?;
        }
        let normalized_keywords: BTreeSet<_> = command
            .keywords
            .iter()
            .map(|keyword| keyword.as_str().to_lowercase())
            .collect();
        if normalized_keywords.len() != command.keywords.len() {
            return Err(registry_error(
                CommandRegistryErrorCode::DuplicateItem,
                format!("command {} contains duplicate keywords", command.id),
            ));
        }
        if command.allowed_contexts.is_empty() {
            return Err(registry_error(
                CommandRegistryErrorCode::MissingReference,
                format!("command {} declares no allowed context", command.id),
            ));
        }
        check_count(
            "allowed command contexts",
            command.allowed_contexts.len(),
            builder.limits.maximum_contexts_per_command,
        )?;
        check_unique("allowed context", &command.allowed_contexts)?;
        for context_id in &command.allowed_contexts {
            if !builder.contexts.contains_key(context_id) {
                return Err(registry_error(
                    CommandRegistryErrorCode::MissingReference,
                    format!(
                        "command {} references unknown context {context_id}",
                        command.id
                    ),
                ));
            }
        }
        check_count(
            "required command capabilities",
            command.required_capabilities.len(),
            builder.limits.maximum_capabilities_per_command,
        )?;
        check_unique("required capability", &command.required_capabilities)?;
        for capability_id in &command.required_capabilities {
            if !builder.capabilities.contains_key(capability_id) {
                return Err(registry_error(
                    CommandRegistryErrorCode::MissingReference,
                    format!(
                        "command {} references unknown capability {capability_id}",
                        command.id
                    ),
                ));
            }
        }
        if !command.visibility.is_valid() {
            return Err(registry_error(
                CommandRegistryErrorCode::InvalidVisibility,
                format!(
                    "command {} has contradictory or empty visibility",
                    command.id
                ),
            ));
        }
        command.arguments.validate_definition(builder.limits)?;
    }
    Ok(())
}

fn validate_text(
    category: &str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), CommandRegistryError> {
    if value.trim().is_empty() {
        return Err(registry_error(
            CommandRegistryErrorCode::EmptyText,
            format!("{category} is empty"),
        ));
    }
    if value.len() > maximum_bytes {
        return Err(registry_error(
            CommandRegistryErrorCode::TextTooLong,
            format!(
                "{category} contains {} bytes; maximum is {maximum_bytes}",
                value.len()
            ),
        ));
    }
    Ok(())
}

fn check_unique<T>(category: &str, values: &[T]) -> Result<(), CommandRegistryError>
where
    T: Ord + fmt::Display,
{
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() != values.len() {
        return Err(registry_error(
            CommandRegistryErrorCode::DuplicateItem,
            format!("{category} list contains a duplicate"),
        ));
    }
    Ok(())
}

fn canonicalize_command(mut command: CommandDefinition) -> CommandDefinition {
    command
        .keywords
        .sort_by_key(|keyword| keyword.as_str().to_lowercase());
    command.allowed_contexts.sort();
    command.required_capabilities.sort();
    command.arguments.canonicalize();
    command
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDigestMaterial<'registry> {
    limits: CommandLimits,
    commands: &'registry [CommandDefinition],
    contexts: &'registry [CommandContextDefinition],
    capabilities: &'registry [CommandCapabilityDefinition],
}

fn compute_digest(
    registry: &CommandRegistry,
) -> Result<CommandRegistryDigest, CommandRegistryError> {
    serde_json::to_vec(&RegistryDigestMaterial {
        limits: registry.limits,
        commands: &registry.commands,
        contexts: &registry.contexts,
        capabilities: &registry.capabilities,
    })
    .map(|bytes| CommandRegistryDigest::from_bytes(&bytes))
    .map_err(|error| {
        registry_error(
            CommandRegistryErrorCode::DigestEncoding,
            format!("could not encode command registry digest material: {error}"),
        )
    })
}
