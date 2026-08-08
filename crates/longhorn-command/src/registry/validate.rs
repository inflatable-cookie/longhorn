//! Registry seal validation.

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

use super::CommandRegistryBuilder;

pub(crate) fn insert_unique<K, V>(
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

pub(crate) fn validate_limits(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
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

pub(crate) fn check_count(category: &str, actual: usize, maximum: usize) -> Result<(), CommandRegistryError> {
    if actual > maximum {
        return Err(registry_error(
            CommandRegistryErrorCode::LimitExceeded,
            format!("registered {actual} {category}; maximum is {maximum}"),
        ));
    }
    Ok(())
}

pub(crate) fn validate_contexts(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
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

pub(crate) fn validate_commands(builder: &CommandRegistryBuilder) -> Result<(), CommandRegistryError> {
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

pub(crate) fn validate_text(
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

pub(crate) fn check_unique<T>(category: &str, values: &[T]) -> Result<(), CommandRegistryError>
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

pub(crate) fn canonicalize_command(mut command: CommandDefinition) -> CommandDefinition {
    command
        .keywords
        .sort_by_key(|keyword| keyword.as_str().to_lowercase());
    command.allowed_contexts.sort();
    command.required_capabilities.sort();
    command.arguments.canonicalize();
    command
}
