use longhorn_command::{
    CommandArgumentSchema, CommandCapabilityDefinition, CommandContextDefinition,
    CommandDefinition, CommandKeyword, CommandLimits, CommandRegistry, CommandRegistryBuilder,
    CommandRegistryGeneration, CommandTextInputPolicy, CommandVisibility,
};
use longhorn_core::{
    CommandCapabilityId, CommandCategoryId, CommandContextId, CommandFieldId, CommandId,
    CommandRouteId,
};

pub(crate) fn command_id(value: &str) -> CommandId {
    CommandId::new(value).expect("fixture command id")
}

pub(crate) fn context_id(value: &str) -> CommandContextId {
    CommandContextId::new(value).expect("fixture context id")
}

pub(crate) fn capability_id(value: &str) -> CommandCapabilityId {
    CommandCapabilityId::new(value).expect("fixture capability id")
}

pub(crate) fn field_id(value: &str) -> CommandFieldId {
    CommandFieldId::new(value).expect("fixture field id")
}

pub(crate) fn keyword(value: &str) -> CommandKeyword {
    CommandKeyword::new(value).expect("fixture keyword")
}

pub(crate) fn context(value: &str, parent: Option<&str>) -> CommandContextDefinition {
    CommandContextDefinition {
        id: context_id(value),
        parent_id: parent.map(context_id),
    }
}

pub(crate) fn capability(value: &str) -> CommandCapabilityDefinition {
    CommandCapabilityDefinition {
        id: capability_id(value),
    }
}

pub(crate) fn command(value: &str, label: &str, allowed_context: &str) -> CommandDefinition {
    CommandDefinition {
        id: command_id(value),
        label: label.to_owned(),
        description: Some(format!("Run {label}")),
        category_path: vec![CommandCategoryId::new("general").expect("fixture category id")],
        keywords: Vec::new(),
        icon: None,
        allowed_contexts: vec![context_id(allowed_context)],
        required_capabilities: Vec::new(),
        visibility: CommandVisibility::ALL,
        text_input_policy: CommandTextInputPolicy::Blocked,
        route: CommandRouteId::new(format!("consumer:{value}")).expect("fixture route id"),
        arguments: CommandArgumentSchema::None,
    }
}

pub(crate) fn minimal_registry(commands: Vec<CommandDefinition>) -> CommandRegistry {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global context");
    for command in commands {
        builder.register_command(command).expect("fixture command");
    }
    builder.seal().expect("fixture registry")
}
