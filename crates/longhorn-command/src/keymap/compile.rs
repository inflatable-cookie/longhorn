//! Effective keymap compilation.

use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{CommandContextId, CommandId, CommandKeymapPresetId, SchemaVersion};

use crate::{
    CommandPlatform, CommandRegistry, CommandRegistryGeneration, CommandReservedChordPolicy,
    CommandTextInputPolicy,
};

use super::{
    CommandBindingDefinition, CommandBindingSource, CommandEffectiveBinding, CommandKeymapConflict,
    CommandKeymapError, CommandKeymapErrorCode, CommandKeymapOverride, CommandKeymapPreset,
    CommandShortcutRecord, MAXIMUM_OVERRIDE_DIRECTIVES, MAXIMUM_PRESET_BINDINGS, collect_conflicts,
    compile_binding, effective_binding, keymap_error, validate_base_target, validate_binding,
};

/// Immutable validated keymap derived from one preset and sparse directive set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandEffectiveKeymap {
    pub(crate) registry_generation: CommandRegistryGeneration,
    pub(crate) preset_id: CommandKeymapPresetId,
    pub(crate) preset_version: SchemaVersion,
    pub(crate) bindings: Vec<CommandEffectiveBinding>,
    pub(crate) conflicts: Vec<CommandKeymapConflict>,
    pub(crate) context_parents: BTreeMap<CommandContextId, Option<CommandContextId>>,
    pub(crate) text_input_policies: BTreeMap<CommandId, CommandTextInputPolicy>,
}

impl CommandEffectiveKeymap {
    /// Validates a preset and sparse directives against one sealed registry.
    pub fn compile(
        registry: &CommandRegistry,
        preset: &CommandKeymapPreset,
        overrides: &[CommandKeymapOverride],
        reserved: &impl CommandReservedChordPolicy,
    ) -> Result<Self, CommandKeymapError> {
        if preset.bindings.len() > MAXIMUM_PRESET_BINDINGS {
            return Err(keymap_error(
                CommandKeymapErrorCode::LimitExceeded,
                None,
                format!(
                    "preset contains {} bindings; maximum is {}",
                    preset.bindings.len(),
                    MAXIMUM_PRESET_BINDINGS
                ),
            ));
        }
        if overrides.len() > MAXIMUM_OVERRIDE_DIRECTIVES {
            return Err(keymap_error(
                CommandKeymapErrorCode::LimitExceeded,
                None,
                format!(
                    "keymap contains {} override directives; maximum is {}",
                    overrides.len(),
                    MAXIMUM_OVERRIDE_DIRECTIVES
                ),
            ));
        }

        let mut base = BTreeMap::new();
        for binding in &preset.bindings {
            if base.insert(binding.id.clone(), binding).is_some() {
                return Err(keymap_error(
                    CommandKeymapErrorCode::DuplicateBindingId,
                    Some(binding.id.clone()),
                    format!("duplicate preset binding {}", binding.id),
                ));
            }
        }

        let mut disabled = BTreeSet::new();
        let mut replacements = BTreeMap::new();
        let mut additions = BTreeMap::new();
        let mut targeted = BTreeSet::new();
        for directive in overrides {
            match directive {
                CommandKeymapOverride::Disable { binding_id } => {
                    validate_base_target(&base, binding_id, &mut targeted)?;
                    disabled.insert(binding_id.clone());
                }
                CommandKeymapOverride::Replace {
                    binding_id,
                    replacement,
                } => {
                    validate_base_target(&base, binding_id, &mut targeted)?;
                    replacements.insert(binding_id.clone(), replacement);
                }
                CommandKeymapOverride::Add { binding } => {
                    if base.contains_key(&binding.id) {
                        return Err(keymap_error(
                            CommandKeymapErrorCode::BindingIdCollision,
                            Some(binding.id.clone()),
                            format!(
                                "added override binding {} collides with a preset binding",
                                binding.id
                            ),
                        ));
                    }
                    if additions.insert(binding.id.clone(), binding).is_some() {
                        return Err(keymap_error(
                            CommandKeymapErrorCode::DuplicateBindingId,
                            Some(binding.id.clone()),
                            format!("duplicate added override binding {}", binding.id),
                        ));
                    }
                }
            }
        }

        let preset_source = CommandBindingSource::Preset {
            preset_id: preset.id.clone(),
            preset_version: preset.version,
        };
        let replacement_source = CommandBindingSource::Replacement {
            preset_id: preset.id.clone(),
            preset_version: preset.version,
        };
        let mut bindings = Vec::with_capacity(base.len() + additions.len());
        for (binding_id, definition) in base {
            let base_arguments = validate_binding(registry, definition, false, reserved)?;
            if disabled.contains(&binding_id) {
                continue;
            }
            if let Some(replacement) = replacements.get(&binding_id) {
                let replacement_definition = CommandBindingDefinition {
                    id: binding_id.clone(),
                    platform: replacement.platform,
                    trigger: replacement.trigger.clone(),
                    context_id: replacement.context_id.clone(),
                    command_id: replacement.command_id.clone(),
                    arguments: replacement.arguments.clone(),
                };
                bindings.push(compile_binding(
                    registry,
                    &replacement_definition,
                    replacement_source.clone(),
                    true,
                    reserved,
                )?);
            } else {
                bindings.push(effective_binding(
                    definition,
                    preset_source.clone(),
                    base_arguments,
                ));
            }
        }
        for definition in additions.into_values() {
            bindings.push(compile_binding(
                registry,
                definition,
                CommandBindingSource::AddedOverride,
                true,
                reserved,
            )?);
        }
        bindings.sort_by(|left, right| left.id.cmp(&right.id));

        let context_parents = registry
            .contexts()
            .map(|context| (context.id.clone(), context.parent_id.clone()))
            .collect();
        let text_input_policies = registry
            .commands()
            .map(|command| (command.id.clone(), command.text_input_policy))
            .collect();
        let conflicts = collect_conflicts(&bindings);

        Ok(Self {
            registry_generation: registry.generation(),
            preset_id: preset.id.clone(),
            preset_version: preset.version,
            bindings,
            conflicts,
            context_parents,
            text_input_policies,
        })
    }

    /// Returns the registry generation used for validation.
    #[must_use]
    pub const fn registry_generation(&self) -> CommandRegistryGeneration {
        self.registry_generation
    }

    /// Returns the active immutable preset identity.
    #[must_use]
    pub const fn preset_id(&self) -> &CommandKeymapPresetId {
        &self.preset_id
    }

    /// Returns the active preset content version.
    #[must_use]
    pub const fn preset_version(&self) -> SchemaVersion {
        self.preset_version
    }

    /// Returns effective bindings in stable binding-id order.
    pub fn bindings(&self) -> impl ExactSizeIterator<Item = &CommandEffectiveBinding> {
        self.bindings.iter()
    }

    /// Returns all known platform-specific conflicts in canonical order.
    pub fn conflicts(&self) -> impl ExactSizeIterator<Item = &CommandKeymapConflict> {
        self.conflicts.iter()
    }

    /// Returns whether any unresolved different-invocation conflict exists.
    #[must_use]
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Returns effective bindings for one command in stable binding-id order.
    pub fn bindings_for_command(
        &self,
        command_id: &CommandId,
    ) -> impl Iterator<Item = &CommandEffectiveBinding> {
        self.bindings
            .iter()
            .filter(move |binding| &binding.invocation.command_id == command_id)
    }

    /// Projects platform-specific chords and labels for one command.
    #[must_use]
    pub fn shortcuts_for_command(
        &self,
        command_id: &CommandId,
        platform: CommandPlatform,
    ) -> Vec<CommandShortcutRecord> {
        self.bindings_for_command(command_id)
            .filter(|binding| binding.platform.includes(platform))
            .map(|binding| {
                let chord = binding
                    .trigger
                    .resolve(platform)
                    .expect("effective binding modifiers were validated");
                CommandShortcutRecord {
                    binding_id: binding.id.clone(),
                    source: binding.source.clone(),
                    context_id: binding.context_id.clone(),
                    label: chord.label(platform),
                    chord,
                }
            })
            .collect()
    }
}
