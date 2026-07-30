use longhorn_command::{
    CommandKeyChord, CommandKeyTrigger, CommandModifierError, CommandModifiers,
    CommandNativeModifier, CommandPhysicalCode, CommandPhysicalCodeError, CommandPlatform,
    CommandTriggerModifiers,
};

fn code(value: &str) -> CommandPhysicalCode {
    CommandPhysicalCode::new(value).expect("physical code")
}

#[test]
fn physical_code_is_bounded_dom_identity_not_produced_text() {
    assert_eq!(code("KeyQ").as_str(), "KeyQ");
    assert_eq!(code("IntlBackslash").as_str(), "IntlBackslash");
    assert_eq!(
        CommandPhysicalCode::new("Unidentified"),
        Err(CommandPhysicalCodeError::Unidentified)
    );
    assert_eq!(
        CommandPhysicalCode::new("q"),
        Err(CommandPhysicalCodeError::InvalidStart)
    );
    assert!(matches!(
        CommandPhysicalCode::new("é"),
        Err(CommandPhysicalCodeError::InvalidCharacter { .. })
    ));
}

#[test]
fn semantic_primary_normalizes_across_every_platform() {
    let trigger = CommandKeyTrigger {
        code: code("KeyK"),
        modifiers: CommandTriggerModifiers {
            primary: true,
            shift: true,
            ..CommandTriggerModifiers::default()
        },
    };

    assert_eq!(
        trigger.resolve(CommandPlatform::MacOs).expect("mac chord"),
        CommandKeyChord {
            code: code("KeyK"),
            modifiers: CommandModifiers {
                shift: true,
                meta: true,
                ..CommandModifiers::default()
            },
        }
    );
    for platform in [CommandPlatform::Windows, CommandPlatform::Linux] {
        assert_eq!(
            trigger.resolve(platform).expect("control chord"),
            CommandKeyChord {
                code: code("KeyK"),
                modifiers: CommandModifiers {
                    control: true,
                    shift: true,
                    ..CommandModifiers::default()
                },
            }
        );
    }
}

#[test]
fn semantic_primary_rejects_ambiguous_duplicate_native_modifiers() {
    let duplicate_meta = CommandTriggerModifiers {
        primary: true,
        meta: true,
        ..CommandTriggerModifiers::default()
    };
    assert_eq!(
        duplicate_meta.resolve(CommandPlatform::MacOs),
        Err(CommandModifierError::DuplicatePrimary {
            platform: CommandPlatform::MacOs,
            modifier: CommandNativeModifier::Meta,
        })
    );

    let duplicate_control = CommandTriggerModifiers {
        primary: true,
        control: true,
        ..CommandTriggerModifiers::default()
    };
    assert_eq!(
        duplicate_control.resolve(CommandPlatform::Windows),
        Err(CommandModifierError::DuplicatePrimary {
            platform: CommandPlatform::Windows,
            modifier: CommandNativeModifier::Control,
        })
    );
    assert!(duplicate_control.resolve(CommandPlatform::MacOs).is_ok());
}

#[test]
fn shortcut_labels_use_canonical_modifier_order() {
    let chord = CommandKeyChord {
        code: code("KeyK"),
        modifiers: CommandModifiers {
            control: true,
            alt: true,
            shift: true,
            meta: true,
        },
    };

    assert_eq!(chord.label(CommandPlatform::MacOs), "⌃⌥⇧⌘K");
    assert_eq!(
        chord.label(CommandPlatform::Windows),
        "Ctrl+Alt+Shift+Meta+K"
    );
}
