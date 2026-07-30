use longhorn_command::{
    CommandArgumentSchema, CommandBindingDefinition, CommandBindingReplacement,
    CommandBindingSource, CommandCandidateDisposition, CommandContextDefinition,
    CommandContextRevision, CommandContextSnapshot, CommandDefinition, CommandEffectiveKeymap,
    CommandKeyChord, CommandKeyResolution, CommandKeyResolutionError, CommandKeyTrigger,
    CommandKeyboardGate, CommandKeyboardInput, CommandKeyboardMode, CommandKeymapErrorCode,
    CommandKeymapOverride, CommandKeymapPreset, CommandLimits, CommandPhysicalCode,
    CommandPlatform, CommandPlatformScope, CommandRegistry, CommandRegistryBuilder,
    CommandRegistryGeneration, CommandReservedChordPolicy, CommandTextInputPolicy,
    CommandTriggerModifiers, CommandVisibility, NoReservedCommandChords,
};
use longhorn_core::{
    CommandBindingId, CommandCategoryId, CommandId, CommandKeymapPresetId, CommandRouteId,
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

fn registry() -> CommandRegistry {
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    for (context_id, parent_id) in [
        ("global", None),
        ("project", Some("global")),
        ("surface", Some("project")),
        ("editor", Some("surface")),
        ("region", Some("surface")),
        ("panel", Some("region")),
    ] {
        builder
            .register_context(CommandContextDefinition {
                id: id(context_id),
                parent_id: parent_id.map(id),
            })
            .expect("context");
    }
    for (command_id, context_id, policy) in [
        ("app:global.open", "global", CommandTextInputPolicy::Allowed),
        (
            "app:surface.toggle",
            "surface",
            CommandTextInputPolicy::Blocked,
        ),
        (
            "app:editor.format",
            "editor",
            CommandTextInputPolicy::Blocked,
        ),
        ("app:panel.close", "panel", CommandTextInputPolicy::Allowed),
    ] {
        builder
            .register_command(CommandDefinition {
                id: id(command_id),
                label: command_id.to_owned(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("general")],
                keywords: Vec::new(),
                icon: None,
                allowed_contexts: vec![id(context_id)],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: policy,
                route: id::<CommandRouteId>(&format!("route:{command_id}")),
                arguments: CommandArgumentSchema::None,
            })
            .expect("command");
    }
    builder.seal().expect("registry")
}

fn code(value: &str) -> CommandPhysicalCode {
    CommandPhysicalCode::new(value).expect("physical code")
}

fn primary_trigger(value: &str) -> CommandKeyTrigger {
    CommandKeyTrigger {
        code: code(value),
        modifiers: CommandTriggerModifiers {
            primary: true,
            ..CommandTriggerModifiers::default()
        },
    }
}

fn binding(
    binding_id: &str,
    trigger: &str,
    context_id: &str,
    command_id: &str,
) -> CommandBindingDefinition {
    CommandBindingDefinition {
        id: id(binding_id),
        platform: CommandPlatformScope::Any,
        trigger: primary_trigger(trigger),
        context_id: id(context_id),
        command_id: id(command_id),
        arguments: Value::Null,
    }
}

fn preset(bindings: Vec<CommandBindingDefinition>) -> CommandKeymapPreset {
    CommandKeymapPreset {
        id: id::<CommandKeymapPresetId>("app:default"),
        version: SchemaVersion::new(1).expect("version"),
        bindings,
    }
}

fn context(path: &[&str]) -> CommandContextSnapshot {
    CommandContextSnapshot::new(
        CommandContextRevision::new(7),
        path.iter().copied().map(id).collect(),
    )
    .expect("context snapshot")
}

fn input(chord: CommandKeyChord) -> CommandKeyboardInput {
    CommandKeyboardInput {
        chord,
        repeat: false,
        composing: false,
        editable_text: false,
    }
}

fn chord(trigger: &str, platform: CommandPlatform) -> CommandKeyChord {
    primary_trigger(trigger)
        .resolve(platform)
        .expect("normalized chord")
}

#[derive(Clone, Debug)]
struct Reserved {
    chord: CommandKeyChord,
}

impl CommandReservedChordPolicy for Reserved {
    fn is_reserved(&self, _platform: CommandPlatform, chord: &CommandKeyChord) -> bool {
        chord == &self.chord
    }
}

#[test]
fn sparse_directives_compile_without_copying_or_mutating_the_preset() {
    let registry = registry();
    let preset = preset(vec![
        binding("base:open", "KeyO", "global", "app:global.open"),
        binding("base:format", "KeyF", "editor", "app:editor.format"),
    ]);
    let overrides = vec![
        CommandKeymapOverride::Disable {
            binding_id: id("base:open"),
        },
        CommandKeymapOverride::Replace {
            binding_id: id("base:format"),
            replacement: CommandBindingReplacement {
                platform: CommandPlatformScope::Any,
                trigger: primary_trigger("KeyK"),
                context_id: id("editor"),
                command_id: id("app:editor.format"),
                arguments: Value::Null,
            },
        },
        CommandKeymapOverride::Add {
            binding: binding("override:panel-close", "KeyW", "panel", "app:panel.close"),
        },
    ];

    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &overrides, &NoReservedCommandChords)
            .expect("effective keymap");
    let bindings = effective.bindings().collect::<Vec<_>>();

    assert_eq!(preset.bindings.len(), 2);
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].id, id::<CommandBindingId>("base:format"));
    assert!(matches!(
        bindings[0].source,
        CommandBindingSource::Replacement { .. }
    ));
    assert_eq!(
        bindings[1].id,
        id::<CommandBindingId>("override:panel-close")
    );
    assert_eq!(bindings[1].source, CommandBindingSource::AddedOverride);
}

#[test]
fn compilation_and_resolution_ignore_preset_and_directive_insertion_order() {
    let registry = registry();
    let first_preset = preset(vec![
        binding("base:open", "KeyO", "global", "app:global.open"),
        binding("base:format", "KeyF", "editor", "app:editor.format"),
    ]);
    let mut second_preset = first_preset.clone();
    second_preset.bindings.reverse();
    let first_overrides = vec![
        CommandKeymapOverride::Disable {
            binding_id: id("base:open"),
        },
        CommandKeymapOverride::Add {
            binding: binding("override:panel-close", "KeyW", "panel", "app:panel.close"),
        },
    ];
    let mut second_overrides = first_overrides.clone();
    second_overrides.reverse();

    let first = CommandEffectiveKeymap::compile(
        &registry,
        &first_preset,
        &first_overrides,
        &NoReservedCommandChords,
    )
    .expect("first");
    let second = CommandEffectiveKeymap::compile(
        &registry,
        &second_preset,
        &second_overrides,
        &NoReservedCommandChords,
    )
    .expect("second");

    assert_eq!(first, second);
}

#[test]
fn platform_specific_bindings_filter_before_context_resolution() {
    let registry = registry();
    let preset = preset(vec![
        CommandBindingDefinition {
            platform: CommandPlatformScope::MacOs,
            ..binding("base:mac-open", "KeyK", "global", "app:global.open")
        },
        CommandBindingDefinition {
            platform: CommandPlatformScope::Windows,
            ..binding(
                "base:windows-toggle",
                "KeyK",
                "surface",
                "app:surface.toggle",
            )
        },
    ]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");
    let hot_context = context(&["global", "project", "surface"]);

    for (platform, expected_command) in [
        (CommandPlatform::MacOs, "app:global.open"),
        (CommandPlatform::Windows, "app:surface.toggle"),
    ] {
        let resolution = effective
            .resolve(
                platform,
                &input(chord("KeyK", platform)),
                &hot_context,
                CommandKeyboardMode::Dispatch,
                &NoReservedCommandChords,
            )
            .expect("resolution");
        let CommandKeyResolution::Resolved { winner, .. } = resolution else {
            panic!("expected platform binding")
        };
        assert_eq!(
            winner.invocation.command_id,
            id::<CommandId>(expected_command)
        );
    }

    assert_eq!(
        effective
            .resolve(
                CommandPlatform::Linux,
                &input(chord("KeyK", CommandPlatform::Linux)),
                &hot_context,
                CommandKeyboardMode::Dispatch,
                &NoReservedCommandChords,
            )
            .expect("linux resolution"),
        CommandKeyResolution::Unbound
    );
}

#[test]
fn validation_rejects_unknown_invalid_duplicate_and_reserved_overrides() {
    let registry = registry();
    let base = preset(vec![binding(
        "base:open",
        "KeyO",
        "global",
        "app:global.open",
    )]);

    let missing = CommandEffectiveKeymap::compile(
        &registry,
        &base,
        &[CommandKeymapOverride::Disable {
            binding_id: id("base:missing"),
        }],
        &NoReservedCommandChords,
    )
    .expect_err("missing base");
    assert_eq!(missing.code(), CommandKeymapErrorCode::MissingBaseBinding);

    let duplicate = CommandEffectiveKeymap::compile(
        &registry,
        &base,
        &[
            CommandKeymapOverride::Disable {
                binding_id: id("base:open"),
            },
            CommandKeymapOverride::Replace {
                binding_id: id("base:open"),
                replacement: CommandBindingReplacement {
                    platform: CommandPlatformScope::Any,
                    trigger: primary_trigger("KeyP"),
                    context_id: id("global"),
                    command_id: id("app:global.open"),
                    arguments: Value::Null,
                },
            },
        ],
        &NoReservedCommandChords,
    )
    .expect_err("duplicate target");
    assert_eq!(
        duplicate.code(),
        CommandKeymapErrorCode::DuplicateOverrideTarget
    );

    let unknown_command = CommandEffectiveKeymap::compile(
        &registry,
        &base,
        &[CommandKeymapOverride::Add {
            binding: binding("override:unknown", "KeyU", "global", "app:missing"),
        }],
        &NoReservedCommandChords,
    )
    .expect_err("unknown command");
    assert_eq!(
        unknown_command.code(),
        CommandKeymapErrorCode::UnknownCommand
    );

    let reserved_chord = chord("KeyR", CommandPlatform::MacOs);
    let reserved = CommandEffectiveKeymap::compile(
        &registry,
        &base,
        &[CommandKeymapOverride::Add {
            binding: binding("override:reserved", "KeyR", "global", "app:global.open"),
        }],
        &Reserved {
            chord: reserved_chord,
        },
    )
    .expect_err("reserved override");
    assert_eq!(reserved.code(), CommandKeymapErrorCode::ReservedChord);
}

#[test]
fn binding_validation_uses_registry_context_argument_and_modifier_authority() {
    let registry = registry();

    let invalid_arguments = CommandEffectiveKeymap::compile(
        &registry,
        &preset(vec![CommandBindingDefinition {
            arguments: serde_json::json!({}),
            ..binding("base:arguments", "KeyA", "global", "app:global.open")
        }]),
        &[],
        &NoReservedCommandChords,
    )
    .expect_err("invalid arguments");
    assert_eq!(
        invalid_arguments.code(),
        CommandKeymapErrorCode::InvalidArguments
    );

    let invalid_context = CommandEffectiveKeymap::compile(
        &registry,
        &preset(vec![binding(
            "base:context",
            "KeyC",
            "global",
            "app:editor.format",
        )]),
        &[],
        &NoReservedCommandChords,
    )
    .expect_err("invalid context");
    assert_eq!(
        invalid_context.code(),
        CommandKeymapErrorCode::ContextNotAllowed
    );

    let invalid_modifiers = CommandEffectiveKeymap::compile(
        &registry,
        &preset(vec![CommandBindingDefinition {
            trigger: CommandKeyTrigger {
                code: code("KeyM"),
                modifiers: CommandTriggerModifiers {
                    primary: true,
                    meta: true,
                    ..CommandTriggerModifiers::default()
                },
            },
            ..binding("base:modifiers", "KeyM", "global", "app:global.open")
        }]),
        &[],
        &NoReservedCommandChords,
    )
    .expect_err("invalid modifiers");
    assert_eq!(
        invalid_modifiers.code(),
        CommandKeymapErrorCode::InvalidModifiers
    );
}

#[test]
fn most_specific_context_wins_and_reports_shadowing() {
    let registry = registry();
    let preset = preset(vec![
        binding("base:global", "KeyK", "global", "app:global.open"),
        binding("base:surface", "KeyK", "surface", "app:surface.toggle"),
        binding("base:editor", "KeyK", "editor", "app:editor.format"),
    ]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");
    let resolution = effective
        .resolve(
            CommandPlatform::MacOs,
            &input(chord("KeyK", CommandPlatform::MacOs)),
            &context(&["global", "project", "surface", "editor"]),
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("resolution");

    let CommandKeyResolution::Resolved { winner, candidates } = resolution else {
        panic!("expected resolved")
    };
    assert_eq!(winner.binding_id, id::<CommandBindingId>("base:editor"));
    assert!(matches!(
        candidates[0].disposition,
        CommandCandidateDisposition::Winner
    ));
    assert!(matches!(
        candidates[1].disposition,
        CommandCandidateDisposition::Shadowed { .. }
    ));
    assert!(matches!(
        candidates[2].disposition,
        CommandCandidateDisposition::Shadowed { .. }
    ));
}

#[test]
fn equal_specificity_different_invocations_conflict_without_consumption() {
    let registry = registry();
    let preset = preset(vec![binding(
        "base:format",
        "KeyK",
        "editor",
        "app:editor.format",
    )]);
    let effective = CommandEffectiveKeymap::compile(
        &registry,
        &preset,
        &[CommandKeymapOverride::Add {
            binding: binding("override:toggle", "KeyK", "editor", "app:surface.toggle"),
        }],
        &NoReservedCommandChords,
    )
    .expect("previewable conflicting effective keymap");

    assert!(effective.has_conflicts());
    assert_eq!(effective.conflicts().count(), 3);
    let resolution = effective
        .resolve(
            CommandPlatform::Linux,
            &input(chord("KeyK", CommandPlatform::Linux)),
            &context(&["global", "project", "surface", "editor"]),
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("resolution");

    assert!(matches!(resolution, CommandKeyResolution::Conflict { .. }));
    assert!(!resolution.is_consumed());
    assert!(
        resolution
            .candidates()
            .iter()
            .all(|candidate| candidate.disposition == CommandCandidateDisposition::Conflict)
    );
}

#[test]
fn identical_invocations_do_not_create_a_hidden_semantic_conflict() {
    let registry = registry();
    let preset = preset(vec![
        binding("base:first", "KeyK", "editor", "app:editor.format"),
        binding("base:second", "KeyK", "editor", "app:editor.format"),
    ]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");
    assert!(!effective.has_conflicts());

    let resolution = effective
        .resolve(
            CommandPlatform::Windows,
            &input(chord("KeyK", CommandPlatform::Windows)),
            &context(&["global", "project", "surface", "editor"]),
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("resolution");
    let CommandKeyResolution::Resolved { winner, candidates } = resolution else {
        panic!("expected resolved")
    };
    assert_eq!(winner.binding_id, id::<CommandBindingId>("base:first"));
    assert_eq!(
        candidates[1].disposition,
        CommandCandidateDisposition::Equivalent
    );
}

#[test]
fn gates_and_consumption_follow_the_keyboard_contract() {
    let registry = registry();
    let preset = preset(vec![binding(
        "base:format",
        "KeyK",
        "editor",
        "app:editor.format",
    )]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");
    let platform = CommandPlatform::MacOs;
    let key = chord("KeyK", platform);
    let hot_context = context(&["global", "project", "surface", "editor"]);

    let repeat = effective
        .resolve(
            platform,
            &CommandKeyboardInput {
                repeat: true,
                ..input(key.clone())
            },
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("repeat");
    assert!(matches!(
        repeat,
        CommandKeyResolution::Gated {
            gate: CommandKeyboardGate::Repeat,
            ..
        }
    ));
    assert!(!repeat.is_consumed());

    let composition = effective
        .resolve(
            platform,
            &CommandKeyboardInput {
                composing: true,
                ..input(key.clone())
            },
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("composition");
    assert!(matches!(
        composition,
        CommandKeyResolution::Gated {
            gate: CommandKeyboardGate::Composition,
            ..
        }
    ));

    let text = effective
        .resolve(
            platform,
            &CommandKeyboardInput {
                editable_text: true,
                ..input(key.clone())
            },
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("text");
    assert!(matches!(
        text,
        CommandKeyResolution::Gated {
            gate: CommandKeyboardGate::TextInput,
            ..
        }
    ));
    assert!(!text.is_consumed());

    let reserved = effective
        .resolve(
            platform,
            &input(key.clone()),
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &Reserved { chord: key.clone() },
        )
        .expect("reserved");
    assert!(matches!(
        reserved,
        CommandKeyResolution::Gated {
            gate: CommandKeyboardGate::Reserved,
            ..
        }
    ));
    assert!(!reserved.is_consumed());

    let captured = effective
        .resolve(
            platform,
            &input(key.clone()),
            &hot_context,
            CommandKeyboardMode::Capture,
            &NoReservedCommandChords,
        )
        .expect("captured");
    assert!(matches!(captured, CommandKeyResolution::Captured { .. }));
    assert!(captured.is_consumed());

    let unbound = effective
        .resolve(
            platform,
            &input(chord("KeyZ", platform)),
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("unbound");
    assert_eq!(unbound, CommandKeyResolution::Unbound);
    assert!(!unbound.is_consumed());

    let resolved = effective
        .resolve(
            platform,
            &input(key),
            &hot_context,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        )
        .expect("resolved");
    assert!(matches!(resolved, CommandKeyResolution::Resolved { .. }));
    assert!(resolved.is_consumed());
}

#[test]
fn reverse_lookup_and_labels_use_effective_runtime_records() {
    let registry = registry();
    let preset = preset(vec![
        binding("base:open", "KeyO", "global", "app:global.open"),
        binding("base:format", "KeyF", "editor", "app:editor.format"),
    ]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");

    let records = effective
        .shortcuts_for_command(&id::<CommandId>("app:global.open"), CommandPlatform::MacOs);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].label, "⌘O");
    assert_eq!(records[0].binding_id, id::<CommandBindingId>("base:open"));
}

#[test]
fn resolver_rejects_context_paths_that_do_not_follow_the_sealed_tree() {
    let registry = registry();
    let preset = preset(vec![binding(
        "base:open",
        "KeyO",
        "global",
        "app:global.open",
    )]);
    let effective =
        CommandEffectiveKeymap::compile(&registry, &preset, &[], &NoReservedCommandChords)
            .expect("effective");

    let invalid = context(&["global", "editor"]);
    assert_eq!(
        effective.resolve(
            CommandPlatform::MacOs,
            &input(chord("KeyO", CommandPlatform::MacOs)),
            &invalid,
            CommandKeyboardMode::Dispatch,
            &NoReservedCommandChords,
        ),
        Err(CommandKeyResolutionError::InvalidContextPath)
    );
}
