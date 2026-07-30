use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{
    CommandBindingId, CommandContextId, CommandId, CommandKeymapPresetId, SchemaVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CommandArguments, CommandContextSnapshot, CommandKeyChord, CommandKeyTrigger,
    CommandKeyboardInput, CommandKeyboardMode, CommandPlatform, CommandPlatformScope,
    CommandRegistry, CommandRegistryGeneration, CommandReservedChordPolicy, CommandTextInputPolicy,
};

const MAXIMUM_PRESET_BINDINGS: usize = 65_536;
const MAXIMUM_OVERRIDE_DIRECTIVES: usize = 65_536;

/// One raw binding declared by an immutable preset or added override.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandBindingDefinition {
    /// Stable base or added-override binding identity.
    pub id: CommandBindingId,
    /// Supported platform posture.
    pub platform: CommandPlatformScope,
    /// One press-only physical trigger.
    pub trigger: CommandKeyTrigger,
    /// Context in which this binding participates.
    pub context_id: CommandContextId,
    /// Semantic command identity.
    pub command_id: CommandId,
    /// Raw arguments validated against the sealed registry.
    #[cfg_attr(feature = "bindings", ts(type = "unknown"))]
    pub arguments: Value,
}

/// Replacement payload for one existing base binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandBindingReplacement {
    /// Replacement platform posture.
    pub platform: CommandPlatformScope,
    /// Replacement physical trigger.
    pub trigger: CommandKeyTrigger,
    /// Replacement context.
    pub context_id: CommandContextId,
    /// Replacement command.
    pub command_id: CommandId,
    /// Replacement raw arguments.
    #[cfg_attr(feature = "bindings", ts(type = "unknown"))]
    pub arguments: Value,
}

/// Immutable consumer-supplied keymap preset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapPreset {
    /// Stable preset identity.
    pub id: CommandKeymapPresetId,
    /// Positive preset content version.
    pub version: SchemaVersion,
    /// Base bindings. Their input order carries no precedence.
    pub bindings: Vec<CommandBindingDefinition>,
}

/// One sparse user override directive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CommandKeymapOverride {
    /// Remove one base binding while preserving its identity as a directive.
    Disable {
        /// Target base binding.
        binding_id: CommandBindingId,
    },
    /// Replace one base binding while retaining its stable base identity.
    Replace {
        /// Target base binding.
        binding_id: CommandBindingId,
        /// Complete replacement payload.
        replacement: CommandBindingReplacement,
    },
    /// Add one new binding with its own stable override identity.
    Add {
        /// Added binding.
        binding: CommandBindingDefinition,
    },
}

/// Normalized command invocation carried by an effective binding.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandInvocation {
    /// Semantic command identity.
    pub command_id: CommandId,
    /// Structurally validated canonical arguments.
    pub arguments: CommandArguments,
}

/// Provenance for one effective binding.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CommandBindingSource {
    /// Unchanged immutable preset binding.
    Preset {
        /// Preset identity.
        preset_id: CommandKeymapPresetId,
        /// Preset content version.
        preset_version: SchemaVersion,
    },
    /// Sparse replacement of one preset binding.
    Replacement {
        /// Preset identity containing the replaced base binding.
        preset_id: CommandKeymapPresetId,
        /// Preset content version.
        preset_version: SchemaVersion,
    },
    /// Sparse added override binding.
    AddedOverride,
}

/// One validated binding in an immutable effective keymap.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandEffectiveBinding {
    /// Stable effective binding identity.
    pub id: CommandBindingId,
    /// Preset or sparse-override provenance.
    pub source: CommandBindingSource,
    /// Supported platform posture.
    pub platform: CommandPlatformScope,
    /// Physical trigger declaration.
    pub trigger: CommandKeyTrigger,
    /// Registered context.
    pub context_id: CommandContextId,
    /// Normalized invocation.
    pub invocation: CommandInvocation,
}

/// One platform-specific shortcut projection for discovery or settings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandShortcutRecord {
    /// Stable binding identity.
    pub binding_id: CommandBindingId,
    /// Preset or override provenance.
    pub source: CommandBindingSource,
    /// Binding context.
    pub context_id: CommandContextId,
    /// Canonical platform chord.
    pub chord: CommandKeyChord,
    /// Deterministic platform label.
    pub label: String,
}

/// Why one runtime candidate did or did not win.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CommandCandidateDisposition {
    /// Canonical representative of one resolved invocation.
    Winner,
    /// Same invocation and specificity as the canonical representative.
    Equivalent,
    /// Lower-specificity candidate shadowed by the named context.
    Shadowed {
        /// More-specific winning or conflicting context.
        #[cfg_attr(feature = "bindings", ts(rename = "byContextId"))]
        by_context_id: CommandContextId,
    },
    /// Candidate participates in an equal-specificity conflict.
    Conflict,
}

/// Explainable candidate produced by the runtime resolver.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandBindingCandidate {
    /// Stable binding identity.
    pub binding_id: CommandBindingId,
    /// Preset or override provenance.
    pub source: CommandBindingSource,
    /// Matching context.
    pub matched_context_id: CommandContextId,
    /// Zero-based position in the ordered hot-context path.
    pub specificity: usize,
    /// Normalized invocation.
    pub invocation: CommandInvocation,
    /// Winner, equivalent, shadowed, or conflict posture.
    pub disposition: CommandCandidateDisposition,
}

/// One resolved binding and invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandBindingWinner {
    /// Stable representative binding identity.
    pub binding_id: CommandBindingId,
    /// Matching context.
    pub matched_context_id: CommandContextId,
    /// Normalized invocation.
    pub invocation: CommandInvocation,
}

/// Explainable equal-specificity conflict.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapConflict {
    /// Runtime platform.
    pub platform: CommandPlatform,
    /// Canonical conflicting chord.
    pub chord: CommandKeyChord,
    /// Shared winning-specificity context.
    pub context_id: CommandContextId,
    /// Stable conflicting binding identities.
    pub binding_ids: Vec<CommandBindingId>,
    /// Distinct normalized invocations proving ambiguity.
    pub invocations: Vec<CommandInvocation>,
}

/// Gate that prevented ordinary command dispatch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeyboardGate {
    /// Repeated press is outside v1.
    Repeat,
    /// IME or composition owns the press.
    Composition,
    /// Platform or shell reserves the chord.
    Reserved,
    /// Editable text owns focus and the command declaration blocks it.
    TextInput,
}

/// Runtime keyboard resolution using the same records projected to UI.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CommandKeyResolution {
    /// No effective binding matches.
    Unbound,
    /// A gate blocked dispatch.
    Gated {
        /// Blocking gate.
        gate: CommandKeyboardGate,
        /// Candidates found before a command-specific text gate.
        candidates: Vec<CommandBindingCandidate>,
    },
    /// Capture mode recorded the chord without dispatch.
    Captured {
        /// Captured canonical chord.
        chord: CommandKeyChord,
        /// Deterministic platform label.
        label: String,
    },
    /// Equal-specificity different invocations remain unresolved.
    Conflict {
        /// Conflict evidence.
        conflict: CommandKeymapConflict,
        /// Complete candidate report.
        candidates: Vec<CommandBindingCandidate>,
    },
    /// One normalized invocation resolved.
    Resolved {
        /// Resolved binding.
        winner: CommandBindingWinner,
        /// Complete candidate report.
        candidates: Vec<CommandBindingCandidate>,
    },
}

impl CommandKeyResolution {
    /// Returns whether the browser adapter must consume this press.
    #[must_use]
    pub const fn is_consumed(&self) -> bool {
        matches!(self, Self::Captured { .. } | Self::Resolved { .. })
    }

    /// Returns the runtime candidate report when binding lookup ran.
    #[must_use]
    pub fn candidates(&self) -> &[CommandBindingCandidate] {
        match self {
            Self::Gated { candidates, .. }
            | Self::Conflict { candidates, .. }
            | Self::Resolved { candidates, .. } => candidates,
            Self::Unbound | Self::Captured { .. } => &[],
        }
    }
}

/// Stable validation category for presets and sparse overrides.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKeymapErrorCode {
    /// Preset or directive count exceeds its defensive ceiling.
    LimitExceeded,
    /// A stable binding id appears more than once.
    DuplicateBindingId,
    /// More than one directive targets the same base binding.
    DuplicateOverrideTarget,
    /// A disable or replacement names no base binding.
    MissingBaseBinding,
    /// Added binding identity collides with a base binding.
    BindingIdCollision,
    /// Binding names no registered command.
    UnknownCommand,
    /// Binding names no registered context.
    UnknownContext,
    /// Binding context is outside the command's admitted context subtree.
    ContextNotAllowed,
    /// Command is not eligible for shortcut discovery or dispatch.
    ShortcutNotEligible,
    /// Raw arguments fail the registered command schema.
    InvalidArguments,
    /// Semantic primary creates a duplicate native modifier.
    InvalidModifiers,
    /// Sparse override claims an injected platform-reserved chord.
    ReservedChord,
}

/// Invalid immutable preset or sparse override state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandKeymapError {
    code: CommandKeymapErrorCode,
    binding_id: Option<CommandBindingId>,
    detail: String,
}

impl CommandKeymapError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> CommandKeymapErrorCode {
        self.code
    }

    /// Returns the affected binding when available.
    #[must_use]
    pub fn binding_id(&self) -> Option<&CommandBindingId> {
        self.binding_id.as_ref()
    }

    /// Returns the diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CommandKeymapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CommandKeymapError {}

fn keymap_error(
    code: CommandKeymapErrorCode,
    binding_id: Option<CommandBindingId>,
    detail: impl Into<String>,
) -> CommandKeymapError {
    CommandKeymapError {
        code,
        binding_id,
        detail: detail.into(),
    }
}

/// Invalid current context facts supplied to the keyboard resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandKeyResolutionError {
    /// Current context is absent from the sealed registry.
    UnknownContext,
    /// Current context path skips or misorders a registered parent.
    InvalidContextPath,
}

impl fmt::Display for CommandKeyResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownContext => {
                formatter.write_str("keyboard context path names an unknown context")
            }
            Self::InvalidContextPath => {
                formatter.write_str("keyboard context path does not follow registered parents")
            }
        }
    }
}

impl Error for CommandKeyResolutionError {}

/// Immutable validated keymap derived from one preset and sparse directive set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandEffectiveKeymap {
    registry_generation: CommandRegistryGeneration,
    preset_id: CommandKeymapPresetId,
    preset_version: SchemaVersion,
    bindings: Vec<CommandEffectiveBinding>,
    conflicts: Vec<CommandKeymapConflict>,
    context_parents: BTreeMap<CommandContextId, Option<CommandContextId>>,
    text_input_policies: BTreeMap<CommandId, CommandTextInputPolicy>,
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

    /// Resolves one physical press against current context and injected gates.
    pub fn resolve(
        &self,
        platform: CommandPlatform,
        input: &CommandKeyboardInput,
        context: &CommandContextSnapshot,
        mode: CommandKeyboardMode,
        reserved: &impl CommandReservedChordPolicy,
    ) -> Result<CommandKeyResolution, CommandKeyResolutionError> {
        self.validate_context(context)?;

        if input.repeat {
            return Ok(gated(CommandKeyboardGate::Repeat));
        }
        if input.composing {
            return Ok(gated(CommandKeyboardGate::Composition));
        }
        if reserved.is_reserved(platform, &input.chord) {
            return Ok(gated(CommandKeyboardGate::Reserved));
        }
        if mode == CommandKeyboardMode::Capture {
            return Ok(CommandKeyResolution::Captured {
                chord: input.chord.clone(),
                label: input.chord.label(platform),
            });
        }

        let positions: BTreeMap<_, _> = context
            .path()
            .enumerate()
            .map(|(index, context_id)| (context_id, index))
            .collect();
        let mut matches: Vec<_> = self
            .bindings
            .iter()
            .filter(|binding| binding.platform.includes(platform))
            .filter_map(|binding| {
                let specificity = positions.get(&binding.context_id).copied()?;
                let chord = binding
                    .trigger
                    .resolve(platform)
                    .expect("effective binding modifiers were validated");
                (chord == input.chord).then_some((binding, specificity))
            })
            .collect();
        if matches.is_empty() {
            return Ok(CommandKeyResolution::Unbound);
        }
        matches.sort_by(|(left, left_specificity), (right, right_specificity)| {
            right_specificity
                .cmp(left_specificity)
                .then_with(|| left.id.cmp(&right.id))
        });

        let winning_specificity = matches[0].1;
        let winning_context = matches[0].0.context_id.clone();
        let winning_invocations: BTreeSet<_> = matches
            .iter()
            .take_while(|(_, specificity)| *specificity == winning_specificity)
            .map(|(binding, _)| binding.invocation.clone())
            .collect();
        let is_conflict = winning_invocations.len() > 1;
        let representative_id = matches[0].0.id.clone();
        let mut candidates = matches
            .iter()
            .map(|(binding, specificity)| CommandBindingCandidate {
                binding_id: binding.id.clone(),
                source: binding.source.clone(),
                matched_context_id: binding.context_id.clone(),
                specificity: *specificity,
                invocation: binding.invocation.clone(),
                disposition: if *specificity < winning_specificity {
                    CommandCandidateDisposition::Shadowed {
                        by_context_id: winning_context.clone(),
                    }
                } else if is_conflict {
                    CommandCandidateDisposition::Conflict
                } else if binding.id == representative_id {
                    CommandCandidateDisposition::Winner
                } else {
                    CommandCandidateDisposition::Equivalent
                },
            })
            .collect::<Vec<_>>();

        if is_conflict {
            let conflict = conflict_from_matches(
                platform,
                input.chord.clone(),
                winning_context,
                &matches,
                winning_specificity,
            );
            return Ok(CommandKeyResolution::Conflict {
                conflict,
                candidates,
            });
        }

        let binding = matches[0].0;
        let winner = CommandBindingWinner {
            binding_id: binding.id.clone(),
            matched_context_id: binding.context_id.clone(),
            invocation: binding.invocation.clone(),
        };
        if input.editable_text
            && self
                .text_input_policies
                .get(&winner.invocation.command_id)
                .is_some_and(|policy| *policy == CommandTextInputPolicy::Blocked)
        {
            return Ok(CommandKeyResolution::Gated {
                gate: CommandKeyboardGate::TextInput,
                candidates: {
                    candidates.shrink_to_fit();
                    candidates
                },
            });
        }

        Ok(CommandKeyResolution::Resolved { winner, candidates })
    }

    fn validate_context(
        &self,
        snapshot: &CommandContextSnapshot,
    ) -> Result<(), CommandKeyResolutionError> {
        let mut previous = None;
        for context_id in snapshot.path() {
            let Some(parent) = self.context_parents.get(context_id) else {
                return Err(CommandKeyResolutionError::UnknownContext);
            };
            if parent.as_ref() != previous {
                return Err(CommandKeyResolutionError::InvalidContextPath);
            }
            previous = Some(context_id);
        }
        Ok(())
    }
}

fn gated(gate: CommandKeyboardGate) -> CommandKeyResolution {
    CommandKeyResolution::Gated {
        gate,
        candidates: Vec::new(),
    }
}

fn validate_base_target(
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

fn compile_binding(
    registry: &CommandRegistry,
    definition: &CommandBindingDefinition,
    source: CommandBindingSource,
    check_reserved: bool,
    reserved: &impl CommandReservedChordPolicy,
) -> Result<CommandEffectiveBinding, CommandKeymapError> {
    let arguments = validate_binding(registry, definition, check_reserved, reserved)?;
    Ok(effective_binding(definition, source, arguments))
}

fn effective_binding(
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

fn validate_binding(
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

fn context_is_descendant(
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

fn collect_conflicts(bindings: &[CommandEffectiveBinding]) -> Vec<CommandKeymapConflict> {
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

fn conflict_from_matches(
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
