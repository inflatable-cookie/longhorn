//! Keymap binding, conflict, and resolution types.

use std::{error::Error, fmt};

use longhorn_core::{
    CommandBindingId, CommandContextId, CommandId, CommandKeymapPresetId, SchemaVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CommandArguments, CommandKeyChord, CommandKeyTrigger, CommandPlatform, CommandPlatformScope,
    CommandTextInputPolicy,
};

pub(crate) const MAXIMUM_PRESET_BINDINGS: usize = 65_536;
pub(crate) const MAXIMUM_OVERRIDE_DIRECTIVES: usize = 65_536;

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

pub(crate) fn keymap_error(
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
