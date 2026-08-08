use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use longhorn_core::{CommandEnumValueId, CommandFieldId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;

use crate::{
    CommandArgumentError, CommandArgumentErrorCode, CommandLimits, CommandRegistryError,
    CommandRegistryErrorCode,
    error::{argument_error, registry_error},
};


use super::{CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandArgumentValue, CommandArguments, CommandFiniteNumber};


pub(crate) fn validate_kind(
    field: &CommandArgumentField,
    limits: CommandLimits,
) -> Result<(), CommandRegistryError> {
    match &field.kind {
        CommandArgumentKind::Number { minimum, maximum } => {
            if minimum.zip(*maximum).is_some_and(|(min, max)| min > max) {
                return Err(invalid_schema(&field.id, "minimum exceeds maximum"));
            }
        }
        CommandArgumentKind::Integer { minimum, maximum } => {
            if minimum.zip(*maximum).is_some_and(|(min, max)| min > max) {
                return Err(invalid_schema(&field.id, "minimum exceeds maximum"));
            }
        }
        CommandArgumentKind::String { maximum_bytes } => {
            if *maximum_bytes == 0 || *maximum_bytes > limits.maximum_argument_string_bytes {
                return Err(invalid_schema(
                    &field.id,
                    format!(
                        "string maximum is {maximum_bytes}; registry maximum is {}",
                        limits.maximum_argument_string_bytes
                    ),
                ));
            }
        }
        CommandArgumentKind::Enum { values } => {
            if values.is_empty() {
                return Err(invalid_schema(&field.id, "enum has no values"));
            }
            if values.len() > limits.maximum_enum_values_per_field {
                return Err(invalid_schema(
                    &field.id,
                    format!(
                        "enum has {} values; maximum is {}",
                        values.len(),
                        limits.maximum_enum_values_per_field
                    ),
                ));
            }
            let unique: BTreeSet<_> = values.iter().collect();
            if unique.len() != values.len() {
                return Err(invalid_schema(&field.id, "enum contains duplicate values"));
            }
        }
        CommandArgumentKind::Boolean => {}
    }
    Ok(())
}

pub(crate) fn invalid_schema(field_id: &CommandFieldId, detail: impl fmt::Display) -> CommandRegistryError {
    registry_error(
        CommandRegistryErrorCode::InvalidArgumentSchema,
        format!("invalid schema for field {field_id}: {detail}"),
    )
}

pub(crate) fn validate_object(
    fields: &[CommandArgumentField],
    input: &Value,
) -> Result<CommandArguments, CommandArgumentError> {
    let object = input.as_object().ok_or_else(|| {
        argument_error(
            CommandArgumentErrorCode::ObjectRequired,
            None,
            "command arguments must be an object",
        )
    })?;
    let by_name: BTreeMap<_, _> = fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    for name in object.keys() {
        if !by_name.contains_key(name.as_str()) {
            return Err(argument_error(
                CommandArgumentErrorCode::UnknownField,
                CommandFieldId::new(name).ok(),
                format!("unknown command argument field {name}"),
            ));
        }
    }

    let mut normalized = BTreeMap::new();
    for field in fields {
        if let Some(raw) = object.get(field.id.as_str()) {
            normalized.insert(field.id.clone(), validate_raw_value(field, raw)?);
        } else if let Some(default) = &field.default {
            normalized.insert(field.id.clone(), default.clone());
        } else if field.required {
            return Err(argument_error(
                CommandArgumentErrorCode::MissingRequiredField,
                Some(field.id.clone()),
                format!("required command argument field {} is absent", field.id),
            ));
        }
    }
    Ok(CommandArguments(normalized))
}

pub(crate) fn validate_raw_value(
    field: &CommandArgumentField,
    raw: &Value,
) -> Result<CommandArgumentValue, CommandArgumentError> {
    let value = match &field.kind {
        CommandArgumentKind::Boolean => raw
            .as_bool()
            .map(CommandArgumentValue::Boolean)
            .ok_or_else(|| type_mismatch(&field.id, "boolean"))?,
        CommandArgumentKind::Number { .. } => raw
            .as_f64()
            .and_then(|value| CommandFiniteNumber::new(value).ok())
            .map(CommandArgumentValue::Number)
            .ok_or_else(|| type_mismatch(&field.id, "finite number"))?,
        CommandArgumentKind::Integer { .. } => raw
            .as_i64()
            .map(CommandArgumentValue::Integer)
            .ok_or_else(|| type_mismatch(&field.id, "signed integer"))?,
        CommandArgumentKind::String { .. } => raw
            .as_str()
            .map(|value| CommandArgumentValue::String(value.to_owned()))
            .ok_or_else(|| type_mismatch(&field.id, "string"))?,
        CommandArgumentKind::Enum { .. } => {
            let raw = raw
                .as_str()
                .ok_or_else(|| type_mismatch(&field.id, "enum string"))?;
            let value = CommandEnumValueId::new(raw).map_err(|_| {
                argument_error(
                    CommandArgumentErrorCode::UnknownEnumValue,
                    Some(field.id.clone()),
                    format!("field {} has unknown enum value {raw}", field.id),
                )
            })?;
            CommandArgumentValue::Enum(value)
        }
    };
    validate_typed_value(&field.id, &field.kind, &value)?;
    Ok(value)
}

pub(crate) fn validate_typed_value(
    field_id: &CommandFieldId,
    kind: &CommandArgumentKind,
    value: &CommandArgumentValue,
) -> Result<(), CommandArgumentError> {
    match (kind, value) {
        (CommandArgumentKind::Boolean, CommandArgumentValue::Boolean(_)) => Ok(()),
        (CommandArgumentKind::Number { minimum, maximum }, CommandArgumentValue::Number(value)) => {
            validate_range(field_id, *value, *minimum, *maximum)
        }
        (
            CommandArgumentKind::Integer { minimum, maximum },
            CommandArgumentValue::Integer(value),
        ) => validate_range(field_id, *value, *minimum, *maximum),
        (CommandArgumentKind::String { maximum_bytes }, CommandArgumentValue::String(value))
            if value.len() <= *maximum_bytes =>
        {
            Ok(())
        }
        (CommandArgumentKind::String { maximum_bytes }, CommandArgumentValue::String(value)) => {
            Err(argument_error(
                CommandArgumentErrorCode::StringTooLong,
                Some(field_id.clone()),
                format!(
                    "field {field_id} contains {} bytes; maximum is {maximum_bytes}",
                    value.len()
                ),
            ))
        }
        (CommandArgumentKind::Enum { values }, CommandArgumentValue::Enum(value))
            if values.contains(value) =>
        {
            Ok(())
        }
        (CommandArgumentKind::Enum { .. }, CommandArgumentValue::Enum(value)) => {
            Err(argument_error(
                CommandArgumentErrorCode::UnknownEnumValue,
                Some(field_id.clone()),
                format!("field {field_id} has unknown enum value {value}"),
            ))
        }
        _ => Err(type_mismatch(field_id, "declared primitive type")),
    }
}

pub(crate) fn validate_range<T>(
    field_id: &CommandFieldId,
    value: T,
    minimum: Option<T>,
    maximum: Option<T>,
) -> Result<(), CommandArgumentError>
where
    T: Copy + PartialOrd,
{
    if minimum.is_some_and(|minimum| value < minimum)
        || maximum.is_some_and(|maximum| value > maximum)
    {
        return Err(argument_error(
            CommandArgumentErrorCode::OutOfRange,
            Some(field_id.clone()),
            format!("field {field_id} is outside its inclusive bounds"),
        ));
    }
    Ok(())
}

pub(crate) fn type_mismatch(field_id: &CommandFieldId, expected: &str) -> CommandArgumentError {
    argument_error(
        CommandArgumentErrorCode::TypeMismatch,
        Some(field_id.clone()),
        format!("field {field_id} must be a {expected}"),
    )
}
