use std::error::Error;

use longhorn_command::{
    CommandArgumentSchema, CommandBindingDefinition, CommandContextDefinition,
    CommandContextRevision, CommandContextSnapshot, CommandDefinition, CommandEffectiveKeymap,
    CommandKeyChord, CommandKeyTrigger, CommandKeyboardInput, CommandKeyboardMode,
    CommandKeymapOverride, CommandKeymapPreset, CommandLimits, CommandModifiers,
    CommandPhysicalCode, CommandPlatform, CommandPlatformScope, CommandRegistryBuilder,
    CommandRegistryGeneration, CommandReservedChordPolicy, CommandSurface, CommandTextInputPolicy,
    CommandTriggerModifiers, CommandVisibility, NoReservedCommandChords,
};
use longhorn_command_config::{
    CommandCatalogueChangedEvent, CommandCatalogueSnapshot, CommandKeymapChangedEvent,
    CommandKeymapCommit, CommandKeymapCommitEvidence, CommandKeymapDiagnostic,
    CommandKeymapDurability, CommandKeymapLoadOrigin, CommandKeymapLoadOutcome,
    CommandKeymapMutationOutcome, CommandKeymapMutationReceipt, CommandKeymapMutationResult,
    CommandKeymapPatch, CommandKeymapPresetRecord, CommandKeymapPreview,
    CommandKeymapPreviewResult, CommandKeymapProtocolVersion, CommandKeymapRecovery,
    CommandKeymapRecoveryCode, CommandKeymapRejection, CommandKeymapRejectionCode,
    CommandKeymapReset, CommandKeymapRevision, CommandKeymapSnapshot, CommandKeymapState,
};
use longhorn_core::{
    CommandBindingId, CommandCategoryId, CommandContextId, CommandId, CommandRequestId,
    CommandRouteId, SchemaVersion,
};
use serde_json::{Value, json, to_value};


pub(crate) fn keyboard_input(
    code: &str,
    repeat: bool,
    composing: bool,
    editable_text: bool,
    control: bool,
    alt: bool,
    meta: bool,
) -> CommandKeyboardInput {
    CommandKeyboardInput {
        chord: CommandKeyChord {
            code: CommandPhysicalCode::new(code).expect("fixture physical code"),
            modifiers: CommandModifiers {
                control,
                alt,
                shift: false,
                meta,
            },
        },
        repeat,
        composing,
        editable_text,
    }
}

pub(crate) fn keyboard_case(
    keymap: &CommandEffectiveKeymap,
    platform: CommandPlatform,
    input: CommandKeyboardInput,
    context: &CommandContextSnapshot,
    mode: CommandKeyboardMode,
) -> Result<Value, Box<dyn Error>> {
    keyboard_case_with_reserved(
        keymap,
        platform,
        input,
        context,
        mode,
        &NoReservedCommandChords,
    )
}

pub(crate) fn keyboard_case_with_reserved(
    keymap: &CommandEffectiveKeymap,
    platform: CommandPlatform,
    input: CommandKeyboardInput,
    context: &CommandContextSnapshot,
    mode: CommandKeyboardMode,
    reserved: &impl CommandReservedChordPolicy,
) -> Result<Value, Box<dyn Error>> {
    let resolution = keymap.resolve(platform, &input, context, mode, reserved)?;
    Ok(json!({
        "platform": platform,
        "input": input,
        "contextPath": context.path().collect::<Vec<_>>(),
        "mode": mode,
        "bindings": keymap.bindings().collect::<Vec<_>>(),
        "consumed": resolution.is_consumed(),
        "resolution": resolution,
    }))
}

pub(crate) struct FixtureReservedChords;

impl CommandReservedChordPolicy for FixtureReservedChords {
    fn is_reserved(&self, _platform: CommandPlatform, chord: &CommandKeyChord) -> bool {
        chord.code.as_str() == "KeyQ"
    }
}

pub(crate) fn authority() -> Result<(longhorn_command::CommandRegistry, CommandKeymapPreset), Box<dyn Error>> {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder.register_context(CommandContextDefinition {
        id: id("global"),
        parent_id: None,
    })?;
    for value in ["app:open", "app:save"] {
        builder.register_command(CommandDefinition {
            id: id(value),
            label: value.into(),
            description: None,
            category_path: vec![id::<CommandCategoryId>("general")],
            keywords: Vec::new(),
            icon: None,
            allowed_contexts: vec![id::<CommandContextId>("global")],
            required_capabilities: Vec::new(),
            visibility: CommandVisibility::ALL,
            text_input_policy: CommandTextInputPolicy::Blocked,
            route: id::<CommandRouteId>(&format!("route:{value}")),
            arguments: CommandArgumentSchema::None,
        })?;
    }
    Ok((
        builder.seal()?,
        CommandKeymapPreset {
            id: id("app:default"),
            version: SchemaVersion::new(1)?,
            bindings: vec![
                binding("base:open", "KeyO", "app:open"),
                binding("base:save", "KeyS", "app:save"),
            ],
        },
    ))
}

pub(crate) fn binding(id_value: &str, code: &str, command: &str) -> CommandBindingDefinition {
    CommandBindingDefinition {
        id: binding_id(id_value),
        platform: CommandPlatformScope::Any,
        trigger: CommandKeyTrigger {
            code: CommandPhysicalCode::new(code).unwrap(),
            modifiers: CommandTriggerModifiers {
                primary: true,
                ..CommandTriggerModifiers::default()
            },
        },
        context_id: id("global"),
        command_id: id::<CommandId>(command),
        arguments: Value::Null,
    }
}

pub(crate) fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().unwrap()
}

pub(crate) fn binding_id(value: &str) -> CommandBindingId {
    id(value)
}

pub(crate) fn request_id(value: &str) -> CommandRequestId {
    id(value)
}
