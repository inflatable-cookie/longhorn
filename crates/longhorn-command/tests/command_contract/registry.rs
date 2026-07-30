use longhorn_command::{
    CommandCapabilityDefinition, CommandContextDefinition, CommandLimits, CommandRegistryBuilder,
    CommandRegistryErrorCode, CommandRegistryGeneration, CommandVisibility,
};
use longhorn_core::{CommandCapabilityId, CommandContextId, CommandId, OpaqueIdError};

use super::support::{capability, command, context};

#[test]
fn malformed_and_duplicate_ids_fail_before_seal() {
    assert_eq!(
        CommandId::new("Bad Command"),
        Err(OpaqueIdError::InvalidCharacter { index: 0 })
    );
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("first context");
    assert_eq!(
        builder
            .register_context(context("global", None))
            .expect_err("duplicate")
            .code(),
        CommandRegistryErrorCode::DuplicateId
    );
}

#[test]
fn contexts_require_one_bounded_acyclic_global_tree() {
    let cases = [
        vec![context("other", None)],
        vec![context("global", Some("other")), context("other", None)],
        vec![
            context("global", None),
            context("one", Some("two")),
            context("two", Some("one")),
        ],
        vec![context("global", None), context("one", Some("missing"))],
    ];
    let expected = [
        CommandRegistryErrorCode::InvalidContextRoot,
        CommandRegistryErrorCode::InvalidContextRoot,
        CommandRegistryErrorCode::ContextCycle,
        CommandRegistryErrorCode::MissingReference,
    ];
    for (contexts, expected) in cases.into_iter().zip(expected) {
        let mut builder = CommandRegistryBuilder::new(
            CommandRegistryGeneration::INITIAL,
            CommandLimits::default(),
        );
        for context in contexts {
            builder.register_context(context).expect("unique context");
        }
        assert_eq!(builder.seal().expect_err("invalid graph").code(), expected);
    }

    let limits = CommandLimits {
        maximum_context_depth: 2,
        ..CommandLimits::default()
    };
    let mut builder = CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, limits);
    for definition in [
        context("global", None),
        context("parent", Some("global")),
        context("child", Some("parent")),
    ] {
        builder.register_context(definition).expect("context");
    }
    assert_eq!(
        builder.seal().expect_err("depth").code(),
        CommandRegistryErrorCode::ContextDepthExceeded
    );
}

#[test]
fn command_references_and_visibility_are_validated_at_seal() {
    let mut unknown_context = command("test:context", "Context", "missing");
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global");
    builder
        .register_command(unknown_context.clone())
        .expect("command");
    assert_eq!(
        builder.seal().expect_err("unknown context").code(),
        CommandRegistryErrorCode::MissingReference
    );

    unknown_context.allowed_contexts = vec![CommandContextId::new("global").expect("id")];
    unknown_context.required_capabilities = vec![CommandCapabilityId::new("missing").expect("id")];
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global");
    builder
        .register_command(unknown_context.clone())
        .expect("command");
    assert_eq!(
        builder.seal().expect_err("unknown capability").code(),
        CommandRegistryErrorCode::MissingReference
    );

    unknown_context.required_capabilities.clear();
    unknown_context.visibility = CommandVisibility::default();
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global");
    builder.register_command(unknown_context).expect("command");
    assert_eq!(
        builder.seal().expect_err("empty visibility").code(),
        CommandRegistryErrorCode::InvalidVisibility
    );
}

#[test]
fn registry_digest_and_order_are_invariant_to_registration_order_and_generation() {
    fn build(reverse: bool, generation: u64) -> longhorn_command::CommandRegistry {
        let mut builder = CommandRegistryBuilder::new(
            CommandRegistryGeneration::new(generation),
            CommandLimits::default(),
        );
        let mut contexts = vec![context("global", None), context("editor", Some("global"))];
        let mut capabilities = vec![capability("editing"), capability("transport")];
        let mut commands = vec![
            command("test:alpha", "Alpha", "global"),
            command("test:beta", "Beta", "editor"),
        ];
        commands[0].required_capabilities =
            vec![CommandCapabilityId::new("transport").expect("capability")];
        commands[1].required_capabilities =
            vec![CommandCapabilityId::new("editing").expect("capability")];
        if reverse {
            contexts.reverse();
            capabilities.reverse();
            commands.reverse();
        }
        for value in contexts {
            builder.register_context(value).expect("context");
        }
        for value in capabilities {
            builder.register_capability(value).expect("capability");
        }
        for value in commands {
            builder.register_command(value).expect("command");
        }
        builder.seal().expect("registry")
    }

    let first = build(false, 4);
    let second = build(true, 9);
    assert_eq!(first.digest(), second.digest());
    assert_ne!(first.generation(), second.generation());
    assert_eq!(
        first
            .commands()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>(),
        ["test:alpha", "test:beta"]
    );
}

#[test]
fn explicit_limits_and_bounded_metadata_fail_closed() {
    let invalid_limits = CommandLimits {
        maximum_commands: 0,
        ..CommandLimits::default()
    };
    assert_eq!(
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, invalid_limits)
            .seal()
            .expect_err("limits")
            .code(),
        CommandRegistryErrorCode::InvalidLimits
    );

    let limits = CommandLimits {
        maximum_label_bytes: 4,
        ..CommandLimits::default()
    };
    let mut builder = CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, limits);
    builder
        .register_context(context("global", None))
        .expect("global");
    builder
        .register_command(command("test:long", "Long label", "global"))
        .expect("command");
    assert_eq!(
        builder.seal().expect_err("long text").code(),
        CommandRegistryErrorCode::TextTooLong
    );
}

#[test]
fn declarations_serialize_strictly() {
    assert!(
        serde_json::from_value::<CommandContextDefinition>(serde_json::json!({
            "id": "global",
            "parentId": null,
            "extra": true
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<CommandCapabilityDefinition>(serde_json::json!({
            "id": "editing"
        }))
        .is_ok()
    );
}
