use longhorn_command::{
    CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandCapabilityDefinition,
    CommandContextDefinition, CommandDefinition, CommandKeyword, CommandLimits, CommandRegistry,
    CommandRegistryBuilder, CommandRegistryGeneration, CommandTextInputPolicy, CommandVisibility,
};
use longhorn_core::{
    CommandCapabilityId, CommandCategoryId, CommandContextId, CommandEnumValueId, CommandFieldId,
    CommandId, CommandRouteId,
};

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("fixture id")
}

fn seal(
    contexts: &[(&str, Option<&str>)],
    capabilities: &[&str],
    commands: Vec<CommandDefinition>,
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
    for capability_id in capabilities {
        builder
            .register_capability(CommandCapabilityDefinition {
                id: id::<CommandCapabilityId>(capability_id),
            })
            .expect("capability");
    }
    for command in commands {
        builder.register_command(command).expect("command");
    }
    builder.seal().expect("donor-shaped registry")
}

fn keyword(value: &str) -> CommandKeyword {
    CommandKeyword::new(value).expect("fixture keyword")
}

#[test]
fn loophole_shaped_catalogue_uses_full_context_hierarchy_without_product_types() {
    let registry = seal(
        &[
            ("global", None),
            ("project", Some("global")),
            ("surface", Some("project")),
            ("editor", Some("surface")),
            ("region", Some("surface")),
            ("panel", Some("region")),
        ],
        &["transport", "editing", "panels"],
        vec![
            CommandDefinition {
                id: id::<CommandId>("loophole:transport.play"),
                label: "Play".to_owned(),
                description: Some("Start project transport".to_owned()),
                category_path: vec![id::<CommandCategoryId>("transport")],
                keywords: vec![keyword("start"), keyword("playback")],
                icon: Some("play".to_owned()),
                allowed_contexts: vec![id::<CommandContextId>("project")],
                required_capabilities: vec![id::<CommandCapabilityId>("transport")],
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Blocked,
                route: id::<CommandRouteId>("loophole:transport.play"),
                arguments: CommandArgumentSchema::None,
            },
            CommandDefinition {
                id: id::<CommandId>("loophole:editor.quantize"),
                label: "Quantize Selection".to_owned(),
                description: Some("Quantize selected events to a grid".to_owned()),
                category_path: vec![
                    id::<CommandCategoryId>("edit"),
                    id::<CommandCategoryId>("timing"),
                ],
                keywords: vec![keyword("grid")],
                icon: None,
                allowed_contexts: vec![id::<CommandContextId>("editor")],
                required_capabilities: vec![id::<CommandCapabilityId>("editing")],
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Blocked,
                route: id::<CommandRouteId>("loophole:editor.quantize"),
                arguments: CommandArgumentSchema::Object {
                    fields: vec![CommandArgumentField {
                        id: id::<CommandFieldId>("grid"),
                        required: true,
                        default: None,
                        kind: CommandArgumentKind::Enum {
                            values: vec![
                                id::<CommandEnumValueId>("quarter"),
                                id::<CommandEnumValueId>("eighth"),
                                id::<CommandEnumValueId>("sixteenth"),
                            ],
                        },
                    }],
                },
            },
            CommandDefinition {
                id: id::<CommandId>("loophole:panel.close"),
                label: "Close Panel".to_owned(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("view")],
                keywords: Vec::new(),
                icon: Some("close".to_owned()),
                allowed_contexts: vec![id::<CommandContextId>("panel")],
                required_capabilities: vec![id::<CommandCapabilityId>("panels")],
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Allowed,
                route: id::<CommandRouteId>("loophole:panel.close"),
                arguments: CommandArgumentSchema::None,
            },
        ],
    );

    assert_eq!(registry.contexts().count(), 6);
    assert_eq!(registry.commands().count(), 3);
    assert_eq!(registry.capabilities().count(), 3);
}

#[test]
fn jetstream_shaped_catalogue_uses_only_global_and_editor_contexts() {
    let registry = seal(
        &[("global", None), ("editor", Some("global"))],
        &[],
        vec![
            CommandDefinition {
                id: id::<CommandId>("jetstream:file.open"),
                label: "Open File".to_owned(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("file")],
                keywords: vec![keyword("load")],
                icon: None,
                allowed_contexts: vec![id::<CommandContextId>("global")],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Allowed,
                route: id::<CommandRouteId>("jetstream:file.open"),
                arguments: CommandArgumentSchema::None,
            },
            CommandDefinition {
                id: id::<CommandId>("jetstream:editor.format"),
                label: "Format Document".to_owned(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("edit")],
                keywords: Vec::new(),
                icon: None,
                allowed_contexts: vec![id::<CommandContextId>("editor")],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Blocked,
                route: id::<CommandRouteId>("jetstream:editor.format"),
                arguments: CommandArgumentSchema::None,
            },
        ],
    );

    assert_eq!(registry.contexts().count(), 2);
    assert_eq!(registry.commands().count(), 2);
    assert_eq!(registry.capabilities().count(), 0);
}
