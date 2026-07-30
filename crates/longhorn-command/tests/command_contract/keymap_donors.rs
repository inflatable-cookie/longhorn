use longhorn_command::{
    CommandArgumentSchema, CommandBindingDefinition, CommandContextDefinition,
    CommandContextRevision, CommandContextSnapshot, CommandDefinition, CommandEffectiveKeymap,
    CommandKeyResolution, CommandKeyTrigger, CommandKeyboardInput, CommandKeyboardMode,
    CommandKeymapPreset, CommandLimits, CommandPhysicalCode, CommandPlatform, CommandPlatformScope,
    CommandRegistry, CommandRegistryBuilder, CommandRegistryGeneration, CommandTextInputPolicy,
    CommandTriggerModifiers, CommandVisibility, NoReservedCommandChords,
};
use longhorn_core::{
    CommandCategoryId, CommandContextId, CommandId, CommandKeymapPresetId, CommandRouteId,
    SchemaVersion,
};
use serde_json::Value;

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("fixture id")
}

fn donor_registry(
    contexts: &[(&str, Option<&str>)],
    commands: &[(&str, &str, CommandTextInputPolicy)],
) -> CommandRegistry {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    for (context_id, parent_id) in contexts {
        builder
            .register_context(CommandContextDefinition {
                id: id::<CommandContextId>(context_id),
                parent_id: parent_id.map(id::<CommandContextId>),
            })
            .expect("context");
    }
    for (command_id, context_id, text_input_policy) in commands {
        builder
            .register_command(CommandDefinition {
                id: id::<CommandId>(command_id),
                label: command_id.to_string(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("general")],
                keywords: Vec::new(),
                icon: None,
                allowed_contexts: vec![id::<CommandContextId>(context_id)],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: *text_input_policy,
                route: id::<CommandRouteId>(&format!("route:{command_id}")),
                arguments: CommandArgumentSchema::None,
            })
            .expect("command");
    }
    builder.seal().expect("registry")
}

fn binding(
    binding_id: &str,
    key: &str,
    context_id: &str,
    command_id: &str,
) -> CommandBindingDefinition {
    CommandBindingDefinition {
        id: id(binding_id),
        platform: CommandPlatformScope::Any,
        trigger: CommandKeyTrigger {
            code: CommandPhysicalCode::new(key).expect("code"),
            modifiers: CommandTriggerModifiers {
                primary: true,
                ..CommandTriggerModifiers::default()
            },
        },
        context_id: id(context_id),
        command_id: id(command_id),
        arguments: Value::Null,
    }
}

fn preset(id_value: &str, bindings: Vec<CommandBindingDefinition>) -> CommandKeymapPreset {
    CommandKeymapPreset {
        id: id::<CommandKeymapPresetId>(id_value),
        version: SchemaVersion::new(1).expect("version"),
        bindings,
    }
}

fn resolve(
    registry: &CommandRegistry,
    preset: &CommandKeymapPreset,
    path: &[&str],
    key: &str,
) -> CommandKeyResolution {
    let platform = CommandPlatform::MacOs;
    let chord = CommandKeyTrigger {
        code: CommandPhysicalCode::new(key).expect("code"),
        modifiers: CommandTriggerModifiers {
            primary: true,
            ..CommandTriggerModifiers::default()
        },
    }
    .resolve(platform)
    .expect("chord");
    let context = CommandContextSnapshot::new(
        CommandContextRevision::INITIAL,
        path.iter().copied().map(id).collect(),
    )
    .expect("context");
    CommandEffectiveKeymap::compile(registry, preset, &[], &NoReservedCommandChords)
        .expect("effective keymap")
        .resolve(
            platform,
            &CommandKeyboardInput {
                chord,
                repeat: false,
                composing: false,
                editable_text: false,
            },
            &context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("resolution")
}

#[test]
fn loophole_shaped_keymap_composes_the_full_context_hierarchy() {
    let registry = donor_registry(
        &[
            ("global", None),
            ("project", Some("global")),
            ("surface", Some("project")),
            ("region", Some("surface")),
            ("panel", Some("region")),
        ],
        &[
            (
                "loophole:transport.play",
                "project",
                CommandTextInputPolicy::Blocked,
            ),
            (
                "loophole:surface.toggle",
                "surface",
                CommandTextInputPolicy::Blocked,
            ),
            (
                "loophole:panel.close",
                "panel",
                CommandTextInputPolicy::Allowed,
            ),
        ],
    );
    let preset = preset(
        "loophole:default",
        vec![
            binding(
                "loophole:transport.play",
                "Space",
                "project",
                "loophole:transport.play",
            ),
            binding(
                "loophole:surface.toggle",
                "KeyK",
                "surface",
                "loophole:surface.toggle",
            ),
            binding(
                "loophole:panel.close",
                "KeyW",
                "panel",
                "loophole:panel.close",
            ),
        ],
    );

    let resolution = resolve(
        &registry,
        &preset,
        &["global", "project", "surface", "region", "panel"],
        "KeyW",
    );
    let CommandKeyResolution::Resolved { winner, .. } = resolution else {
        panic!("expected panel binding")
    };
    assert_eq!(
        winner.invocation.command_id,
        id::<CommandId>("loophole:panel.close")
    );
}

#[test]
fn jetstream_shaped_keymap_needs_only_one_global_context() {
    let registry = donor_registry(
        &[("global", None)],
        &[(
            "jetstream:file.open",
            "global",
            CommandTextInputPolicy::Allowed,
        )],
    );
    let preset = preset(
        "jetstream:default",
        vec![binding(
            "jetstream:file.open",
            "KeyO",
            "global",
            "jetstream:file.open",
        )],
    );

    let resolution = resolve(&registry, &preset, &["global"], "KeyO");
    assert!(matches!(resolution, CommandKeyResolution::Resolved { .. }));
}
