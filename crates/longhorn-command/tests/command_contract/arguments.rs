use longhorn_command::{
    CommandArgumentErrorCode, CommandArgumentField, CommandArgumentKind, CommandArgumentSchema,
    CommandArgumentValue, CommandFiniteNumber, CommandLimits, CommandRegistryBuilder,
    CommandRegistryErrorCode, CommandRegistryGeneration,
};
use longhorn_core::CommandEnumValueId;
use serde_json::json;

use super::support::{command, context, field_id};

fn schema() -> CommandArgumentSchema {
    CommandArgumentSchema::Object {
        fields: vec![
            CommandArgumentField {
                id: field_id("enabled"),
                required: true,
                default: None,
                kind: CommandArgumentKind::Boolean,
            },
            CommandArgumentField {
                id: field_id("gain"),
                required: false,
                default: Some(CommandArgumentValue::Number(
                    CommandFiniteNumber::new(0.5).expect("finite"),
                )),
                kind: CommandArgumentKind::Number {
                    minimum: Some(CommandFiniteNumber::new(0.0).expect("finite")),
                    maximum: Some(CommandFiniteNumber::new(1.0).expect("finite")),
                },
            },
            CommandArgumentField {
                id: field_id("count"),
                required: false,
                default: Some(CommandArgumentValue::Integer(2)),
                kind: CommandArgumentKind::Integer {
                    minimum: Some(1),
                    maximum: Some(8),
                },
            },
            CommandArgumentField {
                id: field_id("name"),
                required: false,
                default: None,
                kind: CommandArgumentKind::String { maximum_bytes: 8 },
            },
            CommandArgumentField {
                id: field_id("mode"),
                required: false,
                default: Some(CommandArgumentValue::Enum(
                    CommandEnumValueId::new("replace").expect("enum value"),
                )),
                kind: CommandArgumentKind::Enum {
                    values: vec![
                        CommandEnumValueId::new("replace").expect("enum value"),
                        CommandEnumValueId::new("insert").expect("enum value"),
                    ],
                },
            },
        ],
    }
}

#[test]
fn no_argument_schema_accepts_only_null() {
    assert!(CommandArgumentSchema::None.validate(&json!(null)).is_ok());
    assert_eq!(
        CommandArgumentSchema::None
            .validate(&json!({}))
            .expect_err("object must fail")
            .code(),
        CommandArgumentErrorCode::ArgumentsNotAllowed
    );
}

#[test]
fn bounded_object_normalizes_defaults_in_field_id_order() {
    let normalized = schema()
        .validate(&json!({"enabled": true, "name": "mix"}))
        .expect("valid input");
    let fields: Vec<_> = normalized
        .fields()
        .map(|(id, value)| (id.as_str(), value))
        .collect();
    assert_eq!(
        fields.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        ["count", "enabled", "gain", "mode", "name"]
    );
    assert_eq!(
        normalized.get(&field_id("count")),
        Some(&CommandArgumentValue::Integer(2))
    );
}

#[test]
fn object_schema_rejects_unknown_missing_nested_array_and_wrong_types() {
    let cases = [
        (
            json!({"enabled": true, "extra": 1}),
            CommandArgumentErrorCode::UnknownField,
        ),
        (
            json!({"name": "mix"}),
            CommandArgumentErrorCode::MissingRequiredField,
        ),
        (
            json!({"enabled": {"nested": true}}),
            CommandArgumentErrorCode::TypeMismatch,
        ),
        (
            json!({"enabled": [true]}),
            CommandArgumentErrorCode::TypeMismatch,
        ),
        (
            json!({"enabled": "yes"}),
            CommandArgumentErrorCode::TypeMismatch,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            schema()
                .validate(&input)
                .expect_err("input must fail")
                .code(),
            expected
        );
    }
}

#[test]
fn object_schema_enforces_ranges_string_bytes_and_closed_enums() {
    let cases = [
        (
            json!({"enabled": true, "gain": 1.1}),
            CommandArgumentErrorCode::OutOfRange,
        ),
        (
            json!({"enabled": true, "count": 0}),
            CommandArgumentErrorCode::OutOfRange,
        ),
        (
            json!({"enabled": true, "name": "long-name"}),
            CommandArgumentErrorCode::StringTooLong,
        ),
        (
            json!({"enabled": true, "mode": "merge"}),
            CommandArgumentErrorCode::UnknownEnumValue,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            schema()
                .validate(&input)
                .expect_err("input must fail")
                .code(),
            expected
        );
    }
}

#[test]
fn non_finite_numbers_and_invalid_defaults_fail() {
    assert!(CommandFiniteNumber::new(f64::NAN).is_err());
    assert!(CommandFiniteNumber::new(f64::INFINITY).is_err());

    let mut invalid = command("test:invalid-default", "Invalid default", "global");
    invalid.arguments = CommandArgumentSchema::Object {
        fields: vec![CommandArgumentField {
            id: field_id("count"),
            required: false,
            default: Some(CommandArgumentValue::Integer(9)),
            kind: CommandArgumentKind::Integer {
                minimum: Some(1),
                maximum: Some(8),
            },
        }],
    };
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global");
    builder.register_command(invalid).expect("registration");
    assert_eq!(
        builder.seal().expect_err("invalid default").code(),
        CommandRegistryErrorCode::InvalidArgumentDefault
    );
}

#[test]
fn invalid_field_bounds_empty_objects_and_duplicate_enum_values_fail() {
    let invalid_kinds = [
        CommandArgumentKind::Integer {
            minimum: Some(2),
            maximum: Some(1),
        },
        CommandArgumentKind::String { maximum_bytes: 0 },
        CommandArgumentKind::Enum { values: Vec::new() },
        CommandArgumentKind::Enum {
            values: vec![
                CommandEnumValueId::new("same").expect("enum"),
                CommandEnumValueId::new("same").expect("enum"),
            ],
        },
    ];
    for kind in invalid_kinds {
        let mut invalid = command("test:invalid-schema", "Invalid schema", "global");
        invalid.arguments = CommandArgumentSchema::Object {
            fields: vec![CommandArgumentField {
                id: field_id("value"),
                required: false,
                default: None,
                kind,
            }],
        };
        let mut builder = CommandRegistryBuilder::new(
            CommandRegistryGeneration::INITIAL,
            CommandLimits::default(),
        );
        builder
            .register_context(context("global", None))
            .expect("global");
        builder.register_command(invalid).expect("registration");
        assert_eq!(
            builder.seal().expect_err("invalid schema").code(),
            CommandRegistryErrorCode::InvalidArgumentSchema
        );
    }

    let mut empty = command("test:empty-schema", "Empty schema", "global");
    empty.arguments = CommandArgumentSchema::Object { fields: Vec::new() };
    let mut builder =
        CommandRegistryBuilder::new(CommandRegistryGeneration::INITIAL, CommandLimits::default());
    builder
        .register_context(context("global", None))
        .expect("global");
    builder.register_command(empty).expect("registration");
    assert_eq!(
        builder.seal().expect_err("empty schema").code(),
        CommandRegistryErrorCode::InvalidArgumentSchema
    );
}
