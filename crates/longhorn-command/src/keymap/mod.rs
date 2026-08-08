//! Keymap presets, sparse overrides, effective bindings, and key resolution.

mod compile;
mod resolve;
mod support;
mod types;

pub use compile::CommandEffectiveKeymap;
pub(crate) use support::{
    collect_conflicts, compile_binding, conflict_from_matches, context_is_descendant,
    effective_binding, gated, validate_base_target, validate_binding,
};
pub use types::{
    CommandBindingCandidate, CommandBindingDefinition, CommandBindingReplacement,
    CommandBindingSource, CommandBindingWinner, CommandCandidateDisposition,
    CommandEffectiveBinding, CommandInvocation, CommandKeyResolution, CommandKeyResolutionError,
    CommandKeyboardGate, CommandKeymapConflict, CommandKeymapError, CommandKeymapErrorCode,
    CommandKeymapOverride, CommandKeymapPreset, CommandShortcutRecord,
};
pub(crate) use types::{MAXIMUM_OVERRIDE_DIRECTIVES, MAXIMUM_PRESET_BINDINGS, keymap_error};
