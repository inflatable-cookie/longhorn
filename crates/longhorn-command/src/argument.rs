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

/// Finite canonical floating-point value accepted by command schemas.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct CommandFiniteNumber(f64);

impl CommandFiniteNumber {
    /// Constructs a finite value, normalizing negative zero.
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("command number must be finite");
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    /// Returns the finite value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for CommandFiniteNumber {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for CommandFiniteNumber {}

impl PartialOrd for CommandFiniteNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CommandFiniteNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Serialize for CommandFiniteNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for CommandFiniteNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f64::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Closed primitive kind accepted by one command argument field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CommandArgumentKind {
    /// Boolean value.
    Boolean,
    /// Finite floating-point value with optional inclusive bounds.
    Number {
        /// Optional inclusive minimum.
        minimum: Option<CommandFiniteNumber>,
        /// Optional inclusive maximum.
        maximum: Option<CommandFiniteNumber>,
    },
    /// Signed integer with optional inclusive bounds.
    Integer {
        /// Optional inclusive minimum.
        #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
        minimum: Option<i64>,
        /// Optional inclusive maximum.
        #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
        maximum: Option<i64>,
    },
    /// UTF-8 string bounded by encoded byte length.
    String {
        /// Maximum UTF-8 bytes.
        maximum_bytes: usize,
    },
    /// Closed string enum.
    Enum {
        /// Admitted values.
        values: Vec<CommandEnumValueId>,
    },
}

/// One named field in a bounded object argument schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandArgumentField {
    /// Stable field identity and serialized object key.
    pub id: CommandFieldId,
    /// Whether omission is invalid when no default exists.
    pub required: bool,
    /// Optional normalized default.
    pub default: Option<CommandArgumentValue>,
    /// Closed primitive field kind.
    pub kind: CommandArgumentKind,
}

/// Closed v1 command argument schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "shape"
)]
pub enum CommandArgumentSchema {
    /// The command accepts only JSON `null`.
    None,
    /// The command accepts one object containing declared primitive fields.
    Object {
        /// Named object fields.
        fields: Vec<CommandArgumentField>,
    },
}

/// One normalized primitive command argument.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    tag = "kind",
    content = "value"
)]
pub enum CommandArgumentValue {
    /// Boolean value.
    Boolean(bool),
    /// Finite floating-point value.
    Number(CommandFiniteNumber),
    /// Signed integer.
    Integer(#[cfg_attr(feature = "bindings", ts(type = "number"))] i64),
    /// Bounded UTF-8 string.
    String(String),
    /// Closed enum value.
    Enum(CommandEnumValueId),
}

/// Structurally validated and deterministically ordered command arguments.
#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "bindings",
    ts(type = "Record<CommandFieldId, CommandArgumentValue>")
)]
#[serde(transparent)]
pub struct CommandArguments(BTreeMap<CommandFieldId, CommandArgumentValue>);

impl CommandArguments {
    /// Returns normalized fields in stable id order.
    pub fn fields(
        &self,
    ) -> impl ExactSizeIterator<Item = (&CommandFieldId, &CommandArgumentValue)> {
        self.0.iter()
    }

    /// Returns one normalized field.
    #[must_use]
    pub fn get(&self, id: &CommandFieldId) -> Option<&CommandArgumentValue> {
        self.0.get(id)
    }
}

impl CommandArgumentSchema {
    /// Validates raw JSON structurally and returns one normalized bounded value.
    pub fn validate(&self, input: &Value) -> Result<CommandArguments, CommandArgumentError> {
        match self {
            Self::None => {
                if input.is_null() {
                    Ok(CommandArguments::default())
                } else {
                    Err(argument_error(
                        CommandArgumentErrorCode::ArgumentsNotAllowed,
                        None,
                        "command does not accept arguments; expected null",
                    ))
                }
            }
            Self::Object { fields } => validate_object(fields, input),
        }
    }

    pub(crate) fn validate_definition(
        &self,
        limits: CommandLimits,
    ) -> Result<(), CommandRegistryError> {
        let Self::Object { fields } = self else {
            return Ok(());
        };
        if fields.is_empty() {
            return Err(registry_error(
                CommandRegistryErrorCode::InvalidArgumentSchema,
                "object argument schema must declare at least one field",
            ));
        }
        if fields.len() > limits.maximum_fields_per_command {
            return Err(registry_error(
                CommandRegistryErrorCode::LimitExceeded,
                format!(
                    "argument schema declares {} fields; maximum is {}",
                    fields.len(),
                    limits.maximum_fields_per_command
                ),
            ));
        }
        let mut ids = BTreeSet::new();
        for field in fields {
            if !ids.insert(field.id.clone()) {
                return Err(registry_error(
                    CommandRegistryErrorCode::DuplicateItem,
                    format!("duplicate argument field {}", field.id),
                ));
            }
            validate_kind(field, limits)?;
            if let Some(default) = &field.default {
                validate_typed_value(&field.id, &field.kind, default).map_err(|error| {
                    registry_error(
                        CommandRegistryErrorCode::InvalidArgumentDefault,
                        format!("invalid default for field {}: {}", field.id, error.detail()),
                    )
                })?;
            }
        }
        Ok(())
    }

    pub(crate) fn canonicalize(&mut self) {
        if let Self::Object { fields } = self {
            fields.sort_by(|left, right| left.id.cmp(&right.id));
        }
    }
}

fn validate_kind(
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

fn invalid_schema(field_id: &CommandFieldId, detail: impl fmt::Display) -> CommandRegistryError {
    registry_error(
        CommandRegistryErrorCode::InvalidArgumentSchema,
        format!("invalid schema for field {field_id}: {detail}"),
    )
}

fn validate_object(
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

fn validate_raw_value(
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

fn validate_typed_value(
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

fn validate_range<T>(
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

fn type_mismatch(field_id: &CommandFieldId, expected: &str) -> CommandArgumentError {
    argument_error(
        CommandArgumentErrorCode::TypeMismatch,
        Some(field_id.clone()),
        format!("field {field_id} must be a {expected}"),
    )
}
