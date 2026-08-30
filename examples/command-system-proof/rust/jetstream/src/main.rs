//! Jetstream-shape command registry proof: core command graph only, no
//! optional settings or Tauri edges.

use longhorn_command::{
    CommandArgumentSchema, CommandBindingDefinition, CommandContextDefinition,
    CommandContextRevision, CommandContextSnapshot, CommandDefinition, CommandEffectiveKeymap,
    CommandKeyResolution, CommandKeyTrigger, CommandKeyboardInput, CommandKeyboardMode,
    CommandKeymapPreset, CommandLimits, CommandPhysicalCode, CommandPlatform, CommandPlatformScope,
    CommandRegistryBuilder, CommandRegistryGeneration, CommandTextInputPolicy,
    CommandTriggerModifiers, CommandVisibility, NoReservedCommandChords,
};
use longhorn_core::{
    CommandBindingId, CommandCategoryId, CommandContextId, CommandId, CommandKeymapPresetId,
    CommandRouteId, SchemaVersion,
};
use serde_json::{Value, json};

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("proof id")
}

fn main() {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(CommandContextDefinition {
            id: id::<CommandContextId>("global"),
            parent_id: None,
        })
        .expect("global context");
    builder
        .register_command(CommandDefinition {
            id: id::<CommandId>("jetstream:file.open"),
            label: "Open File".into(),
            description: None,
            category_path: vec![id::<CommandCategoryId>("file")],
            keywords: vec![],
            icon: None,
            allowed_contexts: vec![id::<CommandContextId>("global")],
            required_capabilities: vec![],
            visibility: CommandVisibility::ALL,
            text_input_policy: CommandTextInputPolicy::Allowed,
            route: id::<CommandRouteId>("jetstream:local.open-file"),
            arguments: CommandArgumentSchema::None,
        })
        .expect("open command");
    let registry = builder.seal().expect("sealed Jetstream registry");
    let preset = CommandKeymapPreset {
        id: id::<CommandKeymapPresetId>("jetstream:default"),
        version: SchemaVersion::new(1).expect("preset version"),
        bindings: vec![CommandBindingDefinition {
            id: id::<CommandBindingId>("jetstream:binding.1"),
            platform: CommandPlatformScope::Any,
            trigger: CommandKeyTrigger {
                code: CommandPhysicalCode::new("KeyO").expect("physical code"),
                modifiers: CommandTriggerModifiers {
                    primary: true,
                    ..CommandTriggerModifiers::default()
                },
            },
            context_id: id::<CommandContextId>("global"),
            command_id: id::<CommandId>("jetstream:file.open"),
            arguments: Value::Null,
        }],
    };
    let keymap = CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
        .expect("effective Jetstream keymap");
    let context = CommandContextSnapshot::new(
        CommandContextRevision::INITIAL,
        vec![id::<CommandContextId>("global")],
    )
    .expect("global context");
    let resolution = keymap
        .resolve(
            CommandPlatform::MacOs,
            &CommandKeyboardInput {
                chord: CommandKeyTrigger {
                    code: CommandPhysicalCode::new("KeyO").expect("physical code"),
                    modifiers: CommandTriggerModifiers {
                        primary: true,
                        ..CommandTriggerModifiers::default()
                    },
                }
                .resolve(CommandPlatform::MacOs)
                .expect("resolved chord"),
                repeat: false,
                composing: false,
                editable_text: false,
            },
            &context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("keyboard resolution");
    let CommandKeyResolution::Resolved { winner, .. } = resolution else {
        panic!("Jetstream binding must resolve");
    };
    println!(
        "{}",
        json!({
            "shape": "jetstream",
            "contexts": registry.contexts().count(),
            "commands": registry.commands().count(),
            "commandId": winner.invocation.command_id.as_str(),
            "route": registry
                .command(&winner.invocation.command_id)
                .expect("command")
                .route
                .as_str(),
            "optionalEdges": [],
        })
    );
}
