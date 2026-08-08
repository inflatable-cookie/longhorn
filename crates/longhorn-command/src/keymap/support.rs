//! Keymap validation and conflict helpers.

use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{CommandBindingId, CommandContextId, CommandId};

use crate::{
    CommandArguments, CommandInvocation, CommandKeyChord, CommandKeyTrigger, CommandPlatform,
    CommandPlatformScope, CommandRegistry, CommandReservedChordPolicy, CommandTextInputPolicy,
};

use super::{
    CommandBindingDefinition, CommandBindingSource, CommandEffectiveBinding, CommandKeyResolution,
    CommandKeyboardGate, CommandKeymapConflict, CommandKeymapError, CommandKeymapErrorCode,
    CommandKeymapOverride, keymap_error,
};

pub(crate) fn gated(gate: CommandKeyboardGate) -> CommandKeyResolution {
    CommandKeyResolution::Gated {
        gate,
        candidates: Vec::new(),
    }
}

pub(crate) fn validate_base_target(
    base: &BTreeMap<CommandBindingId, &CommandBindingDefinition>,
    binding_id: &CommandBindingId,
    targeted: &mut BTreeSet<CommandBindingId>,
) -> Result<(), CommandKeymapError> {
    if !base.contains_key(binding_id) {
        return Err(keymap_error(
            CommandKeymapErrorCode::MissingBaseBinding,
            Some(binding_id.clone()),
            format!("override names unknown base binding {binding_id}"),
        ));
    }
    if !targeted.insert(binding_id.clone()) {
        return Err(keymap_error(
            CommandKeymapErrorCode::DuplicateOverrideTarget,
            Some(binding_id.clone()),
            format!("multiple directives target base binding {binding_id}"),
        ));
    }
    Ok(())
}

pub(crate) fn compile_binding(
    registry: &CommandRegistry,
    definition: &CommandBindingDefinition,
    source: CommandBindingSource,
    check_reserved: bool,
    reserved: &impl CommandReservedChordPolicy,
) -> Result<CommandEffectiveBinding, CommandKeymapError> {
    let arguments = validate_binding(registry, definition, check_reserved, reserved)?;
    Ok(effective_binding(definition, source, arguments))
}

pub(crate) fn effective_binding(
    definition: &CommandBindingDefinition,
    source: CommandBindingSource,
    arguments: CommandArguments,
) -> CommandEffectiveBinding {
    CommandEffectiveBinding {
        id: definition.id.clone(),
        source,
        platform: definition.platform,
        trigger: definition.trigger.clone(),
        context_id: definition.context_id.clone(),
        invocation: CommandInvocation {
            command_id: definition.command_id.clone(),
            arguments,
        },
    }
}

pub(crate) fn validate_binding(
    registry: &CommandRegistry,
    definition: &CommandBindingDefinition,
    check_reserved: bool,
    reserved: &impl CommandReservedChordPolicy,
) -> Result<CommandArguments, CommandKeymapError> {
    let Some(command) = registry.command(&definition.command_id) else {
        return Err(keymap_error(
            CommandKeymapErrorCode::UnknownCommand,
            Some(definition.id.clone()),
            format!(
                "binding {} names unknown command {}",
                definition.id, definition.command_id
            ),
        ));
    };
    if !command.visibility.shortcut || command.visibility.hidden {
        return Err(keymap_error(
            CommandKeymapErrorCode::ShortcutNotEligible,
            Some(definition.id.clone()),
            format!(
                "binding {} targets command {} without shortcut visibility",
                definition.id, definition.command_id
            ),
        ));
    }
    if registry.context(&definition.context_id).is_none() {
        return Err(keymap_error(
            CommandKeymapErrorCode::UnknownContext,
            Some(definition.id.clone()),
            format!(
                "binding {} names unknown context {}",
                definition.id, definition.context_id
            ),
        ));
    }
    if !command
        .allowed_contexts
        .iter()
        .any(|allowed| context_is_descendant(registry, &definition.context_id, allowed))
    {
        return Err(keymap_error(
            CommandKeymapErrorCode::ContextNotAllowed,
            Some(definition.id.clone()),
            format!(
                "binding {} context {} is outside command {} admission contexts",
                definition.id, definition.context_id, definition.command_id
            ),
        ));
    }

    for platform in definition.platform.platforms() {
        let chord = definition.trigger.resolve(platform).map_err(|error| {
            keymap_error(
                CommandKeymapErrorCode::InvalidModifiers,
                Some(definition.id.clone()),
                format!("binding {} has invalid modifiers: {error}", definition.id),
            )
        })?;
        if check_reserved && reserved.is_reserved(platform, &chord) {
            return Err(keymap_error(
                CommandKeymapErrorCode::ReservedChord,
                Some(definition.id.clone()),
                format!(
                    "override binding {} claims a reserved chord on {platform:?}",
                    definition.id
                ),
            ));
        }
    }

    registry
        .validate_arguments(&definition.command_id, &definition.arguments)
        .expect("binding command was validated")
        .map_err(|error| {
            keymap_error(
                CommandKeymapErrorCode::InvalidArguments,
                Some(definition.id.clone()),
                format!("binding {} has invalid arguments: {error}", definition.id),
            )
        })
}

pub(crate) fn context_is_descendant(
    registry: &CommandRegistry,
    context_id: &CommandContextId,
    ancestor_id: &CommandContextId,
) -> bool {
    let mut current = Some(context_id);
    while let Some(context_id) = current {
        if context_id == ancestor_id {
            return true;
        }
        current = registry
            .context(context_id)
            .and_then(|context| context.parent_id.as_ref());
    }
    false
}

pub(crate) fn collect_conflicts(
    bindings: &[CommandEffectiveBinding],
) -> Vec<CommandKeymapConflict> {
    let mut groups: BTreeMap<
        (CommandPlatform, CommandKeyChord, CommandContextId),
        Vec<&CommandEffectiveBinding>,
    > = BTreeMap::new();
    for binding in bindings {
        for platform in binding.platform.platforms() {
            let chord = binding
                .trigger
                .resolve(platform)
                .expect("effective binding modifiers were validated");
            groups
                .entry((platform, chord, binding.context_id.clone()))
                .or_default()
                .push(binding);
        }
    }

    groups
        .into_iter()
        .filter_map(|((platform, chord, context_id), mut bindings)| {
            bindings.sort_by(|left, right| left.id.cmp(&right.id));
            let invocations: BTreeSet<_> = bindings
                .iter()
                .map(|binding| binding.invocation.clone())
                .collect();
            (invocations.len() > 1).then(|| CommandKeymapConflict {
                platform,
                chord,
                context_id,
                binding_ids: bindings
                    .into_iter()
                    .map(|binding| binding.id.clone())
                    .collect(),
                invocations: invocations.into_iter().collect(),
            })
        })
        .collect()
}

pub(crate) fn conflict_from_matches(
    platform: CommandPlatform,
    chord: CommandKeyChord,
    context_id: CommandContextId,
    matches: &[(&CommandEffectiveBinding, usize)],
    winning_specificity: usize,
) -> CommandKeymapConflict {
    let winning: Vec<_> = matches
        .iter()
        .take_while(|(_, specificity)| *specificity == winning_specificity)
        .map(|(binding, _)| *binding)
        .collect();
    let invocations = winning
        .iter()
        .map(|binding| binding.invocation.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    CommandKeymapConflict {
        platform,
        chord,
        context_id,
        binding_ids: winning
            .into_iter()
            .map(|binding| binding.id.clone())
            .collect(),
        invocations,
    }
}
